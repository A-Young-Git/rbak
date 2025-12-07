use anyhow::{Context, Ok, Result};
use clap::{Parser, Subcommand};
use std::{
    fs::{self},
    path::{Path, PathBuf},
};
use tracing::info;

/// Simple file/directory backup tool (.bak files, _bak directories)
#[derive(Debug, Parser)]
#[command(name = "rbak", version = "0.1.0", about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
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
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Commands::File { path } => {
            info!("Backing up file: {}", path.display());
            if let Some(bak) = backup_path(&path, BackupType::File) {
                fs::copy(&path, &bak).context("copying file backup")?;
                info!("Created backup file: {}", bak.display());
            } else {
                anyhow::bail!("{} is not a valid file", path.display());
            }
        }
        Commands::Dir { path } => {
            info!("Backing up directory: {}", path.display());
            if let Some(bak_dir) = backup_path(&path, BackupType::Directory) {
                backup_directory(&path, &bak_dir).context("directory backup")?;
                info!("Created backup directory: {}", bak_dir.display());
            } else {
                anyhow::bail!("{} is not a valid directory", path.display());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_path_file() {
        let path = Path::new("Cargo.toml");
        let bak = backup_path(&path, BackupType::File).unwrap();
        assert_eq!(bak.extension().unwrap(), "bak");
    }

    #[test]
    fn test_backup_path_directory() {
        let path = Path::new(".git");
        let bak = backup_path(&path, BackupType::Directory).unwrap();
        assert!(bak.to_string_lossy().ends_with("_bak"));
    }

    #[test]
    fn test_backup_path_invalid_file() {
        let path = Path::new("nonexistent.txt");
        assert!(backup_path(&path, BackupType::File).is_none());
    }

    #[test]
    fn test_backup_directory_smoke() {
        let tmp = TempDir::new().unwrap();
        let src_dir = tmp.path().join("src");
        let src_file = src_dir.join("test.txt");

        fs::create_dir(&src_dir).unwrap();
        fs::write(&src_file, b"hello").unwrap();

        let dst_dir = src_dir.with_file_name("src_bak");
        backup_directory(&src_dir, &dst_dir).unwrap();

        let backed_up = dst_dir.join("test.txt");
        assert!(backed_up.exists());
        assert_eq!(fs::read_to_string(&backed_up).unwrap(), "hello");
    }
}
