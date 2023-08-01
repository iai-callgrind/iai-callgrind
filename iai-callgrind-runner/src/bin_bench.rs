use std::collections::VecDeque;
use std::ffi::OsString;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::Command;

use colored::Colorize;
use iai_callgrind::{Options, OptionsParser};
use log::{debug, info, log_enabled, trace, Level};
use sanitize_filename::Options as SanitizerOptions;
use tempfile::TempDir;

use crate::callgrind::{CallgrindArgs, CallgrindCommand, CallgrindOutput};
use crate::util::{copy_directory, write_all_to_stderr, write_all_to_stdout};
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
            .map(std::borrow::ToOwned::to_owned)
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
            &config.package_dir,
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

        let output = CallgrindOutput::create(&config.package_dir, &config.module, &self.name);
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
            if log_enabled!(Level::Info) {
                write_all_to_stdout(&stdout);
            }
        }
        if !stderr.is_empty() {
            info!("{} function '{}': stderr:", id, self.name);
            if log_enabled!(Level::Info) {
                write_all_to_stderr(&stderr);
            }
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
            before: Option::default(),
            after: Option::default(),
            setup: Option::default(),
            teardown: Option::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct Fixtures {
    path: PathBuf,
    follow_symlinks: bool,
}

impl Fixtures {
    fn new(path: PathBuf, follow_symlinks: bool) -> Self {
        Self {
            path,
            follow_symlinks,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Config {
    package_dir: PathBuf,
    bench_file: PathBuf,
    module: String,
    bench_bin: PathBuf,
    sandbox: bool,
    fixtures: Option<Fixtures>,
    benches: Vec<BinBench>,
    bench_assists: BenchmarkAssistants,
    callgrind_args: CallgrindArgs,
    allow_aslr: bool,
    arch: String,
}

impl Config {
    #[allow(clippy::too_many_lines)]
    fn from_env_args_iter(env_args_iter: impl Iterator<Item = OsString> + std::fmt::Debug) -> Self {
        let mut env_args_iter = env_args_iter.peekable();

        let package_dir = PathBuf::from(env_args_iter.next().unwrap());
        let bench_file = PathBuf::from(env_args_iter.next().unwrap());
        let module = env_args_iter.next().unwrap().to_str().unwrap().to_owned();
        let bench_bin = PathBuf::from(env_args_iter.next().unwrap());

        let mut sandbox = true;
        if let Some(arg) = env_args_iter.peek() {
            if let Some(("--sandbox", value)) = arg.to_str().unwrap().split_once('=') {
                sandbox = value
                    .strip_prefix('\'')
                    .unwrap()
                    .strip_suffix('\'')
                    .unwrap()
                    .parse::<bool>()
                    .unwrap();
                env_args_iter.next().unwrap();
            }
        }

        let mut fixtures = None;
        if let Some(arg) = env_args_iter.peek() {
            if let Some(("--fixtures", value)) = arg.to_str().unwrap().split_once('=') {
                let args = value
                    .strip_prefix('\'')
                    .unwrap()
                    .strip_suffix('\'')
                    .unwrap()
                    .split("','")
                    .map(std::borrow::ToOwned::to_owned)
                    .collect::<VecDeque<String>>();
                let follow_symlinks = args.get(1).map_or(false, |s| match s.split_once('=') {
                    Some(("follow_symlinks", key)) => key.parse().unwrap(),
                    Some(_) | None => false,
                });
                fixtures = Some(Fixtures::new(PathBuf::from(&args[0]), follow_symlinks));
                env_args_iter.next().unwrap();
            }
        }

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
                        value.to_owned(),
                        AssistantKind::Setup,
                        false,
                    ));
                }
                Some(("--bench-setup", value)) => {
                    bench_assists.setup =
                        Some(Assistant::new(value.to_owned(), AssistantKind::Setup, true));
                }
                Some(("--teardown", value)) => {
                    bench_assists.teardown = Some(Assistant::new(
                        value.to_owned(),
                        AssistantKind::Teardown,
                        false,
                    ));
                }
                Some(("--bench-teardown", value)) => {
                    bench_assists.teardown = Some(Assistant::new(
                        value.to_owned(),
                        AssistantKind::Teardown,
                        true,
                    ));
                }
                Some(("--before", value)) => {
                    bench_assists.before = Some(Assistant::new(
                        value.to_owned(),
                        AssistantKind::Before,
                        false,
                    ));
                }
                Some(("--bench-before", value)) => {
                    bench_assists.before = Some(Assistant::new(
                        value.to_owned(),
                        AssistantKind::Before,
                        true,
                    ));
                }
                Some(("--after", value)) => {
                    bench_assists.after = Some(Assistant::new(
                        value.to_owned(),
                        AssistantKind::After,
                        false,
                    ));
                }
                Some(("--bench-after", value)) => {
                    bench_assists.after =
                        Some(Assistant::new(value.to_owned(), AssistantKind::After, true));
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
            package_dir,
            bench_file,
            module,
            bench_bin,
            sandbox,
            fixtures,
            benches,
            bench_assists,
            callgrind_args,
            allow_aslr,
            arch,
        }
    }
}

fn setup_sandbox(config: &Config) -> Result<TempDir, IaiCallgrindError> {
    debug!("Creating temporary workspace directory");
    let temp_dir = tempfile::tempdir().expect("Create temporary directory");
    if let Some(fixtures) = &config.fixtures {
        debug!(
            "Copying fixtures from '{}' to '{}'",
            &fixtures.path.display(),
            temp_dir.path().display()
        );
        copy_directory(&fixtures.path, temp_dir.path(), fixtures.follow_symlinks)?;
    }
    trace!(
        "Changing current directory to temporary directory: '{}'",
        temp_dir.path().display()
    );
    std::env::set_current_dir(temp_dir.path())
        .expect("Set current directory to temporary workspace directory");
    Ok(temp_dir)
}

pub(crate) fn run(
    env_args_iter: impl Iterator<Item = OsString> + std::fmt::Debug,
) -> Result<(), IaiCallgrindError> {
    let config = Config::from_env_args_iter(env_args_iter);

    // We need the temp_dir to exist within this function or else it's getting dropped and deleted
    // too early.
    let temp_dir = if config.sandbox {
        debug!("Setting up sandbox");
        Some(setup_sandbox(&config)?)
    } else {
        debug!(
            "Sandbox switched off: Running benchmarks in the current directory: '{}'",
            std::env::current_dir().unwrap().display()
        );
        None
    };

    let mut assists = config.bench_assists.clone();

    if let Some(before) = assists.before.as_mut() {
        before.run(&config)?;
    }
    for (counter, bench) in config.benches.iter().enumerate() {
        if let Some(setup) = assists.setup.as_mut() {
            setup.run(&config)?;
        }

        bench.run(counter, &config)?;

        if let Some(teardown) = assists.teardown.as_mut() {
            teardown.run(&config)?;
        }
    }
    if let Some(after) = assists.after.as_mut() {
        after.run(&config)?;
    }

    // Drop temp_dir and it's getting deleted
    drop(temp_dir);
    Ok(())
}
