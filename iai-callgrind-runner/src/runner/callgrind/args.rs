//! The module containing the command line arguments for callgrind
use std::collections::VecDeque;
use std::str::FromStr;

use anyhow::Result;
use log::{log_enabled, warn};

use crate::api::{RawArgs, ValgrindTool};
use crate::error::Error;
use crate::runner::tool::args::{
    defaults, is_ignored_argument, is_ignored_outfile_argument, FairSched, ToolArgs,
};
use crate::util::{bool_to_yesno, yesno_to_bool};

/// The command-line arguments
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Args {
    cache_sim: bool,
    /// --combine-dumps is currently not supported by the callgrind parsers, so we print a warning
    combine_dumps: bool,
    compress_pos: bool,
    compress_strings: bool,
    d1: String,
    dump_instr: bool,
    dump_line: bool,
    fair_sched: FairSched,
    i1: String,
    ll: String,
    other: Vec<String>,
    separate_threads: bool,
    toggle_collect: VecDeque<String>,
    trace_children: bool,
    verbose: bool,
}

impl Args {
    /// Try to create new `Args` from multiple [`RawArgs`]
    pub fn try_from_raw_args(args: &[&RawArgs]) -> Result<Self> {
        let mut default = Self::default();
        default.try_update(args.iter().flat_map(|s| &s.0))?;
        Ok(default)
    }

    /// Try to update these `Args` from the contents of an iterator
    pub fn try_update<'a, T: Iterator<Item = &'a String>>(&mut self, args: T) -> Result<()> {
        for arg in args {
            match arg
                .trim()
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
            {
                Some(("--I1", value)) => value.clone_into(&mut self.i1),
                Some(("--D1", value)) => value.clone_into(&mut self.d1),
                Some(("--LL", value)) => value.clone_into(&mut self.ll),
                Some((key @ "--cache-sim", value)) => {
                    self.cache_sim = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument(key.to_owned(), value.to_owned())
                    })?;
                }
                Some(("--toggle-collect", value)) => {
                    self.toggle_collect.push_back(value.to_owned());
                }
                Some((key @ "--dump-instr", value)) => {
                    self.dump_instr = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument(key.to_owned(), value.to_owned())
                    })?;
                }
                Some((key @ "--dump-line", value)) => {
                    self.dump_line = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument(key.to_owned(), value.to_owned())
                    })?;
                }
                Some((key @ "--trace-children", value)) => {
                    self.trace_children = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument(key.to_owned(), value.to_owned())
                    })?;
                }
                Some((key @ "--separate-threads", value)) => {
                    self.separate_threads = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument(key.to_owned(), value.to_owned())
                    })?;
                }
                Some(("--fair-sched", value)) => {
                    self.fair_sched = FairSched::from_str(value)?;
                }
                Some((
                    key @ ("--combine-dumps" | "--compress-strings" | "--compress-pos"),
                    value,
                )) => {
                    warn!("Ignoring unsupported callgrind argument: '{key}={value}'");
                }
                Some((arg, _)) if is_ignored_outfile_argument(arg) => warn!(
                    "Ignoring callgrind argument '{arg}': Output/Log files of tools are managed \
                     by Iai-Callgrind",
                ),
                Some(_) => self.other.push(arg.clone()),
                None if arg == "-v" || arg == "--verbose" => self.verbose = true,
                None if is_ignored_argument(arg) => {
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

    /// Insert the --toggle-collect argument at the start
    ///
    /// This is pure cosmetics, since callgrind doesn't prioritize the toggles by any order
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
            i1: defaults::I1.into(),
            d1: defaults::D1.into(),
            ll: defaults::LL.into(),
            cache_sim: defaults::CACHE_SIM,
            compress_pos: defaults::COMPRESS_POS,
            compress_strings: defaults::COMPRESS_STRINGS,
            combine_dumps: defaults::COMBINE_DUMPS,
            verbose: log_enabled!(log::Level::Debug),
            dump_line: defaults::DUMP_LINE,
            dump_instr: defaults::DUMP_INSTR,
            toggle_collect: VecDeque::default(),
            other: Vec::default(),
            trace_children: defaults::TRACE_CHILDREN,
            separate_threads: defaults::SEPARATE_THREADS,
            fair_sched: defaults::FAIR_SCHED,
        }
    }
}

impl From<Args> for ToolArgs {
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
            format!(
                "--separate-threads={}",
                bool_to_yesno(value.separate_threads)
            ),
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
            tool: ValgrindTool::Callgrind,
            output_paths: Vec::default(),
            log_path: Option::default(),
            error_exitcode: defaults::ERROR_EXIT_CODE_OTHER_TOOL.into(),
            verbose: value.verbose,
            trace_children: value.trace_children,
            fair_sched: value.fair_sched,
            other,
        }
    }
}
