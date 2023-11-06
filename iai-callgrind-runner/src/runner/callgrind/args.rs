use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use anyhow::Result;
use log::{log_enabled, warn};

use crate::api::RawArgs;
use crate::error::Error;
use crate::util::{bool_to_yesno, yesno_to_bool};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Args {
    i1: String,
    d1: String,
    ll: String,
    cache_sim: bool,
    pub(crate) collect_atstart: bool,
    other: Vec<String>,
    toggle_collect: VecDeque<String>,
    compress_strings: bool,
    compress_pos: bool,
    pub(crate) verbose: bool,
    dump_instr: bool,
    dump_line: bool,
    combine_dumps: bool,
    callgrind_out_file: Option<PathBuf>,
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
                Some(("--I1", value)) => self.i1 = value.to_owned(),
                Some(("--D1", value)) => self.d1 = value.to_owned(),
                Some(("--LL", value)) => self.ll = value.to_owned(),
                Some((key @ "--collect-atstart", value)) => {
                    self.collect_atstart = yesno_to_bool(value).ok_or_else(|| {
                        Error::InvalidCallgrindBoolArgument((key.to_owned(), value.to_owned()))
                    })?;
                }
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
                    | "--cache-sim"
                    | "--callgrind-out-file"
                    | "--compress-strings"
                    | "--compress-pos"
                    | "--combine-dumps"),
                    value,
                )) => {
                    warn!("Ignoring callgrind argument: '{}={}'", key, value);
                }
                Some(_) => self.other.push(arg.clone()),
                None if arg == "-v" || arg == "--verbose" => self.verbose = true,
                None if arg == "-q" || arg == "--quiet" => {
                    self.verbose = false;
                    self.other.push(arg.clone());
                }
                None if matches!(
                    arg.trim(),
                    "-h" | "--help" | "--help-dyn-options" | "--help-debug" | "--version"
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

    pub fn insert_toggle_collect(&mut self, arg: &str) {
        self.toggle_collect.push_front(arg.to_owned());
    }

    pub fn set_output_file<T>(&mut self, arg: T)
    where
        T: AsRef<Path>,
    {
        self.callgrind_out_file = Some(arg.as_ref().to_owned());
    }

    pub fn to_vec(&self) -> Vec<String> {
        let mut args = vec![
            format!("--I1={}", &self.i1),
            format!("--D1={}", &self.d1),
            format!("--LL={}", &self.ll),
            format!("--cache-sim={}", bool_to_yesno(self.cache_sim)),
            format!("--collect-atstart={}", bool_to_yesno(self.collect_atstart)),
            format!(
                "--compress-strings={}",
                bool_to_yesno(self.compress_strings)
            ),
            format!("--compress-pos={}", bool_to_yesno(self.compress_pos)),
            format!("--dump-line={}", bool_to_yesno(self.dump_line)),
            format!("--dump-instr={}", bool_to_yesno(self.dump_instr)),
            format!("--combine-dumps={}", bool_to_yesno(self.combine_dumps)),
        ];

        if self.verbose {
            args.push(String::from("--verbose"));
        }

        args.append(
            &mut self
                .toggle_collect
                .iter()
                .map(|s| format!("--toggle-collect={s}"))
                .collect::<Vec<String>>(),
        );

        if let Some(output_file) = &self.callgrind_out_file {
            args.push(format!(
                "--callgrind-out-file={}",
                output_file.to_string_lossy(),
            ));
        }

        args.extend_from_slice(self.other.as_slice());
        args
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
            collect_atstart: false,
            compress_pos: false,
            compress_strings: false,
            combine_dumps: true,
            verbose: log_enabled!(log::Level::Debug),
            dump_line: true,
            dump_instr: false,
            toggle_collect: VecDeque::default(),
            callgrind_out_file: Option::default(),
            other: Vec::default(),
        }
    }
}
