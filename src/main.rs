use std::process::ExitCode;

use cargo_autoinherit::auto_inherit;

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
    AutoInherit(AutoInherit),
}

#[derive(clap::Parser)]
#[command(about, author, version)]
#[command(group = clap::ArgGroup::new("mode").multiple(false))]
pub struct AutoInherit {
    /// Run autoinherit in check mode
    ///
    /// Instead of automatically fixing non-inherited dependencies, only check that
    /// none exist, exiting with a non-zero exit code if any are found.
    #[arg(long, group = "mode")]
    pub check: bool,
}

fn main() -> Result<ExitCode, anyhow::Error> {
    let CliWrapper {
        command: CargoInvocation::AutoInherit(AutoInherit { check }),
    } = CliWrapper::parse();

    auto_inherit(check)
}
