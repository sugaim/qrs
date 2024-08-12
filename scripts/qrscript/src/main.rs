use std::env;

use anyhow::ensure;
use clap::{CommandFactory, Parser};
use cmds::Cmd;
use dialoguer::Confirm;
use util::interactive_cmd::interactive_cmd;

mod cmds;
mod util;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: cmds::Commands,
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Off)
        .build();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("⚙️  qrscript: developers' utilities⚙️");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let cli = make_cli()?;
    log::info!("{:?}", cli);
    cli.command.run()
}

fn make_cli() -> anyhow::Result<Cli> {
    if env::args().len() != 1 {
        println!("{}", env::args().collect::<Vec<_>>().join(" "));
        return Ok(Cli::parse());
    }
    println!("🔧 Interactive mode. Start to build command...\n");
    let cmd = Cli::command();
    let args = interactive_cmd(&cmd)?;

    println!("\n🎉 Command is built!");
    println!("\t{}\n", args.join(" "));
    let confirmed = Confirm::new()
        .with_prompt("Run the command?".to_string())
        .report(true)
        .interact()?;

    ensure!(confirmed, "Operation cancelled.");
    Ok(Cli::parse_from(args))
}
