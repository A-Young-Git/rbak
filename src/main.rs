/*
* ********** TODOS *******************
* Use clap for turning this into a CLI app
* Add docstrings to this for better documentation
* Add logging etc to improve program
* ************************************
*/

use anyhow::{Context, Ok, Result};
use clap::{Parser, Subcommand};
use std::{
    fs::{self},
    path::{Path, PathBuf},
};

/// Simple file/directory backup tool (.bak files, _bak directories)
#[derive(Debug, Parser)]
#[command(name = "rbak", version = "0.1.0", about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Backup a single file (creates file.bak)
    File {
        /// Path to file to backup
        path: PathBuf,
    },
    /// Backup a directory recursively (creates dir_bak)
    Dir {
        /// Path to directory to backup
        path: PathBuf,
    },
}

pub enum BackupType {
    File,
    Directory,
}

/// Creates a backup path with appropriate suffix (.bak for files, _bak for directories).
///
/// Returns `None` if the path doesn't match the specified `BackupType`.
pub fn backup_path(path: &Path, kind: BackupType) -> Option<PathBuf> {
    let metadata = fs::metadata(path).ok()?;

    let (is_valid, suffix) = match kind {
        BackupType::File if metadata.is_file() => (true, "bak"),
        BackupType::Directory if metadata.is_dir() => (true, "_bak"),
        _ => (false, ""),
    };

    if !is_valid {
        return None;
    }

    let parent = path.parent()?;
    let name = path.file_name()?;
    let mut bak_path = PathBuf::from(parent);

    match kind {
        BackupType::File => bak_path.push(Path::new(name).with_extension(suffix)),
        BackupType::Directory => bak_path.push(format!(
            "{}_{}",
            name.to_string_lossy(),
            suffix.trim_start_matches('_')
        )),
    }
    Some(bak_path)
}

/// Recursively copies a directory tree to the destination.
///
/// Creates all necessary parent directories and handles files/subdirectories.
pub fn backup_directory(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).context("creating backup directory tree")?;

    for entry in fs::read_dir(src).context("reading source directory")? {
        let entry = entry.context("reading directory entry")?;
        let file_type = entry.file_type().context("getting file type")?;
        let src_path = entry.path();
        let mut dst_path = PathBuf::from(dst);
        dst_path.push(entry.file_name());

        if file_type.is_dir() {
            backup_directory(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path).context("copying file")?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::File { path } => {
            println!("Backing up file {:?}", path);
            if let Some(bak) = backup_path(&path, BackupType::File) {
                fs::copy(&path, &bak).context("copying file backup")?;
                println!("Created {:?}", bak);
            } else {
                anyhow::bail!("{} is not a valid file", path.display());
            }
        }
        Commands::Dir { path } => {
            println!("Backing up directory: {:?}", path);
            if let Some(bak_dir) = backup_path(&path, BackupType::Directory) {
                backup_directory(&path, &bak_dir).context("director backup")?;
                println!("Created: {:?}", bak_dir);
            } else {
                anyhow::bail!("{} is not a valid directory", path.display());
            }
        }
    }

    Ok(())
}
