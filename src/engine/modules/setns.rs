use std::path::PathBuf;

use anyhow::Context;
use unshare::{Command, GidMap, Namespace, UidMap};

use crate::engine::ContainerConfig;

pub struct ContainerNamespaces {
    root: Option<PathBuf>,
    unshare_net: bool,
}

impl ContainerNamespaces {
    pub fn new(config: &ContainerConfig) -> anyhow::Result<ContainerNamespaces> {
        Ok(ContainerNamespaces {
            root: config.root.clone(),
            unshare_net: config.unshare_net,
        })
    }

    pub fn apply(&self, command: &mut Command) -> anyhow::Result<()> {
        command.unshare(&[
            Namespace::User,
            Namespace::Pid,
            Namespace::Ipc,
            Namespace::Mount,
        ]);
        if self.unshare_net {
            command.unshare(&[Namespace::Net]);
        }
        command.set_id_maps(
            // Identity map
            vec![UidMap {
                inside_uid: 0,
                outside_uid: 0,
                count: 65536,
            }],
            vec![GidMap {
                inside_gid: 0,
                outside_gid: 0,
                count: 65536,
            }],
        );

        if let Some(root) = &self.root {
            command.chroot_dir(
                root.canonicalize()
                    .with_context(|| format!("canonicalize {root:?}"))?,
            );
            // We are not using pivot_root here, since that without overlayfs,
            // we cannot guarantee that self.root is a mount point whose device
            // is different from the current root directory.
        }

        Ok(())
    }
}
