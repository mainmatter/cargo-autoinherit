use cargo_autoinherit::auto_inherit;

use clap::{Args, Parser};

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
    AutoInherit(AutoInheritArgs),
}

#[derive(Args)]
pub struct AutoInheritArgs {
    /// Package name(s) of workspace member(s) to exclude.
    #[arg(short, long)]
    exclude: Vec<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let cli = CliWrapper::parse();

    match cli.command {
        CargoInvocation::AutoInherit(args) => auto_inherit(args.exclude),
    }
}
