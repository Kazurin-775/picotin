use std::path::PathBuf;

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
        root: Option<PathBuf>,

        #[arg(long)]
        cpu_mul: Option<f32>,
        #[arg(long)]
        mem_mib: Option<u64>,

        #[arg(long)]
        no_unshare_net: bool,

        command: Option<PathBuf>,
    },
    Link {
        lhs: String,
        rhs: String,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module("picotin", log::LevelFilter::Debug)
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Commands::New {
            root,
            cpu_mul,
            mem_mib,
            no_unshare_net,
            command,
        } => {
            let config = engine::ContainerConfig {
                root,
                command,
                cpu_mul,
                mem_mib,
                unshare_net: !no_unshare_net,
            };
            let container = Container::new(config).context("create container")?;
            container.run().context("run container")?;
        }
        Commands::Link { lhs, rhs } => {
            engine::add_veth_link(&lhs, &rhs)?;
        }
    }
    Ok(())
}
