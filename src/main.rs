use core::str;
use std::{
    collections::HashSet,
    ffi::OsString,
    fs,
    io::{self, BufRead, BufReader, Write},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};

use clap::{Parser, Subcommand};
use directories_next::ProjectDirs;
use serde::Deserialize;

/// Containerfile used to build nix image and copy packages into dev image
static CONTAINERFILE: &'static [u8] = include_bytes!("Containerfile");

/// Used by serde to generate a default docker name
fn default_docker_name() -> String {
    "podman".to_string()
}

/// Used by serde to generate a default nix docker image to pull
fn default_nix_image() -> String {
    "docker.io/nixos/nix:latest".to_string()
}

/// Used by serde to generate default base packages to install
fn default_base_packages() -> HashSet<String> {
    // list adapted from
    // https://github.com/89luca89/distrobox/blob/main/distrobox-init
    // and
    // https://github.com/devcontainers/features/blob/main/src/common-utils/main.sh
    HashSet::from_iter(
        vec![
            "bash",
            "bash-completion",
            "bc",
            "curl",
            "diffutils",
            "findutils",
            "glibc",
            "gnupg",
            "iputils",
            "inetutils",
            "keyutils",
            "less",
            "lsof",
            "man",
            "mlocate",
            "mtr",
            "ncurses",
            "nssmdns",
            "openssh",
            "pigz",
            "pinentry-tty",
            "procps",
            "rsync",
            "shadow",
            "sudo",
            "tcpdump",
            "time",
            "traceroute",
            "tree",
            "tzdata",
            "unzip",
            "util-linux",
            "wget",
            "zip",
        ]
        .iter()
        .map(|s| s.to_string()),
    )
}

/// Stores the values used to configure this application.
#[derive(Debug, Deserialize)]
struct Config {
    /// The name of the docker-compatible cli to use. This should be "podman"
    /// if podman is installed.
    #[serde(default = "default_docker_name")]
    docker_name: String,

    /// Base image to build all the nix packages from. This must have nix
    /// cli installed. If not specified, the value defaults to
    /// docker.io/nixos/nix:latest.
    #[serde(default = "default_nix_image")]
    nix_image: String,

    /// The base packages to install into the environment. This defaults to a
    /// vector of various nixpkgs that tend to be useful, such as git.
    #[serde(default = "default_base_packages")]
    base_packages: HashSet<String>,

    /// Additional packages to install into the environment. This defaults to
    //// an empty vector and is always user specified.
    additional_packages: HashSet<String>,
}

impl Config {
    fn all_packages(&self) -> String {
        let all_packages = self.base_packages.union(&self.additional_packages);

        all_packages
            .into_iter()
            .map(|s| format!("nixpkgs#{}", s))
            .reduce(|a, b| format!("{} {}", a, b))
            .unwrap_or_default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            docker_name: default_docker_name(),
            nix_image: default_nix_image(),
            base_packages: default_base_packages(),
            additional_packages: Default::default(),
        }
    }
}

/// TODO
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Whether to build a containerfile or use an image
    #[command(subcommand)]
    mode: Mode,

    /// Directory to map into the container
    workspace: PathBuf,

    /// Override default config directory
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

/// Where to obtain the dev image from
#[derive(Subcommand)]
enum Mode {
    /// Build and set up a containerfile
    Containerfile {
        /// Specify the containerfile to build from
        #[arg(value_name = "CONTAINERFILE")]
        containerfile: PathBuf,
    },

    /// Pull or use an existing image
    Image {
        /// Specify the image to use or pull
        #[arg(value_name = "IMAGE NAME")]
        image: String,
    },
}

/// Returns proper config for the application based on the following logic:
/// If the override file exists, parse it. Otherwise, if a config in the
/// default location exists, parse it. If neither exist, use the default
/// config values. Any failed parse returns an `std::io::Error`.
///
/// The `directories_next` crate is used to ensure cross platform
/// compatibility, although the chances this app works on windows are rather
/// low.
fn parse_config(config_override: Option<PathBuf>) -> Result<Config, io::Error> {
    // Returns the default config text if it exists or can be read,
    // otherwise returns None
    fn default_config_text() -> Option<String> {
        let project_dirs = ProjectDirs::from("io.github", "anglesideangle", "yadt");
        let path = project_dirs.map(|dirs| dirs.config_dir().to_path_buf());
        path.map(|p| fs::read_to_string(p).ok()).flatten()
    }

    // bad override config should fail
    let override_text = config_override.map(fs::read_to_string).transpose()?;

    if let Some(config_text) = override_text.or_else(default_config_text) {
        toml::from_str(&config_text)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.message()))
    } else {
        Ok(Default::default())
    }
}

fn main() -> Result<(), io::Error> {
    // clap is actually magic
    let cli = Cli::parse();

    let config = parse_config(cli.config)?;

    let dev_image = match cli.mode {
        Mode::Containerfile { containerfile } => {
            let mut cmd = Command::new(&config.docker_name)
                .arg("build")
                .arg("-f")
                .arg(
                    fs::canonicalize(containerfile)
                        .expect("Could not canonicalize containerfile path"),
                )
                .stdout(Stdio::piped())
                .spawn()?;

            BufReader::new(
                cmd.stdout
                    .take()
                    .expect("Could not capture build process stdout."),
            )
            .lines()
            .last()
            .expect("Build command did not write to stdout.")
            .expect("Could not read last line of stdout")
        }
        Mode::Image { image } => image,
    };

    // TODO
    // if desired nix packages exist on the home system, copy them in to save
    // bandwidth
    // or just mount the entire host /nix into the nix image
    // or get rid of the base image and just copy stuff from the host system

    let mut workspace_arg = OsString::from("WORKSPACE=");
    workspace_arg.push(fs::canonicalize(&cli.workspace)?);

    let mut build_process = Command::new(&config.docker_name)
        .arg("build")
        .arg("-f")
        .arg("-")
        .arg("--build-arg")
        .arg(format!("NIX_IMAGE={}", config.nix_image))
        .arg("--build-arg")
        .arg(format!("DEV_IMAGE={}", dev_image))
        .arg("--build-arg")
        .arg(format!("PACKAGES_STRING={}", config.all_packages()))
        // .arg("--build-arg")
        // .arg(format!("USERNAME={}", username))
        // .arg("--build-arg")
        // .arg(format!("UID={}", uid))
        // .arg("--build-arg")
        // .arg(format!("GID={}", gid))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = build_process
        .stdin
        .take()
        .expect("Could not capture build process stdin.");

    let stdout = build_process
        .stdout
        .take()
        .expect("Could not capture build process stdout.");

    // write containerfile from this app's binary to the build process stdin
    // this seems to need to be in a separate thread because
    thread::spawn(move || stdin.write_all(CONTAINERFILE));

    // build_process.wait_with_output()?;
    // build_process.wait()?;

    let reader = BufReader::new(stdout);

    // this is a hack to forward command's progress to stdout while keeping it
    // an iterator. ideally the side effects from this shouldn't matter
    let container_id = reader
        .lines()
        .map(|line| {
            if let Ok(l) = &line {
                println!(">>> {}", l);
            }
            line
        })
        .last()
        .expect("Build command did not write to stdout.")
        .expect("Could not read last line of stdout");

    println!("hash: {:?}", container_id);

    // build_process.wait()?;

    // let mut workspace_volume_arg = OsString::from("WORKSPACE=");
    let mut workspace_vol_arg = fs::canonicalize(&cli.workspace)?;
    workspace_vol_arg.push(":/workspace:rw");

    Command::new(&config.docker_name)
        .arg("run")
        .arg("--rm")
        .arg("--tty")
        .arg("--interactive")
        .arg("--volume")
        .arg(workspace_vol_arg)
        .arg("--workdir")
        .arg("/workspace")
        .arg("--userns")
        .arg("keep-id") // TODO this creates a hard dependency on podman
        .arg("--name")
        .arg("yadt-test-run")
        .arg("--http-proxy") // making the most of the podman dep
        .arg("--network")
        .arg("host")
        .arg("--env-merge")
        .arg("PATH=${PATH}:/yadt-bin")
        .arg(container_id)
        .arg("/bin/bash")
        .exec();
    // .output()?;

    // println!("hmm");

    // .arg("--user")
    // .arg(format!("{}:{}", uid))
    // .arg("userns")

    Ok(())
}
