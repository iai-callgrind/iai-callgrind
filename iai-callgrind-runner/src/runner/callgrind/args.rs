use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;
use log::{log_enabled, warn};

use crate::api::RawArgs;
use crate::error::Error;
use crate::runner::tool;
use crate::util::{bool_to_yesno, yesno_to_bool};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Args {
    i1: String,
    d1: String,
    ll: String,
    cache_sim: bool,
    other: Vec<String>,
    toggle_collect: VecDeque<String>,
    compress_strings: bool,
    compress_pos: bool,
    verbose: bool,
    dump_instr: bool,
    dump_line: bool,
    combine_dumps: bool,
    callgrind_out_file: Option<PathBuf>,
    log_arg: Option<OsString>,
}

impl Args {
    pub fn from_raw_args(args: &[&RawArgs]) -> Result<Self> {
        let mut default = Self::default();
        default.update(args.iter().flat_map(|s| &s.0))?;
        Ok(default)
    }

    pub fn update<'a, T: Iterator<Item = &'a String>>(&mut self, args: T) -> Result<()> {
        for arg in args {
            match arg
                .trim()
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
            {
                Some(("--I1", value)) => value.clone_into(&mut self.i1),
                Some(("--D1", value)) => value.clone_into(&mut self.d1),
                Some(("--LL", value)) => value.clone_into(&mut self.ll),
                Some((key @ "--dump-instr", value)) => {
                    self.dump_instr = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidCallgrindBoolArgument((key.to_owned(), value.to_owned()))
                    })?;
                }
                Some((key @ "--dump-line", value)) => {
                    self.dump_line = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidCallgrindBoolArgument((key.to_owned(), value.to_owned()))
                    })?;
                }
                Some(("--toggle-collect", value)) => {
                    self.toggle_collect.push_back(value.to_owned());
                }
                Some((
                    key @ ("--separate-threads"
                    | "--callgrind-out-file"
                    | "--compress-strings"
                    | "--compress-pos"
                    | "--combine-dumps"
                    | "--log-file"
                    | "--log-fd"
                    | "--log-socket"
                    | "--xml"
                    | "--xml-file"
                    | "--xml-fd"
                    | "--xml-socket"
                    | "--xml-user-comment"
                    | "--tool"),
                    value,
                )) => {
                    warn!("Ignoring callgrind argument: '{}={}'", key, value);
                }
                Some(_) => self.other.push(arg.clone()),
                None if arg == "-v" || arg == "--verbose" => self.verbose = true,
                None if matches!(
                    arg.trim(),
                    "-h" | "--help"
                        | "--help-dyn-options"
                        | "--help-debug"
                        | "--version"
                        | "-q"
                        | "--quiet"
                ) =>
                {
                    warn!("Ignoring callgrind argument: '{arg}'");
                }
                None if arg.starts_with('-') => self.other.push(arg.clone()),
                // ignore positional arguments for now. It might be a filtering argument for cargo
                // bench
                None => {}
            }
        }
        Ok(())
    }

    // Insert the --toggle-collect argument at the start
    //
    // This is pure cosmetics, since callgrind doesn't prioritize the toggles by any order
    pub fn insert_toggle_collect(&mut self, arg: &str) {
        self.toggle_collect.push_front(arg.to_owned());
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            // Set some reasonable cache sizes. The exact sizes matter less than having fixed sizes,
            // since otherwise callgrind would take them from the CPU and make benchmark runs
            // even more incomparable between machines.
            i1: String::from("32768,8,64"),
            d1: String::from("32768,8,64"),
            ll: String::from("8388608,16,64"),
            cache_sim: true,
            compress_pos: false,
            compress_strings: false,
            combine_dumps: true,
            verbose: log_enabled!(log::Level::Debug),
            dump_line: true,
            dump_instr: false,
            toggle_collect: VecDeque::default(),
            callgrind_out_file: Option::default(),
            log_arg: Option::default(),
            other: Vec::default(),
        }
    }
}

impl From<Args> for tool::args::ToolArgs {
    fn from(mut value: Args) -> Self {
        let mut other = vec![
            format!("--I1={}", &value.i1),
            format!("--D1={}", &value.d1),
            format!("--LL={}", &value.ll),
            format!("--cache-sim={}", bool_to_yesno(value.cache_sim)),
            format!(
                "--compress-strings={}",
                bool_to_yesno(value.compress_strings)
            ),
            format!("--compress-pos={}", bool_to_yesno(value.compress_pos)),
            format!("--dump-line={}", bool_to_yesno(value.dump_line)),
            format!("--dump-instr={}", bool_to_yesno(value.dump_instr)),
            format!("--combine-dumps={}", bool_to_yesno(value.combine_dumps)),
        ];
        other.append(
            &mut value
                .toggle_collect
                .iter()
                .map(|s| format!("--toggle-collect={s}"))
                .collect::<Vec<String>>(),
        );
        other.append(&mut value.other);

        Self {
            tool: tool::ValgrindTool::Callgrind,
            output_paths: value
                .callgrind_out_file
                .map_or_else(Vec::new, |o| vec![o.into()]),
            log_path: value.log_arg,
            error_exitcode: "0".to_owned(),
            verbose: value.verbose,
            other,
        }
    }
}
