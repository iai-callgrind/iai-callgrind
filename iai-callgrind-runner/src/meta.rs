use std::path::PathBuf;

use log::debug;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub arch: String,
    pub aslr: bool,
    pub target_dir: PathBuf,
}

impl Metadata {
    pub fn new() -> Self {
        let arch = std::env::consts::ARCH.to_owned();
        debug!("Detected architecture: {}", arch);

        let target_dir = std::env::var_os("CARGO_TARGET_DIR")
            .map_or_else(
                || {
                    cargo_metadata::MetadataCommand::new()
                        .no_deps()
                        .exec()
                        .map_or_else(
                            |_| PathBuf::from("target"),
                            |p| p.target_directory.into_std_path_buf(),
                        )
                },
                PathBuf::from,
            )
            .join("iai")
            .join(std::env::var_os("CARGO_PKG_NAME").map_or_else(PathBuf::new, PathBuf::from));
        debug!("Detected target directory: '{}'", target_dir.display());

        let aslr = std::env::var_os("IAI_ALLOW_ASLR").is_some();
        if aslr {
            debug!("Found IAI_ALLOW_ASLR environment variable. Trying to run with ASLR enabled.");
        }
        Self {
            arch,
            aslr,
            target_dir,
        }
    }
}
