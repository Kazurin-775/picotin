use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::Context;
use unshare::Command;

mod config;
mod modules;
use self::modules::*;

pub use config::ContainerConfig;

pub struct Container {
    cgroup: ContainerCgroup,
    ns: ContainerNamespaces,
    _dir_bomb: DirBomb,
}

static CONTAINER_RUNTIME_DATA: &str = "/var/run/picotin";

fn ensure_runtime_data_dir() -> std::io::Result<()> {
    match std::fs::create_dir(CONTAINER_RUNTIME_DATA) {
        Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(()),
        other => other,
    }
}

impl Container {
    pub fn new(config: ContainerConfig) -> anyhow::Result<Container> {
        ensure_runtime_data_dir().context("create runtime data directory")?;

        // Generate ID and info path
        let (id, path) = loop {
            let id = format!("{:08x}", rand::random::<u32>());
            let path = Path::new(CONTAINER_RUNTIME_DATA).join(&id);
            match std::fs::create_dir(&path) {
                Ok(()) => break (id, path),
                Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err).context("create container info directory"),
            }
        };
        log::debug!("Creating container {}", id);
        let dir_bomb = DirBomb { path };

        // Create namespaces
        let ns = ContainerNamespaces::new(&config).context("create namespaces")?;

        // Create cgroup
        let cgroup = ContainerCgroup::new(&id, &config).context("create cgroup")?;

        Ok(Container {
            cgroup,
            ns,
            _dir_bomb: dir_bomb,
        })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let cgroup = self.cgroup.try_clone().context("clone cgroup instance")?;

        let mut command = Command::new("/bin/bash");
        command.before_unfreeze(move |pid| {
            log::debug!("Child process spawned as PID {}", pid);
            match cgroup.jail_pid(pid) {
                Ok(()) => Ok(()),
                Err(err) => {
                    log::error!("Failed to add PID {} to cgroup: {:#}", pid, err);
                    Err(Box::new(std::io::Error::from(ErrorKind::Other)))
                }
            }
        });
        self.ns
            .apply(&mut command)
            .context("set child's namespaces")?;

        let mut child = command
            .spawn()
            .map_err(|err| anyhow::anyhow!("spawn child process: {}", err))?;
        let status = child.wait().context("wait for child process")?;
        log::debug!("Child process exited with status {}", status);
        Ok(())
    }
}

impl Drop for Container {
    fn drop(&mut self) {
        if let Err(err) = self.cgroup.delete() {
            log::error!("Failed to delete cgroup: {:#}", err);
        }
    }
}

struct DirBomb {
    path: PathBuf,
}

impl Drop for DirBomb {
    fn drop(&mut self) {
        log::debug!("Cleaning up {:?}", self.path);
        if let Err(err) = std::fs::remove_dir_all(&self.path) {
            log::error!("Failed to delete {:?}: {}", self.path, err);
        }
    }
}
