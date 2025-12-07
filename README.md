# rbak

`rbak` is a simple Rust command-line backup tool that creates backups of files and directories:
- Files are backed up with a `.bak` extension (e.g., `file.txt` → `file.bak`)
- Directories are backed up recursively with a `_bak` suffix (e.g., `mydir` → `mydir_bak/`)

## Features

- Lightweight and easy to use CLI
- Recursive directory backups
- Clear error handling and validation
- Cross-platform support using Rust's standard library

## Usage

### Backup a file

`rbak file path/to/file.txt`


This creates `path/to/file.bak`.

### Backup a directory recursively

`rbak dir path/to/directory`


This creates `path/to/directory_bak/` with all contents copied recursively.

### Help

`rbak --help`


## Installation

`cargo install rbak`


## Development

- Written in Rust using `clap` for CLI argument parsing and `anyhow` for error handling.
- Designed with modular and robust code ideal for extension.

## License

MIT



