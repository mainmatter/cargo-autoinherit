use cargo_autoinherit::{auto_inherit, AutoInheritConf};
use std::process::ExitCode;

use clap::Parser;

#[derive(Parser)]
#[command(bin_name = "cargo")]
struct CliWrapper {
    #[command(subcommand)]
    command: CargoInvocation,
}

#[derive(Parser)]
pub enum CargoInvocation {
    /// Automatically centralize all dependencies as workspace dependencies.
    #[command(name = "autoinherit")]
    AutoInherit(AutoInheritConf),
}

fn main() -> Result<ExitCode, anyhow::Error> {
    let cli = CliWrapper::parse();
    let CargoInvocation::AutoInherit(conf) = cli.command;
    auto_inherit(conf)
}
