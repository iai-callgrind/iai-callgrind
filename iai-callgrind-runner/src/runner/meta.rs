use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;
use log::{debug, warn};

use super::Error;
use crate::api::{EventKind, RegressionConfig};
use crate::runner::envs;
use crate::util::{resolve_binary_path, yesno_to_bool};

#[derive(Debug, Clone)]
pub struct Cmd {
    pub bin: PathBuf,
    pub args: Vec<OsString>,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub arch: String,
    pub aslr_enabled: bool,
    pub project_root: PathBuf,
    pub target_dir: PathBuf,
    pub valgrind: Cmd,
    pub valgrind_wrapper: Option<Cmd>,
    pub regression_config: Option<RegressionConfig>,
}

impl Metadata {
    pub fn new() -> Result<Self> {
        let arch = std::env::consts::ARCH.to_owned();
        debug!("Detected architecture: {}", arch);
        let meta = cargo_metadata::MetadataCommand::new()
            .no_deps()
            .exec()
            .expect("Querying metadata of cargo workspace succeeds");

        let project_root = meta.workspace_root.into_std_path_buf();
        debug!("Detected project root: '{}'", project_root.display());

        let target_dir = std::env::var_os(envs::CARGO_TARGET_DIR)
            .map_or_else(|| meta.target_directory.into_std_path_buf(), PathBuf::from)
            .join("iai")
            .join(std::env::var_os(envs::CARGO_PKG_NAME).map_or_else(PathBuf::new, PathBuf::from));

        debug!("Detected target directory: '{}'", target_dir.display());

        let aslr_enabled = std::env::var_os(envs::IAI_CALLGRIND_ALLOW_ASLR).is_some();
        if aslr_enabled {
            debug!(
                "Found {} environment variable. Running with ASLR enabled.",
                envs::IAI_CALLGRIND_ALLOW_ASLR
            );
        }

        // Invoke Valgrind, disabling ASLR if possible because ASLR could noise up the results a bit
        let valgrind_path = resolve_binary_path("valgrind")?;
        let valgrind_wrapper = if aslr_enabled {
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
            aslr_enabled,
            target_dir,
            valgrind: Cmd {
                bin: valgrind_path,
                args: vec![],
            },
            valgrind_wrapper,
            project_root,
            regression_config: try_regression_config_from_env()?,
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

fn try_regression_config_from_env() -> Result<Option<RegressionConfig>> {
    let mut regression = None;
    if let Ok(regression_env) = std::env::var("IAI_CALLGRIND_REGRESSION") {
        let regression_env = regression_env.trim();
        if regression_env.is_empty() {
            return Err(Error::EnvironmentVariableError((
                "IAI_CALLGRIND_REGRESSION".to_owned(),
                "No limits found: At least one limit must be specified".to_owned(),
            ))
            .into());
        }

        if regression_env.eq_ignore_ascii_case("default") {
            regression = Some(RegressionConfig::default());
        } else {
            let mut limits = vec![];

            for split in regression_env.split(',') {
                let split = split.trim();

                if let Some((key, value)) = split.split_once('=') {
                    let (key, value) = (key.trim(), value.trim());
                    let event_kind =
                        EventKind::from_str_ignore_case(key).ok_or_else(|| -> anyhow::Error {
                            Error::EnvironmentVariableError((
                                "IAI_CALLGRIND_REGRESSION".to_owned(),
                                format!("Unknown event kind: '{key}'"),
                            ))
                            .into()
                        })?;

                    let pct = value.parse::<f64>().map_err(|error| -> anyhow::Error {
                        Error::EnvironmentVariableError((
                            "IAI_CALLGRIND_REGRESSION".to_owned(),
                            format!("Invalid percentage for '{key}': {error}"),
                        ))
                        .into()
                    })?;
                    limits.push((event_kind, pct));
                } else {
                    return Err(Error::EnvironmentVariableError((
                        "IAI_CALLGRIND_REGRESSION".to_owned(),
                        format!("Invalid format of key/value pair: '{split}'"),
                    ))
                    .into());
                }
            }

            let regression = regression.get_or_insert(RegressionConfig::default());
            regression.limits = limits;
        }
    }

    if let Ok(fail_fast_env) = std::env::var("IAI_CALLGRIND_REGRESSION_FAIL_FAST") {
        if let Some(regression) = regression.as_mut() {
            let fail_fast_env = fail_fast_env.trim();
            let fail_fast = yesno_to_bool(fail_fast_env.to_lowercase().as_str()).ok_or_else(
                || -> anyhow::Error {
                    Error::EnvironmentVariableError((
                        "IAI_CALLGRIND_REGRESSION_FAIL_FAST".to_owned(),
                        format!("Expected 'yes' or 'no' but found: '{fail_fast_env}'"),
                    ))
                    .into()
                },
            )?;

            regression.fail_fast = Some(fail_fast);
        } else {
            warn!(
                "Ignoring IAI_CALLGRIND_REGRESSION_FAIL_FAST: No IAI_CALLGRIND_REGRESSION \
                 environment variable found"
            );
        }
    }

    Ok(regression)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serial_test::serial;
    use EventKind::*;

    use super::*;

    #[test]
    #[serial]
    fn test_try_regression_config_from_env_when_no_envs_present_then_none() {
        std::env::remove_var("IAI_CALLGRIND_REGRESSION");
        std::env::remove_var("IAI_CALLGRIND_REGRESSION_FAIL_FAST");
        assert!(try_regression_config_from_env().unwrap().is_none());
    }

    #[test]
    #[serial]
    fn test_try_regression_config_from_env_when_fail_fast_only_then_none() {
        std::env::remove_var("IAI_CALLGRIND_REGRESSION");
        std::env::set_var("IAI_CALLGRIND_REGRESSION_FAIL_FAST", "yes");
        assert!(try_regression_config_from_env().unwrap().is_none());
    }

    #[rstest]
    #[case::regression_default(Some("default"), None, vec![], None)]
    #[case::regression_default_case_insensitive(Some("DefAulT"), None, vec![], None)]
    #[case::regression_default_and_fail_fast(Some("default"), Some("yes"), vec![], Some(true))]
    #[case::regression_only(Some("Ir=10"), None, vec![(Ir, 10f64)], None)]
    #[case::regression_case_insensitive(Some("EstIMATedCycles=10"), None, vec![(EstimatedCycles, 10f64)], None)]
    #[case::regression_and_fail_fast(Some("Ir=10"), Some("yes"), vec![(Ir, 10f64)], Some(true))]
    #[case::multiple_regression(Some("Ir=10,EstimatedCycles=5"), None, vec![(Ir, 10f64), (EstimatedCycles, 5f64)], None)]
    #[case::multiple_regression_with_whitespace(Some("Ir= 10 ,  EstimatedCycles = 5"), None, vec![(Ir, 10f64), (EstimatedCycles, 5f64)], None)]
    #[case::fail_fast_case_insensitive(Some("default"), Some("YeS"), vec![], Some(true))]
    #[case::fail_fast_no(Some("default"), Some("no"), vec![], Some(false))]
    #[serial]
    fn test_try_regression_config_from_env(
        #[case] regression_var: Option<&str>,
        #[case] fail_fast_var: Option<&str>,
        #[case] expected_limits: Vec<(EventKind, f64)>,
        #[case] expected_fail_fast: Option<bool>,
    ) {
        std::env::remove_var("IAI_CALLGRIND_REGRESSION");
        std::env::remove_var("IAI_CALLGRIND_REGRESSION_FAIL_FAST");

        if let Some(regression_var) = regression_var {
            std::env::set_var("IAI_CALLGRIND_REGRESSION", regression_var);
        }
        if let Some(fail_fast_var) = fail_fast_var {
            std::env::set_var("IAI_CALLGRIND_REGRESSION_FAIL_FAST", fail_fast_var);
        }

        let expected = RegressionConfig {
            limits: expected_limits,
            fail_fast: expected_fail_fast,
        };

        let actual = try_regression_config_from_env().unwrap();
        assert_eq!(actual, Some(expected));
    }

    #[track_caller]
    fn assert_environment_variable_error(actual: &anyhow::Error, var: &str, reason: &str) {
        assert_eq!(
            actual.to_string(),
            Error::EnvironmentVariableError((var.to_owned(), reason.to_owned())).to_string()
        );
    }

    #[rstest]
    #[case::regression_wrong_format_of_key_value_pair(
        Some("Ir:10"),
        None,
        "IAI_CALLGRIND_REGRESSION",
        "Invalid format of key/value pair: 'Ir:10'"
    )]
    #[case::regression_unknown_event_kind(
        Some("WRONG=10"),
        None,
        "IAI_CALLGRIND_REGRESSION",
        "Unknown event kind: 'WRONG'"
    )]
    #[case::regression_invalid_percentage(
        Some("Ir=10.0.0"),
        None,
        "IAI_CALLGRIND_REGRESSION",
        "Invalid percentage for 'Ir': invalid float literal"
    )]
    #[case::regression_empty_limits(
        Some(""),
        None,
        "IAI_CALLGRIND_REGRESSION",
        "No limits found: At least one limit must be specified"
    )]
    #[case::fail_fast_invalid(
        Some("Ir=10"),
        Some("YEAH"),
        "IAI_CALLGRIND_REGRESSION_FAIL_FAST",
        "Expected 'yes' or 'no' but found: 'YEAH'"
    )]
    #[serial]
    fn test_try_regression_config_from_env_then_error(
        #[case] regression_var: Option<&str>,
        #[case] fail_fast_var: Option<&str>,
        #[case] expected_var: &str,
        #[case] expected_reason: &str,
    ) {
        std::env::remove_var("IAI_CALLGRIND_REGRESSION");
        std::env::remove_var("IAI_CALLGRIND_REGRESSION_FAIL_FAST");

        if let Some(regression_var) = regression_var {
            std::env::set_var("IAI_CALLGRIND_REGRESSION", regression_var);
        }
        if let Some(fail_fast_var) = fail_fast_var {
            std::env::set_var("IAI_CALLGRIND_REGRESSION_FAIL_FAST", fail_fast_var);
        }

        assert_environment_variable_error(
            &try_regression_config_from_env().unwrap_err(),
            expected_var,
            expected_reason,
        );
    }
}
