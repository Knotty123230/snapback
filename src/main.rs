mod config;
mod backup;
mod util;

use std::path::PathBuf;
use clap::{Parser, Subcommand};

use crate::{backup::Backup, config::Config};

fn main() {
    let args = Args::parse();
    
    match args.command {
        Command::Create { path } => {
            println!("Creating backup for path: {:?}", path);
            //need to handle and get error messaage informative
            let backup = Backup::new(path);
            match backup {
                Ok(mut backup) => {
                    match backup.write_backup() {
                        Ok(_) => {
                            println!("backup written");
                        },
                        Err(e) => {
                            eprintln!("error -> {:#?}", e);
                        },
                    }
                },
                Err(e) => {
                    eprintln!("error -> {:#?}", e);
                },
            }
        }
        Command::Restore { backup_number , path} => {
            println!("Restoring backup #{} to path: {:?}", backup_number, path);
            match Backup::restore(backup_number, &path) {
                Ok(_) => println!("Restore completed successfully"),
                Err(e) => eprintln!("Restore failed: {}", e),
            }
        }
        Command::List { path } => {
            println!("Listing backups for: {:?}", path);
            match Backup::list_backups(&path) {
                Ok(_) => {},
                Err(e) => eprintln!("Failed to list backups: {}", e),
            }
        }
        Command::Config { action } => {
            match action {
                ConfigAction::Show => {
                    let config = Config::load().unwrap_or_default();
                    config.print_config();
                }
                ConfigAction::Init => {
                    match Config::default().save() {
                        Ok(_) => println!("Configuration initialized successfully"),
                        Err(e) => eprintln!("Failed to initialize config: {}", e),
                    }
                }
                ConfigAction::Path { backup_path, info_path } => {
                    let mut config = Config::load().unwrap_or_default();
                    
                    if let Some(backup_path) = backup_path {
                        config.backup_default_path = Some(backup_path);
                    }
                    if let Some(info_path) = info_path {
                        config.backup_info_default_path = Some(info_path);
                    }
                    
                    match config.save() {
                        Ok(_) => println!("Configuration updated successfully"),
                        Err(e) => eprintln!("Failed to update config: {}", e),
                    }
                }
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "backup")]
#[command(version, about = "A backup tool", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create a backup of the specified path
    Create {
        /// Path to directory or file to backup
        path: PathBuf,
    },
    /// Restore a backup by number
    Restore {
        /// Backup number to restore
        backup_number: u32,
        /// Path to directory or file to restore to
        path: PathBuf,
    },
    /// List all available backups for a path
    List {
        /// Path to directory or file to list backups for
        path: PathBuf,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Initialize default configuration file
    Init,
    /// Set backup and info paths
    Path {
        /// Set backup directory path
        #[arg(long)]
        backup_path: Option<String>,
        /// Set backup info directory path
        #[arg(long)]
        info_path: Option<String>,
    },
}
