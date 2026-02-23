use std::{env, fs};

use clap::{ArgAction, Parser, Subcommand};
use color_eyre::{Result, eyre::OptionExt};

use crate::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    subcommand: Option<VaultCommand>
}

#[derive(Subcommand)]
enum VaultCommand {
    Open {
        #[arg(long)]
        file: Option<PathBuf>,

        name: String,
        #[arg(long, action = ArgAction::SetTrue)]
        set_default: bool
    },
    Delete {
        name: String
    }
}

pub struct VaultName(pub String);
pub struct InitialFile(pub Option<PathBuf>);
pub fn handle_args() -> Result<(VaultName, InitialFile)> {
    let mut path = dirs::data_dir().ok_or_eyre("Cannot determine data dir")?;
    path.push("pokisona");
    fs::create_dir_all(&path)?;
    let cli = Cli::parse();
    let (vault_name, file) = match cli.subcommand {
        Some(VaultCommand::Open {
            name,
            set_default: true,
            file
        }) => {
            path.push("default");
            fs::write(&path, &name)?;
            path.pop();
            (name, file)
        }
        Some(VaultCommand::Open {
            name,
            set_default: false,
            file
        }) => (name, file),
        Some(VaultCommand::Delete { name }) => {
            // TODO: create a confirmation prompt
            path.push(name);
            fs::remove_dir(&path)?;
            std::process::exit(0);
        }
        None => {
            path.push("default");
            let name = fs::read_to_string(&path)?;
            path.pop();
            (name, None)
        }
    };

    path.extend(["vaults", &vault_name]);
    fs::create_dir_all(&path)?;
    env::set_current_dir(&path)?;

    fs::create_dir_all(".pokisona")?;

    Ok((VaultName(vault_name), InitialFile(file)))
}
