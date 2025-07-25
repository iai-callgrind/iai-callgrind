//! The module containing the [`Metadata`] and [`Cmd`]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use cargo_metadata::TargetKind;
use clap::Parser;
use log::debug;

use super::args::CommandLineArgs;
use super::envs;
use crate::util::resolve_binary_path;

/// The basic commands (like valgrind) to be executed with default arguments
#[derive(Debug, Clone)]
pub struct Cmd {
    /// The arguments for the executable
    pub args: Vec<OsString>,
    /// The path to the executable
    pub bin: PathBuf,
}

/// `Metadata` contains all information that needs to be collected from cargo and the environment
///
/// More specifically, `Metadata` contains global constants, environment variables and command-line
/// arguments, the basic valgrind [`Cmd`], ...
#[derive(Debug, Clone)]
pub struct Metadata {
    /// A string describing the architecture of the CPU that is currently in use (e.g. "x86")
    pub arch: String,
    /// The command-line arguments parsed from the arguments to `cargo bench -- ARGS` as ARGS
    pub args: CommandLineArgs,
    /// The name of the benchmark to run (might be different to the name of the file)
    pub bench_name: String,
    /// The path to the project top-level directory
    pub project_root: PathBuf,
    /// The absolute path of the `HOME` (per default `$WORKSPACE_ROOT/target/iai`). Plus, if
    /// configured, the target of the host like `x86_64-linux-unknown-gnu`. The final component is
    /// the `CARGO_PKG_NAME`.
    ///
    /// Examples:
    /// * `/home/my/workspace/my-project/target/iai/my-project` or
    /// * `/home/my/workspace/my-project/target/iai/x86_64-linux-unknown-gnu/my-project`
    pub target_dir: PathBuf,
    /// The valgrind [`Cmd`]
    pub valgrind: Cmd,
    /// The valgrind wrapper [`Cmd`]
    pub valgrind_wrapper: Option<Cmd>,
}

impl From<&Metadata> for Command {
    fn from(meta: &Metadata) -> Self {
        meta.valgrind_wrapper.as_ref().map_or_else(
            || {
                let meta_cmd = &meta.valgrind;
                let mut cmd = Self::new(&meta_cmd.bin);
                cmd.args(&meta_cmd.args);
                cmd
            },
            |meta_cmd| {
                let mut cmd = Self::new(&meta_cmd.bin);
                cmd.args(&meta_cmd.args);
                cmd
            },
        )
    }
}

impl Metadata {
    /// Create a `new` Metadata
    pub fn new(
        raw_command_line_args: &[String],
        package_name: &str,
        bench_file: &Path,
    ) -> Result<Self> {
        let args = CommandLineArgs::parse_from(raw_command_line_args);

        let arch = std::env::consts::ARCH.to_owned();
        debug!("Detected architecture: {arch}");
        let meta = cargo_metadata::MetadataCommand::new()
            .no_deps()
            .exec()
            .expect("Querying metadata of cargo workspace succeeds");

        let package = meta
            .packages
            .iter()
            .find(|p| p.name == package_name)
            .expect("The package name should exist");
        let bench_name = package
            .targets
            .iter()
            .find_map(|t| {
                (t.kind.contains(&TargetKind::Bench) && t.src_path.ends_with(bench_file))
                    .then_some(t.name.clone())
            })
            .expect("The benchmark name should exist");

        let project_root = meta.workspace_root.into_std_path_buf();
        debug!("Detected project root: '{}'", project_root.display());

        let target_dir = {
            let mut home = args.home.as_ref().map_or_else(
                || {
                    std::env::var_os(envs::CARGO_TARGET_DIR)
                        .map_or_else(|| meta.target_directory.into_std_path_buf(), PathBuf::from)
                        .join("iai")
                },
                Clone::clone,
            );

            if args.separate_targets {
                home = home.join(env!("IC_BUILD_TRIPLE").to_ascii_lowercase());
            }
            home.join(
                std::env::var_os(envs::CARGO_PKG_NAME).map_or_else(PathBuf::new, PathBuf::from),
            )
        };

        debug!("Detected target directory: '{}'", target_dir.display());

        // Invoke Valgrind, disabling ASLR if possible because ASLR could noise up the results a bit
        let valgrind_path = resolve_binary_path("valgrind")?;
        let valgrind_wrapper = if args.allow_aslr.unwrap_or_default() {
            debug!("Running with ASLR enabled");
            None
        } else if cfg!(target_os = "linux") {
            debug!("Trying to run with ASLR disabled: Using 'setarch'");

            if let Ok(set_arch) = resolve_binary_path("setarch") {
                Some(Cmd {
                    bin: set_arch,
                    args: vec![
                        OsString::from(&arch),
                        OsString::from("-R"),
                        OsString::from(&valgrind_path),
                    ],
                })
            } else {
                debug!("Failed to switch ASLR off: 'setarch' not found. Running with ASLR enabled");
                None
            }
        } else if cfg!(target_os = "freebsd") {
            debug!("Trying to run with ASLR disabled: Using 'proccontrol'");

            if let Ok(proc_control) = resolve_binary_path("proccontrol") {
                Some(Cmd {
                    bin: proc_control,
                    args: vec![
                        OsString::from("-m"),
                        OsString::from("aslr"),
                        OsString::from("-s"),
                        OsString::from("disable"),
                        OsString::from(&valgrind_path),
                    ],
                })
            } else {
                debug!(
                    " Failed to switch ASLR off: 'proccontrol' not found. Running with ASLR \
                     enabled"
                );
                None
            }
        } else {
            debug!("Failed to switch ASLR off. No utility available. Running with ASLR enabled");
            None
        };

        Ok(Self {
            arch,
            target_dir,
            valgrind: Cmd {
                bin: valgrind_path,
                args: vec![],
            },
            valgrind_wrapper,
            project_root,
            args,
            bench_name,
        })
    }
}
