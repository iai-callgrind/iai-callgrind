use std::ffi::OsString;

use log::warn;

use super::{ToolOutputPath, ValgrindTool};
use crate::api::{self};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolArgs {
    tool: ValgrindTool,
    output_paths: Vec<OsString>,
    other: Vec<String>,
}

impl ToolArgs {
    pub fn from_raw_args(tool: ValgrindTool, raw_args: api::RawArgs) -> Self {
        let mut other = vec![];
        for arg in raw_args.0 {
            match arg.split_once('=') {
                Some((
                    "--dhat-out-file" | "--massif-out-file" | "--bb-out-file" | "--pc-out-file",
                    _,
                )) => warn!("Ignoring {} argument: '{arg}'", tool.id()),
                Some(_) => other.push(arg),
                None => {}
            }
        }
        Self {
            tool,
            output_paths: Vec::default(),
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

        let path = &output_path.path;
        let mut extension = path
            .extension()
            .expect("An extension must be present")
            .to_os_string();
        match self.tool {
            ValgrindTool::Callgrind => unreachable!("Callgrind is not managed here"),
            ValgrindTool::Massif => {
                let mut out_file = OsString::from("--massif-out-file=");
                if let Some(modifier) = modifier {
                    extension.push(".");
                    extension.push(modifier.as_ref());
                    out_file.push(path.with_extension(extension));
                } else {
                    out_file.push(path);
                }
                self.output_paths.push(out_file);
            }
            ValgrindTool::DHAT => {
                let mut out_file = OsString::from("--dhat-out-file=");
                if let Some(modifier) = modifier {
                    extension.push(".");
                    extension.push(modifier.as_ref());
                    out_file.push(path.with_extension(extension));
                } else {
                    out_file.push(path);
                }
                self.output_paths.push(out_file);
            }
            ValgrindTool::BBV => {
                let mut bb_extension = extension.clone();
                bb_extension.push(".bb");
                let mut pc_extension = extension.clone();
                pc_extension.push(".pc");
                if let Some(modifier) = modifier {
                    bb_extension.push(".");
                    bb_extension.push(modifier.as_ref());
                    pc_extension.push(".");
                    pc_extension.push(modifier.as_ref());
                }
                let mut bb_out = OsString::from("--bb-out-file=");
                bb_out.push(path.with_extension(bb_extension));
                self.output_paths.push(bb_out);
                let mut pc_out = OsString::from("--pc-out-file=");
                pc_out.push(path.with_extension(pc_extension));
                self.output_paths.push(pc_out);
            }
            _ => {}
        }
    }

    pub fn to_vec(&self) -> Vec<OsString> {
        let mut vec: Vec<OsString> = vec![];

        vec.push(format!("--tool={}", self.tool.id()).into());
        vec.extend(self.other.iter().map(OsString::from));
        vec.extend_from_slice(&self.output_paths);

        vec
    }
}
