use std::{
    io::{self, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

use clap::Parser;

static CONTAINERFILE: &'static [u8] = include_bytes!("Containerfile");

static NIX_IMAGE: &'static str = "docker.io/nixos/nix:latest";
static PACKAGES_STR: &'static str = "nixpkgs#python314 nixpkgs#helix nixpkgs#bash";

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Directory to build
    dir: Option<PathBuf>,

    /// Specifies nonstandard config
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

fn main() -> Result<(), io::Error> {
    let mut build_process = Command::new("podman")
        .arg("build")
        .arg(".")
        .arg("-f")
        .arg("-")
        .arg("-t")
        .arg("nix-test")
        .arg("--build-arg")
        .arg(format!("NIX_IMAGE={}", NIX_IMAGE))
        .arg("--build-arg")
        .arg(format!("PACKAGES_STRING={}", PACKAGES_STR))
        // .arg("--build-args")
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = build_process.stdin.as_mut() {
        stdin.write_all(CONTAINERFILE)?;
    }

    build_process.wait_with_output()?;

    Ok(())
}
