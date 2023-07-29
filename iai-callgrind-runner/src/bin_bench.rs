use std::collections::VecDeque;
use std::ffi::OsString;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::Command;

use colored::Colorize;
use iai_callgrind::{Options, OptionsParser};
use log::{debug, info};
use sanitize_filename::Options as SanitizerOptions;

use crate::callgrind::{CallgrindArgs, CallgrindCommand, CallgrindOutput};
use crate::util::{write_all_to_stderr, write_all_to_stdout};
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct Assistant {
    name: String,
    kind: AssistantKind,
    bench: bool,
}

impl Assistant {
    fn new(name: String, kind: AssistantKind, bench: bool) -> Self {
        Self { name, kind, bench }
    }

    fn run_bench(&self, config: &Config) -> Result<(), IaiCallgrindError> {
        let command = CallgrindCommand::new(config.allow_aslr, &config.arch);
        let executable_args = vec![
            "--iai-run".to_owned(),
            self.kind.id(),
            format!("{}::{}", &config.module, &self.name),
        ];
        let mut callgrind_args = config.callgrind_args.clone();
        callgrind_args.collect_atstart = false;
        callgrind_args.insert_toggle_collect(&format!("*{}::{}", &config.module, &self.name));

        let output = CallgrindOutput::create(&config.module, &self.name);
        callgrind_args.set_output_file(&output.file.display().to_string());
        command.run(
            &callgrind_args,
            &config.bench_bin,
            executable_args,
            vec![],
            &Options::default().env_clear(false),
        )?;

        let new_stats = output.parse(&config.bench_file, &config.module, &self.name);

        let old_output = output.old_output();
        let old_stats = old_output
            .exists()
            .then(|| old_output.parse(&config.bench_file, &config.module, &self.name));

        println!("{}", format!("{}::{}", &config.module, &self.name).green());
        new_stats.print(old_stats);
        Ok(())
    }

    fn run_plain(&self, config: &Config) -> Result<(), IaiCallgrindError> {
        let id = self.kind.id();
        let mut command = Command::new(&config.bench_bin);
        command.arg("--iai-run");
        command.arg(&id);

        let (stdout, stderr) = command
            .output()
            .map_err(|error| IaiCallgrindError::LaunchError(config.bench_bin.clone(), error))
            .and_then(|output| {
                if output.status.success() {
                    Ok((output.stdout, output.stderr))
                } else {
                    Err(IaiCallgrindError::BenchmarkLaunchError(output))
                }
            })?;

        if !stdout.is_empty() {
            info!("{} function '{}': stdout:", id, self.name);
            write_all_to_stdout(&stdout);
        }
        if !stderr.is_empty() {
            info!("{} function '{}': stderr:", id, self.name);
            write_all_to_stderr(&stderr);
        }
        Ok(())
    }

    fn run(&mut self, config: &Config) -> Result<(), IaiCallgrindError> {
        if self.bench {
            match self.kind {
                AssistantKind::Setup | AssistantKind::Teardown => self.bench = false,
                _ => {}
            }
            self.run_bench(config)
        } else {
            self.run_plain(config)
        }
    }
}

#[derive(Debug, Clone)]
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
                    benches.last_mut().unwrap().opts =
                        OptionsParser::default().from_arg(value).unwrap();
                }
                Some(_) | None => break,
            }
            env_args_iter.next();
        }

        let mut bench_assists = BenchmarkAssistants::default();
        while let Some(arg) = env_args_iter.peek() {
            match arg.to_str().unwrap().split_once('=') {
                Some(("--setup", value)) => {
                    bench_assists.setup = Some(Assistant::new(
                        value.to_string(),
                        AssistantKind::Setup,
                        false,
                    ))
                }
                Some(("--bench-setup", value)) => {
                    bench_assists.setup = Some(Assistant::new(
                        value.to_string(),
                        AssistantKind::Setup,
                        true,
                    ))
                }
                Some(("--teardown", value)) => {
                    bench_assists.teardown = Some(Assistant::new(
                        value.to_string(),
                        AssistantKind::Teardown,
                        false,
                    ))
                }
                Some(("--bench-teardown", value)) => {
                    bench_assists.teardown = Some(Assistant::new(
                        value.to_string(),
                        AssistantKind::Teardown,
                        true,
                    ))
                }
                Some(("--before", value)) => {
                    bench_assists.before = Some(Assistant::new(
                        value.to_string(),
                        AssistantKind::Before,
                        false,
                    ))
                }
                Some(("--bench-before", value)) => {
                    bench_assists.before = Some(Assistant::new(
                        value.to_string(),
                        AssistantKind::Before,
                        true,
                    ))
                }
                Some(("--after", value)) => {
                    bench_assists.after = Some(Assistant::new(
                        value.to_string(),
                        AssistantKind::After,
                        false,
                    ))
                }
                Some(("--bench-after", value)) => {
                    bench_assists.after = Some(Assistant::new(
                        value.to_string(),
                        AssistantKind::After,
                        true,
                    ))
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
    let mut assists = config.bench_assists.clone();

    if let Some(before) = assists.before.as_mut() {
        before.run(&config)?
    }
    for (counter, bench) in config.benches.iter().enumerate() {
        if let Some(setup) = assists.setup.as_mut() {
            setup.run(&config)?
        }

        bench.run(counter, &config)?;

        if let Some(teardown) = assists.teardown.as_mut() {
            teardown.run(&config)?
        }
    }
    if let Some(after) = assists.after.as_mut() {
        after.run(&config)?
    }
    Ok(())
}
