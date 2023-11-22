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
    // TODO: Sort out --tool
    pub fn from_raw_args(tool: ValgrindTool, raw_args: api::RawArgs) -> Self {
        let mut other = vec![];
        let mut error_exitcode = None;
        for arg in raw_args.0 {
            match arg
                .trim()
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
            {
                Some((
                    "--dhat-out-file" | "--massif-out-file" | "--bb-out-file" | "--pc-out-file"
                    | "--log-file",
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

        let mut output_path = output_path.clone();
        match self.tool {
            ValgrindTool::Callgrind => unreachable!("Callgrind is not managed here"),
            ValgrindTool::Massif => {
                let mut arg = OsString::from("--massif-out-file=");
                if let Some(modifier) = modifier {
                    output_path
                        .extension
                        .push_str(&format!(".{}", modifier.as_ref()));
                }
                arg.push(output_path.to_path());
                self.output_paths.push(arg);
            }
            ValgrindTool::DHAT => {
                let mut arg = OsString::from("--dhat-out-file=");
                if let Some(modifier) = modifier {
                    output_path
                        .extension
                        .push_str(&format!(".{}", modifier.as_ref()));
                }
                arg.push(output_path.to_path());
                self.output_paths.push(arg);
            }
            ValgrindTool::BBV => {
                let mut bb_arg = OsString::from("--bb-out-file=");
                let mut pc_arg = OsString::from("--pc-out-file=");
                let mut bb_out = output_path.clone();
                let mut pc_out = output_path;
                bb_out.extension.push_str(".bb");
                pc_out.extension.push_str(".pc");
                if let Some(modifier) = modifier {
                    bb_out
                        .extension
                        .push_str(&format!(".{}", modifier.as_ref()));
                    pc_out
                        .extension
                        .push_str(&format!(".{}", modifier.as_ref()));
                }
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
        let mut log_output = output_path.to_log_output();
        if let Some(modifier) = modifier {
            log_output
                .extension
                .push_str(&format!(".{}", modifier.as_ref()));
        }
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
