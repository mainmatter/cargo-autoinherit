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
    AutoInherit,
}

fn main() -> Result<(), anyhow::Error> {
    let _cli = CliWrapper::parse();
    auto_inherit()
}
