use anyhow::{Context, Ok, Result};
use clap::{Parser, Subcommand};
use std::{
    fs::{self},
    path::{Path, PathBuf},
};
use tracing::info;

/// Simple file/directory backup tool (.bak files, _bak directories)
#[derive(Debug, Parser)]
#[command(name = "rbak", version = "1.0.0", about, long_about = None)]
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
        /// Optional destination path for backup file
        #[arg(short, long)]
        dest: Option<PathBuf>,
    },
    /// Backup a directory recursively (creates dir_bak)
    Dir {
        /// Path to directory to backup
        path: PathBuf,
        /// Optional destination path for backup directory
        #[arg(short, long)]
        dest: Option<PathBuf>,
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
        Commands::File { path, dest } => {
            info!("Backing up file: {}", path.display());

            let bak = if let Some(dest_dir) = dest {
                // Custom dest: dest_dir/filename.bak
                let mut bak = dest_dir;
                if let Some(name) = path.file_name() {
                    bak.push(Path::new(name).with_extension("bak"));
                }
                bak
            } else {
                // Default: same dir as source
                backup_path(&path, BackupType::File)
                    .ok_or_else(|| anyhow::anyhow!("Invalid file"))?
            };

            fs::copy(&path, &bak).context("copying file backup")?;
            info!("Created backup file: {}", bak.display());
        }
        Commands::Dir { path, dest } => {
            info!("Backing up directory: {}", path.display());

            let bak_dir = if let Some(dest_dir) = dest {
                // Build backup path relative to dest_dir, reusing backup_path logic
                let orig_backup = backup_path(&path, BackupType::Directory)
                    .ok_or_else(|| anyhow::anyhow!("Invalid directory"))?;
                let bak_dir_name = orig_backup.file_name().unwrap();

                let mut bak_dir = dest_dir;
                bak_dir.push(bak_dir_name);
                bak_dir
            } else {
                backup_path(&path, BackupType::Directory)
                    .ok_or_else(|| anyhow::anyhow!("Invalid directory"))?
            };

            backup_directory(&path, &bak_dir).context("directory backup")?;
            info!("Created backup directory: {}", bak_dir.display());
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

    #[test]
    fn test_backup_file_with_dest() {
        let tmp = TempDir::new().unwrap();
        let src_file = tmp.path().join("test.txt");
        fs::write(&src_file, b"hello").unwrap();

        let dest_dir = tmp.path().join("backups");
        fs::create_dir_all(&dest_dir).unwrap();

        let bak = {
            let mut bak_path = dest_dir.clone();
            bak_path.push(Path::new("test.txt").with_extension("bak"));
            bak_path
        };

        assert!(!bak.exists());

        // Manually simulate the logic:
        let result = if let Some(dest) = Some(dest_dir.clone()) {
            let mut bak = dest;
            bak.push(Path::new("test.txt").with_extension("bak"));
            bak
        } else {
            backup_path(&src_file, BackupType::File).unwrap()
        };

        assert_eq!(result, bak);

        // Perform copy to simulate what main does
        fs::copy(&src_file, &result).unwrap();
        assert!(result.exists());
        assert_eq!(fs::read(&result).unwrap(), b"hello"[..]);
    }

    #[test]
    fn test_backup_path_directory_with_dest() {
        let tmp = TempDir::new().unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        let dest_dir = tmp.path().join("backups");
        fs::create_dir_all(&dest_dir).unwrap();

        let expected_backup_dir = {
            let mut path = dest_dir.clone();
            path.push("src_bak");
            path
        };

        // Use backup_path to get default name (with _bak suffix)
        let default_backup_dir = backup_path(&src_dir, BackupType::Directory).unwrap();

        // Simulate logic in main:
        let bak_dir = if let Some(dest) = Some(dest_dir.clone()) {
            let orig_backup = default_backup_dir.clone();
            let bak_dir_name = orig_backup.file_name().unwrap();

            let mut bak_dir = dest;
            bak_dir.push(bak_dir_name);
            bak_dir
        } else {
            backup_path(&src_dir, BackupType::Directory).unwrap()
        };

        assert_eq!(bak_dir, expected_backup_dir);

        // Now simulate recursive directory copy
        backup_directory(&src_dir, &bak_dir).unwrap();

        // Destination directory should exist
        assert!(bak_dir.exists());
    }
}
