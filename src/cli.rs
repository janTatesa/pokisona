use std::{env, fs};

use clap::{ArgAction, Parser, Subcommand};
use color_eyre::{Result, eyre::OptionExt};

use crate::{PathBuf, config::Config, file_store::FileLocator};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    subcommand: Option<VaultCommand>,
    #[arg(long, action = ArgAction::SetTrue)]
    use_default_config: bool,
    #[arg(long)]
    file: Option<FileLocator>
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

pub struct VaultName(pub String);
pub struct InitialFile(pub Option<FileLocator>);
pub fn handle_args() -> Result<(VaultName, InitialFile, Config)> {
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
            fs::remove_dir(&path)?;
            std::process::exit(0);
        }
        None => {
            path.push("default");
            let name = fs::read_to_string(&path)?;
            path.pop();
            name
        }
    };

    path.extend(["vaults", &vault_name]);
    fs::create_dir_all(&path)?;
    env::set_current_dir(&path)?;

    // TODO: this should be conditional
    fs::create_dir_all(".pokisona")?;
    let path = &PathBuf::from_iter([".pokisona", "config.toml"]);
    // TODO: maybe there could be a cross-vault config
    let config = Config::new(path, cli.use_default_config)?;
    Ok((VaultName(vault_name), InitialFile(cli.file), config))
}
