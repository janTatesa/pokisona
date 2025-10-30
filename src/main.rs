mod app;
mod command;
mod command_history;
mod config;
mod file_store;
mod markdown;
mod widget;
mod window;

use std::{fs, path::PathBuf};

use clap::{ArgAction, Parser, Subcommand};
use color_eyre::{Result, eyre::OptionExt};

use crate::{app::Pokisona, config::Config};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    subcommand: Option<VaultCommand>,
    #[arg(long, action = ArgAction::SetTrue)]
    use_default_config: bool,
    #[arg(long)]
    file: Option<PathBuf>
}

#[derive(Subcommand)]
enum VaultCommand {
    Open {
        name: String,
        #[arg(long, action = ArgAction::SetTrue)]
        set_default: bool
    },
    Delete {
        name: String
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut path = dirs::data_dir().ok_or_eyre("Cannot determine data dir")?;
    path.push("pokisona");
    fs::create_dir_all(&path)?;
    let cli = Cli::parse();
    let vault_name = match cli.subcommand {
        Some(VaultCommand::Open {
            name,
            set_default: true
        }) => {
            path.push("default");
            fs::write(&path, &name)?;
            path.pop();
            name
        }
        Some(VaultCommand::Open {
            name,
            set_default: false
        }) => name,
        Some(VaultCommand::Delete { name }) => {
            // TODO: create a confirmation prompt
            path.push(name);
            return Ok(fs::remove_dir(&path)?);
        }
        None => {
            path.push("default");
            let name = fs::read_to_string(&path)?;
            path.pop();
            name
        }
    };

    path.extend(["vaults", &vault_name, ".pokisona"]);
    fs::create_dir_all(&path)?;
    path.push("config.toml");
    let config = Config::new(&path, cli.use_default_config)?;
    path.pop();
    path.pop();
    Pokisona::run(vault_name, path, cli.file, config)?;
    Ok(())
}
