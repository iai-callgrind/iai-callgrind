//! The module containing the command line arguments for cachegrind

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
    d1: String,
    fair_sched: FairSched,
    i1: String,
    ll: String,
    other: Vec<String>,
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
            let arg = arg.trim();
            match arg.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                Some(("--I1", value)) => value.clone_into(&mut self.i1),
                Some(("--D1", value)) => value.clone_into(&mut self.d1),
                Some(("--LL", value)) => value.clone_into(&mut self.ll),
                Some((key @ "--cache-sim", value)) => {
                    self.cache_sim = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument(key.to_owned(), value.to_owned())
                    })?;
                }
                Some((key @ "--trace-children", value)) => {
                    self.trace_children = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidBoolArgument(key.to_owned(), value.to_owned())
                    })?;
                }
                Some(("--fair-sched", value)) => {
                    self.fair_sched = FairSched::from_str(value)?;
                }
                Some((arg, _)) if is_ignored_outfile_argument(arg) => warn!(
                    "Ignoring cachegrind argument '{arg}': Output/Log files of tools are managed \
                     by Iai-Callgrind",
                ),
                None if matches!(arg, "-v" | "--verbose") => self.verbose = true,
                None if is_ignored_argument(arg) => {
                    warn!("Ignoring cachegrind argument: '{arg}'");
                }
                None | Some(_) => self.other.push(arg.to_owned()),
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
            other: Vec::default(),
            trace_children: defaults::TRACE_CHILDREN,
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
        ];
        other.append(&mut value.other);

        Self {
            tool: ValgrindTool::Cachegrind,
            output_paths: Vec::default(),
            log_path: Option::default(),
            xtree_path: Option::default(),
            xleak_path: Option::default(),
            error_exitcode: defaults::ERROR_EXIT_CODE_OTHER_TOOL.into(),
            verbose: value.verbose,
            trace_children: value.trace_children,
            fair_sched: value.fair_sched,
            other,
        }
    }
}
