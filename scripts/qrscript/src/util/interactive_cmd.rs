use anyhow::bail;
use clap::{builder::PossibleValue, Arg, Command};
use dialoguer::{Input, Select};

pub fn interactive_cmd(cmd: &Command) -> anyhow::Result<Vec<String>> {
    let mut cmd_str = Default::default();
    ask_cmd_args(cmd, &mut cmd_str)?;
    Ok(cmd_str)
}

fn ask_cmd_args(cmd: &Command, dst: &mut Vec<String>) -> anyhow::Result<()> {
    dst.push(cmd.get_name().to_string());
    let subcmd_names: Vec<_> = cmd.get_subcommands().map(|c| c.get_name()).collect();
    if !subcmd_names.is_empty() {
        let selection = Select::new()
            .items(&subcmd_names)
            .default(0)
            .with_prompt("Select subcommand")
            .interact()?;

        let subcmd = cmd.get_subcommands().nth(selection).unwrap();
        ask_cmd_args(subcmd, dst)
    } else {
        let args = cmd.get_arguments();
        for arg in args {
            ask_arg(arg, dst)?;
        }
        Ok(())
    }
}

fn ask_arg(arg: &Arg, dst: &mut Vec<String>) -> anyhow::Result<()> {
    let mut ask_msg = "".to_string();
    if let Some(help) = arg.get_help() {
        ask_msg.push_str(&format!("{}. ", help));
    }
    let option = get_option_str(arg)?;
    ask_msg.push_str(&option);

    let possible_values = arg.get_possible_values();
    let default = arg.get_default_values().first().and_then(|s| s.to_str());

    let input = if !possible_values.is_empty() {
        ask_enum(&ask_msg, possible_values)?
    } else {
        let input = Input::<String>::new()
            .allow_empty(true)
            .with_prompt(ask_msg);
        match default {
            Some(default) => input.default(default.to_string()).interact()?,
            None => input.interact()?,
        }
    };

    if input.is_empty() {
        return Ok(());
    }
    dst.push(option);
    dst.push(input.trim().to_string());
    Ok(())
}

fn ask_enum(msg: &str, possibilities: Vec<PossibleValue>) -> anyhow::Result<String> {
    let items: Vec<_> = possibilities.iter().map(|p| p.get_name()).collect();
    let selection = Select::new()
        .items(&items)
        .default(0)
        .with_prompt(msg)
        .interact()?;
    Ok(possibilities[selection].get_name().to_string())
}

fn get_option_str(arg: &Arg) -> anyhow::Result<String> {
    if let Some(long) = arg.get_long() {
        Ok(format!("--{long}"))
    } else if let Some(short) = arg.get_short() {
        Ok(format!("-{short}"))
    } else {
        bail!("Positional argument is not supported");
    }
}
