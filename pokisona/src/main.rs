mod app;
mod color;
mod command;
mod markdown_store;
mod window;

use clap::{ArgAction, Parser, Subcommand, command};
use color_eyre::{Result, eyre::OptionExt};

use crate::app::Pokisona;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    subcommand: Option<VaultCommand>
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

    let vault_name = match Cli::parse().subcommand {
        Some(VaultCommand::Open {
            name,
            set_default: true
        }) => {
            path.push("default");
            std::fs::write(&path, &name)?;
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
            return Ok(std::fs::remove_dir(&path)?);
        }
        None => {
            path.push("default");
            let name = std::fs::read_to_string(&path)?;
            path.pop();
            name
        }
    };

    path.extend(["vaults", &vault_name]);

    std::fs::create_dir_all(&path)?;
    Pokisona::run(vault_name, path)?;
    Ok(())
}
