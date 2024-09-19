use std::ffi::OsString;
use std::fmt::Display;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use log::warn;

use super::{ToolOutputPath, ValgrindTool};
use crate::api::{self};
use crate::error::Error;
use crate::util::{bool_to_yesno, yesno_to_bool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FairSched {
    Yes,
    No,
    Try,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolArgs {
    pub tool: ValgrindTool,
    pub output_paths: Vec<OsString>,
    pub log_path: Option<OsString>,
    pub error_exitcode: String,
    pub verbose: bool,
    pub trace_children: bool,
    pub fair_sched: FairSched,
    pub other: Vec<String>,
}

impl Display for FairSched {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            FairSched::Yes => "yes",
            FairSched::No => "no",
            FairSched::Try => "try",
        };
        write!(f, "{string}")
    }
}

impl FromStr for FairSched {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "no" => Ok(FairSched::No),
            "yes" => Ok(FairSched::Yes),
            "try" => Ok(FairSched::Try),
            _ => Err(anyhow!(
                "Invalid argument for --fair-sched. Valid arguments are: 'yes', 'no', 'try'"
            )),
        }
    }
}

impl ToolArgs {
    pub fn try_from_raw_args(tool: ValgrindTool, raw_args: api::RawArgs) -> Result<Self> {
        let mut tool_args = Self {
            tool,
            output_paths: Vec::default(),
            log_path: Option::default(),
            error_exitcode: match tool {
                ValgrindTool::Memcheck | ValgrindTool::Helgrind | ValgrindTool::DRD => {
                    "201".to_owned()
                }
                ValgrindTool::Callgrind
                | ValgrindTool::Massif
                | ValgrindTool::DHAT
                | ValgrindTool::BBV => "0".to_owned(),
            },
            verbose: false,
            other: Vec::default(),
            trace_children: true,
            fair_sched: FairSched::Try,
        };

        for arg in raw_args.0 {
            match arg
                .trim()
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
            {
                Some(("--tool", _)) => warn!("Ignoring {} argument '{arg}'", tool.id()),
                Some((
                    "--dhat-out-file" | "--massif-out-file" | "--bb-out-file" | "--pc-out-file"
                    | "--log-file" | "--log-fd" | "--log-socket" | "--xml" | "--xml-file"
                    | "--xml-fd" | "--xml-socket" | "--xml-user-comment",
                    _,
                )) => warn!(
                    "Ignoring {} argument '{arg}': Output/Log files of tools are managed by \
                     Iai-Callgrind",
                    tool.id()
                ),
                Some(("--error-exitcode", value)) => {
                    value.clone_into(&mut tool_args.error_exitcode);
                }
                Some((key @ "--trace-children", value)) => {
                    tool_args.trace_children = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument((key.to_owned(), value.to_owned()))
                    })?;
                }
                Some(("--fair-sched", value)) => {
                    tool_args.fair_sched = FairSched::from_str(value)?;
                }
                None if matches!(
                    arg.as_str(),
                    "-h" | "--help"
                        | "--help-dyn-options"
                        | "--help-debug"
                        | "--version"
                        | "-q"
                        | "--quiet"
                ) =>
                {
                    warn!("Ignoring {} argument '{arg}'", tool.id());
                }
                None if matches!(arg.as_str(), "--verbose") => tool_args.verbose = true,
                None | Some(_) => tool_args.other.push(arg),
            }
        }

        Ok(tool_args)
    }

    // TODO: memcheck: --xtree-leak-file=<filename> [default: xtleak.kcg.%p]
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
                    output_path.with_modifiers(["%p"])
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
                    output_path.with_modifiers(["%p"])
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
                    output_path.with_modifiers(["%p"])
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
                        output_path.with_modifiers([modifier.as_ref(), "bb"]),
                        output_path.with_modifiers([modifier.as_ref(), "pc"]),
                    )
                } else if self.trace_children {
                    (
                        output_path.with_modifiers(["%p", "bb"]),
                        output_path.with_modifiers(["%p", "pc"]),
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
            // The other tools don't have an outfile
            _ => {}
        }
    }

    pub fn set_log_arg<T>(&mut self, output_path: &ToolOutputPath, modifier: Option<T>)
    where
        T: AsRef<str>,
    {
        let log_output = if let Some(modifier) = modifier {
            output_path
                .to_log_output()
                .with_modifiers([modifier.as_ref()])
        } else if self.trace_children {
            output_path.to_log_output().with_modifiers(["%p"])
        } else {
            output_path.to_log_output()
        };
        let mut arg = OsString::from("--log-file=");
        arg.push(log_output.to_path());
        self.log_path = Some(arg);
    }

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
