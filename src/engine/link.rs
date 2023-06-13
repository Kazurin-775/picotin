use std::path::PathBuf;

use anyhow::Context;

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
    let lhs_pid: u64 = std::fs::read_to_string(lhs_pid)
        .context("read lhs pid")?
        .parse()
        .context("parse lhs pid")?;
    let lhs_net_ns =
        std::fs::read_link(format!("/proc/{lhs_pid}/ns/net")).context("read lhs net namespace")?;
    log::debug!("Container {lhs:?}'s PID = {lhs_pid}, network namespace = {lhs_net_ns:?}");

    let mut rhs_pid = PathBuf::from(super::CONTAINER_RUNTIME_DATA);
    rhs_pid.push(rhs);
    rhs_pid.push("init_pid");
    let rhs_pid: u64 = std::fs::read_to_string(rhs_pid)
        .context("read rhs pid")?
        .parse()
        .context("parse rhs pid")?;
    let rhs_net_ns =
        std::fs::read_link(format!("/proc/{rhs_pid}/ns/net")).context("read rhs net namespace")?;
    log::debug!("Container {rhs:?}'s PID = {rhs_pid}, network namespace = {rhs_net_ns:?}");

    Ok(())
}
