use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use log::{log_enabled, warn};

use crate::api::RawArgs;
use crate::error::Error;
use crate::runner::tool;
use crate::runner::tool::args::{defaults, FairSched};
use crate::util::{bool_to_yesno, yesno_to_bool};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Args {
    i1: String,
    d1: String,
    ll: String,
    cache_sim: bool,
    other: Vec<String>,
    verbose: bool,
    cachegrind_out_file: Option<PathBuf>,
    log_arg: Option<OsString>,
    trace_children: bool,
    separate_threads: bool,
    fair_sched: FairSched,
}

impl Args {
    pub fn try_from_raw_args(args: &[&RawArgs]) -> Result<Self> {
        let mut default = Self::default();
        default.try_update(args.iter().flat_map(|s| &s.0))?;
        Ok(default)
    }

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
                        Error::InvalidBoolArgument((key.to_owned(), value.to_owned()))
                    })?;
                }
                Some((key @ "--trace-children", value)) => {
                    self.trace_children = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument((key.to_owned(), value.to_owned()))
                    })?;
                }
                Some((key @ "--separate-threads", value)) => {
                    self.separate_threads = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument((key.to_owned(), value.to_owned()))
                    })?;
                }
                Some(("--fair-sched", value)) => {
                    self.fair_sched = FairSched::from_str(value)?;
                }
                Some((
                    key @ ("--cachegrind-out-file"
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
                    warn!("Ignoring cachegrind argument: '{key}={value}'");
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
                    warn!("Ignoring cachegrind argument: '{arg}'");
                }
                None if arg.starts_with('-') => self.other.push(arg.clone()),
                // ignore positional arguments for now. It might be a filtering argument for cargo
                // bench
                None => {}
            }
        }
        Ok(())
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            i1: defaults::I1.into(),
            d1: defaults::D1.into(),
            ll: defaults::LL.into(),
            cache_sim: defaults::CACHE_SIM,
            verbose: log_enabled!(log::Level::Debug),
            cachegrind_out_file: Option::default(),
            log_arg: Option::default(),
            other: Vec::default(),
            trace_children: defaults::TRACE_CHILDREN,
            separate_threads: defaults::SEPARATE_THREADS,
            fair_sched: defaults::FAIR_SCHED,
        }
    }
}

// TODO: Move this into tool::args::ToolArgs
impl From<Args> for tool::args::ToolArgs {
    fn from(mut value: Args) -> Self {
        let mut other = vec![
            format!("--I1={}", &value.i1),
            format!("--D1={}", &value.d1),
            format!("--LL={}", &value.ll),
            format!("--cache-sim={}", bool_to_yesno(value.cache_sim)),
            format!(
                "--separate-threads={}",
                bool_to_yesno(value.separate_threads)
            ),
        ];
        other.append(&mut value.other);

        Self {
            tool: tool::ValgrindTool::Cachegrind,
            output_paths: value
                .cachegrind_out_file
                .map_or_else(Vec::new, |o| vec![o.into()]),
            log_path: value.log_arg,
            error_exitcode: defaults::ERROR_EXIT_CODE_OTHER_TOOL.into(),
            verbose: value.verbose,
            trace_children: value.trace_children,
            fair_sched: value.fair_sched,
            other,
        }
    }
}
