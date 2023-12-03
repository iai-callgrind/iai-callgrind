use std::ffi::OsString;

use log::warn;

use super::{ToolOutputPath, ValgrindTool};
use crate::api::{self};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolArgs {
    tool: ValgrindTool,
    output_paths: Vec<OsString>,
    log_path: Option<OsString>,
    error_exitcode: String,
    other: Vec<String>,
}

impl ToolArgs {
    pub fn from_raw_args(tool: ValgrindTool, raw_args: api::RawArgs) -> Self {
        let mut other = vec![];
        let mut error_exitcode = None;
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
                     iai-callgrind",
                    tool.id()
                ),
                Some(("--error-exitcode", value)) => {
                    error_exitcode = Some(value.to_owned());
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
                None | Some(_) => other.push(arg),
            }
        }
        Self {
            tool,
            output_paths: Vec::default(),
            log_path: None,
            error_exitcode: error_exitcode.unwrap_or_else(|| match tool {
                ValgrindTool::Memcheck | ValgrindTool::Helgrind | ValgrindTool::DRD => {
                    "201".to_owned()
                }
                ValgrindTool::Callgrind
                | ValgrindTool::Massif
                | ValgrindTool::DHAT
                | ValgrindTool::BBV => "0".to_owned(),
            }),
            other,
        }
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
            ValgrindTool::Callgrind => unreachable!("Callgrind is not managed here"),
            ValgrindTool::Massif => {
                let mut arg = OsString::from("--massif-out-file=");
                let massif_out_path = if let Some(modifier) = modifier {
                    output_path.with_modifiers([modifier.as_ref()])
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
        vec.extend(self.other.iter().map(OsString::from));
        vec.extend_from_slice(&self.output_paths);
        if let Some(log_arg) = self.log_path.as_ref() {
            vec.push(log_arg.clone());
        }

        vec
    }
}
