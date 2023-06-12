use unshare::{Command, GidMap, Namespace, UidMap};

pub struct ContainerNamespaces {}

impl ContainerNamespaces {
    pub fn new() -> anyhow::Result<ContainerNamespaces> {
        Ok(ContainerNamespaces {})
    }

    pub fn apply(&self, command: &mut Command) -> anyhow::Result<()> {
        command
            .unshare(&[
                Namespace::User,
                Namespace::Pid,
                Namespace::Ipc,
                Namespace::Mount,
            ])
            .set_id_maps(
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
        Ok(())
    }
}
