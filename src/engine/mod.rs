use std::{
    io::ErrorKind,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Context;

mod config;
mod modules;
use self::modules::*;

pub use config::ContainerConfig;

pub struct Container {
    cgroup: ContainerCgroup,
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

        // Create cgroup
        let cgroup = ContainerCgroup::new(&id, &config).context("create cgroup")?;

        Ok(Container {
            cgroup,
            _dir_bomb: dir_bomb,
        })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let cgroup = self.cgroup.try_clone().context("clone cgroup instance")?;

        let mut child = unsafe {
            Command::new("/bin/bash")
                .pre_exec(move || {
                    let pid = std::process::id();
                    log::debug!("Entered child process {}", pid);
                    match cgroup.jail_me() {
                        Ok(()) => Ok(()),
                        Err(err) => {
                            log::error!("Failed to add PID {} to cgroup: {:#}", pid, err);
                            Err(std::io::Error::from(ErrorKind::Other))
                        }
                    }
                })
                .spawn()
                .context("spawn child process")?
        };
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
