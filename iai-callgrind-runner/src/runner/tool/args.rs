use std::ffi::OsString;

use log::warn;

use super::{ToolOutputPath, ValgrindTool};
use crate::api::{self};
use crate::util::{bool_to_yesno, yesno_to_bool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolArgs {
    pub tool: ValgrindTool,
    pub output_paths: Vec<OsString>,
    pub log_path: Option<OsString>,
    pub error_exitcode: String,
    pub verbose: bool,
    pub trace_children: bool,
    pub other: Vec<String>,
}

impl ToolArgs {
    pub fn from_raw_args(tool: ValgrindTool, raw_args: api::RawArgs) -> Self {
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
            trace_children: false,
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
                Some(("--trace-children", value)) => {
                    if let Some(arg) = yesno_to_bool(value) {
                        tool_args.trace_children = arg;
                    } else {
                        warn!(
                            "Ignoring malformed value '{value}' for --trace-children. Expecting \
                             'yes' or 'no'"
                        );
                    }
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

        tool_args
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
                        output_path.with_modifiers(["bb", modifier.as_ref()]),
                        output_path.with_modifiers(["pc", modifier.as_ref()]),
                    )
                } else if self.trace_children {
                    (
                        output_path.with_modifiers(["bb", "%p"]),
                        output_path.with_modifiers(["pc", "%p"]),
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
        // FIXME: ADD verbose
        vec.extend(self.other.iter().map(OsString::from));
        vec.extend_from_slice(&self.output_paths);
        if let Some(log_arg) = self.log_path.as_ref() {
            vec.push(log_arg.clone());
        }

        vec
    }
}
