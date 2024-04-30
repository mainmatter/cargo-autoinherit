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

    /// No changes but exit code 1 if anything would be changed or any dependencies cannot be inherited.
    #[arg(short, long, action)]
    dry_run: bool,
}

fn main() -> Result<(), anyhow::Error> {
    let cli = CliWrapper::parse();

    match cli.command {
        CargoInvocation::AutoInherit(args) => {
            let code = auto_inherit(args.exclude, args.dry_run)?;
            if code != 0 {
                std::process::exit(code);
            } else {
                Ok(())
            }
        }
    }
}
