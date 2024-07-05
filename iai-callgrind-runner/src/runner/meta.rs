use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use clap::Parser;
use log::debug;

use super::args::CommandLineArgs;
use super::envs;
use crate::api::RegressionConfig;
use crate::util::resolve_binary_path;

#[derive(Debug, Clone)]
pub struct Cmd {
    pub bin: PathBuf,
    pub args: Vec<OsString>,
}

/// `Metadata` contains all information that needs to be collected from cargo, global constants,
/// environment variables and command line arguments
#[derive(Debug, Clone)]
pub struct Metadata {
    pub arch: String,
    pub project_root: PathBuf,
    pub target_dir: PathBuf,
    pub valgrind: Cmd,
    pub valgrind_wrapper: Option<Cmd>,
    pub regression_config: Option<RegressionConfig>,
    pub args: CommandLineArgs,
    pub bench_name: String,
}

impl Metadata {
    pub fn new(
        raw_command_line_args: &[String],
        package_name: &str,
        bench_file: &Path,
    ) -> Result<Self> {
        let args = CommandLineArgs::parse_from(raw_command_line_args);

        let arch = std::env::consts::ARCH.to_owned();
        debug!("Detected architecture: {}", arch);
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
                (t.kind.iter().any(|k| k == "bench") && t.src_path.ends_with(bench_file))
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
            regression_config: Into::<Option<RegressionConfig>>::into(&args),
            args,
            bench_name,
        })
    }
}

impl From<&Metadata> for Command {
    fn from(meta: &Metadata) -> Self {
        meta.valgrind_wrapper.as_ref().map_or_else(
            || {
                let meta_cmd = &meta.valgrind;
                let mut cmd = Command::new(&meta_cmd.bin);
                cmd.args(&meta_cmd.args);
                cmd
            },
            |meta_cmd| {
                let mut cmd = Command::new(&meta_cmd.bin);
                cmd.args(&meta_cmd.args);
                cmd
            },
        )
    }
}
