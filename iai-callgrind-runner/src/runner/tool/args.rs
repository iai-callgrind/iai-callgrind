//! The module containing all elements for [`ToolArgs`]

/// Module containing the Iai-Callgrind defaults for the command line arguments of all tools
#[allow(missing_docs)]
#[allow(clippy::arbitrary_source_item_ordering)]
pub mod defaults {
    use super::FairSched;

    ////////////////////////////////////////////////////
    // Shared defaults between cachegrind and callgrind
    // Set some reasonable cache sizes. The exact sizes matter less than having fixed sizes, since
    // otherwise callgrind would take them from the CPU and make benchmark runs even more
    // incomparable between machines.
    pub const I1: &str = "32768,8,64";
    pub const D1: &str = "32768,8,64";
    pub const LL: &str = "8388608,16,64";
    pub const CACHE_SIM: bool = true;
    ////////////////////////////////////////////////////

    ////////////////////////////////////////////////////
    // Defaults specific to callgrind
    pub const COMPRESS_POS: bool = false;
    pub const COMPRESS_STRINGS: bool = false;
    pub const COMBINE_DUMPS: bool = false;
    pub const DUMP_LINE: bool = true;
    pub const DUMP_INSTR: bool = false;
    pub const SEPARATE_THREADS: bool = true;
    ////////////////////////////////////////////////////

    ////////////////////////////////////////////////////
    // Shared defaults between error emitting tools like Memcheck
    pub const ERROR_EXIT_CODE_ERROR_TOOL: &str = "201";
    pub const ERROR_EXIT_CODE_OTHER_TOOL: &str = "0";
    ////////////////////////////////////////////////////

    ////////////////////////////////////////////////////
    // Shared defaults between all tools
    pub const TRACE_CHILDREN: bool = true;
    pub const FAIR_SCHED: FairSched = FairSched::Try;
    pub const VERBOSE: bool = false;
    ////////////////////////////////////////////////////
}

use std::ffi::OsString;
use std::fmt::Display;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use log::warn;

use super::path::ToolOutputPath;
use crate::api::{RawArgs, ValgrindTool};
use crate::error::Error;
use crate::util::{bool_to_yesno, yesno_to_bool};

/// The possible values of the --fair-sched cli arg
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FairSched {
    /// Corresponds to `yes`
    Yes,
    /// Corresponds to `no`
    No,
    /// Corresponds to `try`
    Try,
}

/// The arguments to pass to the valgrind tool
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolArgs {
    /// The error exit code for error checking tools like `Memcheck`
    pub error_exitcode: String,
    /// The --fair-sched argument
    pub fair_sched: FairSched,
    /// The logfile paths argument --log-file
    pub log_path: Option<OsString>,
    /// All other arguments
    pub other: Vec<String>,
    /// The output paths argument like --callgrind-out-file, ...
    pub output_paths: Vec<OsString>,
    /// The [`ValgrindTool`]
    pub tool: ValgrindTool,
    /// The --trace-children argument
    pub trace_children: bool,
    /// If --verbose is set to true of false
    pub verbose: bool,
    /// The xtree paths argument --xtree-leak-file
    pub xleak_path: Option<OsString>,
    /// The xtree paths argument --xtree-memory-file
    pub xtree_path: Option<OsString>,
}

impl Display for FairSched {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Self::Yes => "yes",
            Self::No => "no",
            Self::Try => "try",
        };
        write!(f, "{string}")
    }
}

impl FromStr for FairSched {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "no" => Ok(Self::No),
            "yes" => Ok(Self::Yes),
            "try" => Ok(Self::Try),
            _ => Err(anyhow!(
                "Invalid argument for --fair-sched. Valid arguments are: 'yes', 'no', 'try'"
            )),
        }
    }
}

impl ToolArgs {
    /// Try to create a new `ToolArgs` from multiple `RawArgs`
    pub fn try_from_raw_args(tool: ValgrindTool, raw_args: &[&RawArgs]) -> Result<Self> {
        let mut tool_args = Self {
            tool,
            output_paths: Vec::default(),
            log_path: Option::default(),
            error_exitcode: match tool {
                ValgrindTool::Memcheck | ValgrindTool::Helgrind | ValgrindTool::DRD => {
                    defaults::ERROR_EXIT_CODE_ERROR_TOOL.to_owned()
                }
                ValgrindTool::Callgrind
                | ValgrindTool::Massif
                | ValgrindTool::DHAT
                | ValgrindTool::BBV
                | ValgrindTool::Cachegrind => defaults::ERROR_EXIT_CODE_OTHER_TOOL.to_owned(),
            },
            verbose: defaults::VERBOSE,
            other: Vec::default(),
            trace_children: defaults::TRACE_CHILDREN,
            fair_sched: defaults::FAIR_SCHED,
        };

        for args in raw_args {
            for arg in &args.0 {
                let arg = arg.trim();
                match arg.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                    Some(("--error-exitcode", value)) => {
                        value.clone_into(&mut tool_args.error_exitcode);
                    }
                    Some((key @ "--trace-children", value)) => {
                        tool_args.trace_children = yesno_to_bool(value).ok_or_else(|| {
                            Error::InvalidBoolArgument(key.to_owned(), value.to_owned())
                        })?;
                    }
                    Some(("--fair-sched", value)) => {
                        tool_args.fair_sched = FairSched::from_str(value)?;
                    }
                    Some((arg, _)) if is_ignored_outfile_argument(arg) => warn!(
                        "Ignoring {} argument '{arg}': Output/Log files of tools are managed by \
                         Iai-Callgrind",
                        tool.id()
                    ),
                    None if matches!(arg, "-v" | "--verbose") => tool_args.verbose = true,
                    None if is_ignored_argument(arg) => {
                        warn!("Ignoring {} argument '{arg}'", tool.id());
                    }
                    None | Some(_) => tool_args.other.push(arg.to_owned()),
                }
            }
        }

        Ok(tool_args)
    }

    // TODO: memcheck: --xtree-leak-file=<filename> [default: xtleak.kcg.%p]
    /// Set the output file argument depending on the tool of this `ToolArgs`
    pub fn set_output_arg<T>(&mut self, output_path: &ToolOutputPath, modifier: Option<T>)
    where
        T: AsRef<str>,
    {
        if !self.tool.has_output_file() {
            return;
        }

        match self.tool {
            ValgrindTool::Callgrind => {
                let mut arg = OsString::from("--callgrind-out-file=");
                let callgrind_out_path = if let Some(modifier) = modifier {
                    output_path.with_modifiers([modifier.as_ref()])
                } else if self.trace_children {
                    output_path.with_modifiers(["#%p"])
                } else {
                    output_path.clone()
                };
                arg.push(callgrind_out_path.to_path());
                self.output_paths.push(arg);
            }
            ValgrindTool::Massif => {
                let mut arg = OsString::from("--massif-out-file=");
                let massif_out_path = if let Some(modifier) = modifier {
                    output_path.with_modifiers([modifier.as_ref()])
                } else if self.trace_children {
                    output_path.with_modifiers(["#%p"])
                } else {
                    output_path.clone()
                };
                arg.push(massif_out_path.to_path());
                self.output_paths.push(arg);
            }
            ValgrindTool::DHAT => {
                let mut arg = OsString::from("--dhat-out-file=");
                let dhat_out_path = if let Some(modifier) = modifier {
                    output_path.with_modifiers([modifier.as_ref()])
                } else if self.trace_children {
                    output_path.with_modifiers(["#%p"])
                } else {
                    output_path.clone()
                };
                arg.push(dhat_out_path.to_path());
                self.output_paths.push(arg);
            }
            ValgrindTool::BBV => {
                let mut bb_arg = OsString::from("--bb-out-file=");
                let mut pc_arg = OsString::from("--pc-out-file=");
                let (bb_out, pc_out) = if let Some(modifier) = modifier {
                    (
                        output_path.with_modifiers(["bb", modifier.as_ref()]),
                        output_path.with_modifiers(["pc", modifier.as_ref()]),
                    )
                } else if self.trace_children {
                    (
                        output_path.with_modifiers(["bb", "#%p"]),
                        output_path.with_modifiers(["pc", "#%p"]),
                    )
                } else {
                    (
                        output_path.with_modifiers(["bb"]),
                        output_path.with_modifiers(["pc"]),
                    )
                };
                bb_arg.push(bb_out.to_path());
                pc_arg.push(pc_out.to_path());
                self.output_paths.push(bb_arg);
                self.output_paths.push(pc_arg);
            }
            ValgrindTool::Cachegrind => {
                let mut arg = OsString::from("--cachegrind-out-file=");
                let cachegrind_out_path = if let Some(modifier) = modifier {
                    output_path.with_modifiers([modifier.as_ref()])
                } else if self.trace_children {
                    output_path.with_modifiers(["#%p"])
                } else {
                    output_path.clone()
                };
                arg.push(cachegrind_out_path.to_path());
                self.output_paths.push(arg);
            }
            // The other tools don't have an outfile
            _ => {}
        }
    }

    /// Set the logfile argument
    pub fn set_log_arg<T>(&mut self, output_path: &ToolOutputPath, modifier: Option<T>)
    where
        T: AsRef<str>,
    {
        let log_output = if let Some(modifier) = modifier {
            output_path
                .to_log_output()
                .with_modifiers([modifier.as_ref()])
        } else if self.trace_children {
            output_path.to_log_output().with_modifiers(["#%p"])
        } else {
            output_path.to_log_output()
        };
        let mut arg = OsString::from("--log-file=");
        arg.push(log_output.to_path());
        self.log_path = Some(arg);
    }

    /// Convert into a vector of arguments usable as input for [`std::process::Command::args`]
    pub fn to_vec(&self) -> Vec<OsString> {
        let mut vec: Vec<OsString> = vec![];

        vec.push(format!("--tool={}", self.tool.id()).into());
        vec.push(format!("--error-exitcode={}", &self.error_exitcode).into());
        vec.push(format!("--trace-children={}", &bool_to_yesno(self.trace_children)).into());
        vec.push(format!("--fair-sched={}", self.fair_sched).into());
        if self.verbose {
            vec.push("--verbose".into());
        }

        vec.extend(self.other.iter().map(OsString::from));
        vec.extend_from_slice(&self.output_paths);
        if let Some(log_arg) = self.log_path.as_ref() {
            vec.push(log_arg.clone());
        }

        vec
    }
}

/// Return true if this is an ignored argument related to output or logfiles
pub fn is_ignored_outfile_argument(arg: &str) -> bool {
    matches!(
        arg,
        "--dhat-out-file"
            | "--massif-out-file"
            | "--callgrind-out-file"
            | "--cachegrind-out-file"
            | "--bb-out-file"
            | "--pc-out-file"
            | "--log-file"
            | "--log-fd"
            | "--log-socket"
            | "--xml"
            | "--xml-file"
            | "--xml-fd"
            | "--xml-socket"
            | "--xml-user-comment"
            | "--xtree-leak-file"
            | "--xtree-memory-file"
    )
}

/// Return true if this is a generic ignored argument
pub fn is_ignored_argument(arg: &str) -> bool {
    matches!(
        arg,
        "-h" | "--help"
            | "--help-dyn-options"
            | "--help-debug"
            | "--version"
            | "-q"
            | "--quiet"
            | "--tool"
    )
}
