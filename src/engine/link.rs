use std::{
    fs::File,
    net::{IpAddr, Ipv4Addr},
    os::fd::AsRawFd,
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
use async_executor::Executor;
use futures_util::TryStreamExt;
use nix::sched::CloneFlags;

pub fn add_veth_link(lhs: &str, rhs: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !lhs.as_bytes().contains(&b'/'),
        "invalid lhs container name",
    );
    anyhow::ensure!(
        !rhs.as_bytes().contains(&b'/'),
        "invalid rhs container name",
    );

    let mut lhs_pid = PathBuf::from(super::CONTAINER_RUNTIME_DATA);
    lhs_pid.push(lhs);
    lhs_pid.push("init_pid");
    let lhs_pid: u32 = std::fs::read_to_string(lhs_pid)
        .context("read lhs pid")?
        .parse()
        .context("parse lhs pid")?;
    let lhs_net_ns =
        std::fs::read_link(format!("/proc/{lhs_pid}/ns/net")).context("read lhs net namespace")?;
    log::debug!("Container {lhs:?}'s PID = {lhs_pid}, network namespace = {lhs_net_ns:?}");

    let mut rhs_pid = PathBuf::from(super::CONTAINER_RUNTIME_DATA);
    rhs_pid.push(rhs);
    rhs_pid.push("init_pid");
    let rhs_pid: u32 = std::fs::read_to_string(rhs_pid)
        .context("read rhs pid")?
        .parse()
        .context("parse rhs pid")?;
    let rhs_net_ns =
        std::fs::read_link(format!("/proc/{rhs_pid}/ns/net")).context("read rhs net namespace")?;
    log::debug!("Container {rhs:?}'s PID = {rhs_pid}, network namespace = {rhs_net_ns:?}");

    std::fs::write(
        format!("{}/{}/paired_with", super::CONTAINER_RUNTIME_DATA, lhs),
        rhs,
    )
    .context("write lhs paired_with")?;
    std::fs::write(
        format!("{}/{}/paired_with", super::CONTAINER_RUNTIME_DATA, rhs),
        lhs,
    )
    .context("write rhs paired_with")?;

    let executor = Arc::new(Executor::new());
    futures_lite::future::block_on(executor.run(executor.spawn(create_veth(
        Arc::clone(&executor),
        lhs.to_owned(),
        rhs.to_owned(),
        lhs_pid,
        rhs_pid,
    ))))
    .context("create veth")?;

    Ok(())
}

async fn create_veth(
    executor: Arc<Executor<'static>>,
    lhs: String,
    rhs: String,
    lhs_pid: u32,
    rhs_pid: u32,
) -> anyhow::Result<()> {
    let (conn, handle, _) =
        rtnetlink::new_connection_with_socket::<netlink_proto::sys::SmolSocket>()?;
    executor.spawn(conn).detach();

    log::debug!("Creating veth-{lhs} and veth-{rhs}");
    handle
        .link()
        .add()
        .veth(format!("veth-{lhs}"), format!("veth-{rhs}"))
        .execute()
        .await
        .context("create veth")?;

    let lhs_if_idx = handle
        .link()
        .get()
        .match_name(format!("veth-{lhs}"))
        .execute()
        .try_next()
        .await?
        .ok_or(anyhow::anyhow!("couldn't find interface veth-{lhs}"))?
        .header
        .index;
    let rhs_if_idx = handle
        .link()
        .get()
        .match_name(format!("veth-{rhs}"))
        .execute()
        .try_next()
        .await?
        .ok_or(anyhow::anyhow!("couldn't find interface veth-{rhs}"))?
        .header
        .index;

    log::debug!("Associating veth-{lhs} (index {lhs_if_idx}) with netns of PID {lhs_pid}");
    handle
        .link()
        .set(lhs_if_idx)
        .up()
        .setns_by_pid(lhs_pid)
        .execute()
        .await
        .context("set lhs's netns")?;

    log::debug!("Associating veth-{rhs} (index {rhs_if_idx}) with netns of PID {rhs_pid}");
    handle
        .link()
        .set(rhs_if_idx)
        .up()
        .setns_by_pid(rhs_pid)
        .execute()
        .await
        .context("set rhs's netns")?;

    log::debug!("Assigning IP address 192.168.1.1/24 to veth-{lhs}");
    assign_ip_addr_in_ns(
        lhs_pid,
        &executor,
        lhs_if_idx,
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        24,
    )
    .await
    .with_context(|| format!("assign IP address 192.168.1.1/24 to veth-{lhs}"))?;

    log::debug!("Assigning IP address 192.168.1.2/24 to veth-{rhs}");
    assign_ip_addr_in_ns(
        rhs_pid,
        &executor,
        rhs_if_idx,
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
        24,
    )
    .await
    .with_context(|| format!("assign IP address 192.168.1.2/24 to veth-{rhs}"))?;

    Ok(())
}

async fn assign_ip_addr_in_ns(
    pid: u32,
    executor: &Executor<'static>,
    if_index: u32,
    addr: IpAddr,
    prefix_len: u8,
) -> anyhow::Result<()> {
    let old_ns = File::open("/proc/self/ns/net").context("open /proc/self/ns/net")?;
    let new_ns = File::open(format!("/proc/{pid}/ns/net"))
        .with_context(|| format!("open /proc/{pid}/ns/net"))?;

    nix::sched::setns(new_ns.as_raw_fd(), CloneFlags::CLONE_NEWNET)
        .context("switch to new network namespace")?;

    // The netlink socket must be reopened inside the new namespace
    let (conn, handle, _) =
        rtnetlink::new_connection_with_socket::<netlink_proto::sys::SmolSocket>()
            .context("reopen netlink socket")?;
    executor.spawn(conn).detach();
    handle
        .address()
        .add(if_index, addr, prefix_len)
        .execute()
        .await
        .context("rtnetlink call")?;

    nix::sched::setns(old_ns.as_raw_fd(), CloneFlags::CLONE_NEWNET)
        .context("switch to old network namespace")?;

    Ok(())
}
