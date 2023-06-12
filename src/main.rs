use anyhow::Context;
use clap::{Parser, Subcommand};
use engine::Container;

mod engine;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    New {
        #[arg(long)]
        cpu_mul: Option<f32>,
        #[arg(long)]
        mem_mib: Option<u64>,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Commands::New { cpu_mul, mem_mib } => {
            let config = engine::ContainerConfig { cpu_mul, mem_mib };
            let container = Container::new(config).context("create container")?;
            container.run().context("run container")?;
        }
    }
    Ok(())
}
