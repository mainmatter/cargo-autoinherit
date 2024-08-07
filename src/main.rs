use cargo_autoinherit::{auto_inherit, AutoInheritConf};

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
    AutoInherit {
        /// Represents inherited dependencies as `package.workspace = true` if possible.
        #[arg(long)]
        prefer_simple_dotted: bool,
    },
}

fn main() -> Result<(), anyhow::Error> {
    let cli = CliWrapper::parse();
    let conf = match cli.command {
        CargoInvocation::AutoInherit {
            prefer_simple_dotted,
        } => AutoInheritConf {
            prefer_simple_dotted,
        },
    };
    auto_inherit(&conf)
}
