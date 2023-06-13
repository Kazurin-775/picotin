use std::path::PathBuf;

pub struct ContainerConfig {
    pub root: Option<PathBuf>,
    pub command: Option<PathBuf>,

    pub cpu_mul: Option<f32>,
    pub mem_mib: Option<u64>,
}
