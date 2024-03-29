use cargo_autoinherit::auto_inherit;
use clap::Parser;

mod cli;

fn main() -> Result<(), anyhow::Error> {
    let _cli = crate::cli::Cli::parse();
    auto_inherit()
}
