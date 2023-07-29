use std::collections::VecDeque;
use std::ffi::OsString;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::Command;

use colored::Colorize;
use iai_callgrind::{Options, OptionsParser};
use log::{debug, info};
use sanitize_filename::Options as SanitizerOptions;

use crate::callgrind::{CallgrindArgs, CallgrindCommand, CallgrindOutput};
use crate::{get_arch, IaiCallgrindError};

#[derive(Debug)]
struct BinBench {
    command: PathBuf,
    args: Vec<String>,
    envs: Vec<(String, String)>,
    opts: Options,
}

impl BinBench {
    fn from_env_arg(arg: &str) -> Self {
        let mut args = arg
            .strip_prefix('\'')
            .unwrap()
            .strip_suffix('\'')
            .unwrap()
            .split("','")
            .map(|s| s.to_owned())
            .collect::<VecDeque<String>>();
        Self {
            command: PathBuf::from(args.pop_front().unwrap()),
            args: args.into(),
            envs: vec![],
            opts: Options::default(),
        }
    }

    fn set_envs_from_arg(&mut self, arg: &str) {
        let args = arg
            .strip_prefix('\'')
            .unwrap()
            .strip_suffix('\'')
            .unwrap()
            .split("','")
            .filter_map(|s| match s.split_once('=') {
                Some((key, value)) => Some((key.to_owned(), value.to_owned())),
                None => match std::env::var(s) {
                    Ok(value) => Some((s.to_owned(), value)),
                    Err(_) => None,
                },
            })
            .collect::<Vec<(String, String)>>();
        self.envs = args;
    }

    fn run(&self, counter: usize, config: &Config) -> Result<(), IaiCallgrindError> {
        let command = CallgrindCommand::new(config.allow_aslr, &config.arch);

        let mut callgrind_args = config.callgrind_args.clone();
        if let Some(entry_point) = &self.opts.entry_point {
            callgrind_args.collect_atstart = false;
            callgrind_args.insert_toggle_collect(entry_point);
        } else {
            callgrind_args.collect_atstart = true;
        }

        let output = CallgrindOutput::create(
            &config.module,
            &format!("{}.{}", self.sanitized_command_string(), counter),
        );
        callgrind_args.set_output_file(&output.file.display().to_string());

        command.run(
            &callgrind_args,
            &self.command,
            self.args.clone(),
            self.envs.clone(),
            &self.opts,
        )?;

        let new_stats = output.parse_summary();

        let old_output = output.old_output();
        let old_stats = old_output.exists().then(|| old_output.parse_summary());

        println!(
            "{}{}{}",
            &config.module.green(),
            "::".green(),
            self.to_string().green()
        );
        new_stats.print(old_stats);
        Ok(())
    }

    fn sanitized_command_string(&self) -> String {
        sanitize_filename::sanitize_with_options(
            self.command.display().to_string(),
            SanitizerOptions {
                windows: true,
                truncate: true,
                replacement: "_",
            },
        )
    }
}

impl Display for BinBench {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{} {}",
            self.command.display(),
            self.args.join(" ")
        ))
    }
}

#[derive(Debug)]
enum AssistantKind {
    Setup,
    Teardown,
    Before,
    After,
}

impl AssistantKind {
    fn id(&self) -> String {
        match self {
            AssistantKind::Setup => "setup".to_owned(),
            AssistantKind::Teardown => "teardown".to_owned(),
            AssistantKind::Before => "before".to_owned(),
            AssistantKind::After => "after".to_owned(),
        }
    }
}

#[derive(Debug)]
struct Assistant {
    name: String,
    kind: AssistantKind,
}

impl Assistant {
    fn new(name: String, kind: AssistantKind) -> Self {
        Self { name, kind }
    }

    fn run(&self, bench_bin: &Path) -> Result<(), IaiCallgrindError> {
        let id = self.kind.id();
        let mut command = Command::new(bench_bin);
        command.arg("--iai-run");
        command.arg(&id);

        let (stdout, stderr) = command
            .output()
            .map_err(IaiCallgrindError::LaunchError)
            .and_then(|output| {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(output.stdout.as_slice());
                    let stderr = String::from_utf8_lossy(output.stderr.as_slice());
                    Ok((stdout.trim_end().to_string(), stderr.trim_end().to_string()))
                } else {
                    Err(IaiCallgrindError::BenchmarkLaunchError(output))
                }
            })?;

        if !stdout.is_empty() {
            info!("{} function '{}': stdout:\n{}", id, self.name, stdout);
        }
        if !stderr.is_empty() {
            info!("{} function '{}': stderr:\n{}", id, self.name, stderr);
        }

        Ok(())
    }
}

#[derive(Debug)]
struct BenchmarkAssistants {
    before: Option<Assistant>,
    after: Option<Assistant>,
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
}

impl Default for BenchmarkAssistants {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkAssistants {
    fn new() -> Self {
        Self {
            before: Default::default(),
            after: Default::default(),
            setup: Default::default(),
            teardown: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    bench_file: PathBuf,
    benches: Vec<BinBench>,
    bench_bin: PathBuf,
    bench_assists: BenchmarkAssistants,
    module: String,
    callgrind_args: CallgrindArgs,
    allow_aslr: bool,
    arch: String,
}

impl Config {
    fn from_env_args_iter(env_args_iter: impl Iterator<Item = OsString>) -> Self {
        let mut env_args_iter = env_args_iter.peekable();

        let bench_file = PathBuf::from(env_args_iter.next().unwrap());
        let module = env_args_iter.next().unwrap().to_str().unwrap().to_owned();
        let bench_bin = PathBuf::from(env_args_iter.next().unwrap());

        let mut benches = vec![];
        while let Some(arg) = env_args_iter.peek() {
            match arg.to_str().unwrap().split_once('=') {
                Some(("--run", value)) => benches.push(BinBench::from_env_arg(value)),
                Some(("--run-envs", value)) => benches.last_mut().unwrap().set_envs_from_arg(value),
                Some(("--run-opts", value)) => {
                    benches.last_mut().unwrap().opts = OptionsParser::new(Options::default())
                        .from_arg(value)
                        .unwrap();
                }
                Some(_) | None => break,
            }
            env_args_iter.next();
        }

        let mut bench_assists = BenchmarkAssistants::default();
        while let Some(arg) = env_args_iter.peek() {
            match arg.to_str().unwrap().split_once('=') {
                Some(("--setup", value)) => {
                    bench_assists.setup =
                        Some(Assistant::new(value.to_string(), AssistantKind::Setup))
                }
                Some(("--teardown", value)) => {
                    bench_assists.teardown =
                        Some(Assistant::new(value.to_string(), AssistantKind::Teardown))
                }
                Some(("--before", value)) => {
                    bench_assists.before =
                        Some(Assistant::new(value.to_string(), AssistantKind::Before))
                }
                Some(("--after", value)) => {
                    bench_assists.after =
                        Some(Assistant::new(value.to_string(), AssistantKind::After))
                }
                Some(_) | None => break,
            }
            env_args_iter.next();
        }

        let mut callgrind_args = env_args_iter.collect::<Vec<OsString>>();
        if callgrind_args.last().map_or(false, |a| a == "--bench") {
            callgrind_args.pop();
        }
        let callgrind_args = CallgrindArgs::from_args(&callgrind_args);

        let arch = get_arch();
        debug!("Detected architecture: {}", arch);

        let allow_aslr = std::env::var_os("IAI_ALLOW_ASLR").is_some();
        if allow_aslr {
            debug!("Found IAI_ALLOW_ASLR environment variable. Trying to run with ASLR enabled.");
        }

        Self {
            bench_file,
            benches,
            bench_bin,
            bench_assists,
            module,
            callgrind_args,
            allow_aslr,
            arch,
        }
    }
}

pub fn run(
    env_args_iter: impl Iterator<Item = OsString> + std::fmt::Debug,
) -> Result<(), IaiCallgrindError> {
    let config = Config::from_env_args_iter(env_args_iter);
    if let Some(before) = &config.bench_assists.before {
        before.run(&config.bench_bin)?
    }
    for (counter, bench) in config.benches.iter().enumerate() {
        if let Some(setup) = &config.bench_assists.setup {
            setup.run(&config.bench_bin)?
        }

        bench.run(counter, &config)?;

        if let Some(teardown) = &config.bench_assists.teardown {
            teardown.run(&config.bench_bin)?
        }
    }
    if let Some(after) = &config.bench_assists.after {
        after.run(&config.bench_bin)?
    }
    Ok(())
}
