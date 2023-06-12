use cgroups_rs::{cgroup_builder::CgroupBuilder, Cgroup, CgroupPid};

use crate::engine::ContainerConfig;

pub struct ContainerCgroup {
    id: String,
    inner: Cgroup,
}

impl ContainerCgroup {
    pub fn new(container_id: &str, config: &ContainerConfig) -> anyhow::Result<ContainerCgroup> {
        let id = format!("picotin-{}", container_id);
        log::debug!("Creating cgroup {}", id);
        let mut cgroup = CgroupBuilder::new(&id);

        if let Some(cpu_mul) = config.cpu_mul {
            let quota = (cpu_mul * 100000.0).round() as i64;

            log::debug!(
                "cgroup: Setting CPU multiplier to {:.2}x (quota {})",
                cpu_mul,
                quota,
            );
            cgroup = cgroup.cpu().period(100000).quota(quota).done();
        }

        if let Some(mem_mib) = config.mem_mib {
            log::debug!("cgroup: Setting memory hard limit to {} MiB", mem_mib);
            cgroup = cgroup
                .memory()
                .memory_hard_limit((mem_mib << 20) as i64)
                .swappiness(0) // disallow usage of swap
                .done();
        }

        let cgroup = cgroup.build(cgroups_rs::hierarchies::auto())?;
        Ok(ContainerCgroup { id, inner: cgroup })
    }

    pub fn try_clone(&self) -> anyhow::Result<ContainerCgroup> {
        Ok(ContainerCgroup {
            id: self.id.clone(),
            inner: Cgroup::new(cgroups_rs::hierarchies::auto(), &self.id)?,
        })
    }

    pub fn jail_pid(&self, pid: u32) -> anyhow::Result<()> {
        log::debug!("Adding PID {} to cgroup {}", pid, self.id);
        self.inner.add_task(CgroupPid::from(pid as u64))?;
        Ok(())
    }

    pub fn delete(&mut self) -> anyhow::Result<()> {
        log::debug!("Deleting cgroup {}", self.id);
        self.inner.delete().map_err(Into::into)
    }
}
