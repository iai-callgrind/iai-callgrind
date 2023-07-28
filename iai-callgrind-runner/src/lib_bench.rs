use colored::Colorize;
use std::{ffi::OsString, path::PathBuf, process::Stdio};

use log::{debug, info};

use crate::{
    basic_valgrind, get_arch, parse_callgrind_output, valgrind_without_aslr, CallgrindArgs,
    CallgrindStats, IaiCallgrindError,
};

#[derive(Debug)]
pub struct Config {
    bench_file: PathBuf,
    benches: Vec<String>,
    executable: PathBuf,
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
        let executable = PathBuf::from(env_args_iter.next().unwrap());

        let mut benches = vec![];
        while let Some(arg) = env_args_iter.peek() {
            match arg.to_str().unwrap().split_once('=') {
                Some((key, value)) if key == "--iai-bench" || key == "--run" => {
                    benches.push(value.to_string())
                }
                Some(_) | None => break,
            }
            env_args_iter.next();
        }

        let mut callgrind_args = env_args_iter
            .filter(|a| a.to_str().unwrap().starts_with("--"))
            .map(|s| s.to_str().unwrap().to_owned())
            .collect::<Vec<String>>();
        if callgrind_args.last().map_or(false, |a| a == "--bench") {
            callgrind_args.pop();
        }
        let callgrind_args = CallgrindArgs::from_args(callgrind_args);

        let arch = get_arch();
        debug!("Detected architecture: {}", arch);

        let allow_aslr = std::env::var_os("IAI_ALLOW_ASLR").is_some();
        if allow_aslr {
            debug!("Found IAI_ALLOW_ASLR environment variable. Trying to run with ASLR enabled.");
        }

        Self {
            bench_file,
            benches,
            executable,
            module,
            callgrind_args,
            allow_aslr,
            arch,
        }
    }
}

#[inline(never)]
fn run_bench(
    index: usize,
    function_name: &str,
    config: &Config,
) -> Result<(CallgrindStats, Option<CallgrindStats>), IaiCallgrindError> {
    let mut cmd = if config.allow_aslr {
        debug!("Running with ASLR enabled");
        basic_valgrind()
    } else {
        match valgrind_without_aslr(config.arch.as_str()) {
            Some(cmd) => {
                debug!("Running with ASLR disabled");
                cmd
            }
            None => {
                debug!("Running with ASLR enabled");
                basic_valgrind()
            }
        }
    };

    let target = PathBuf::from("target/iai");
    let module_path: PathBuf = config.module.split("::").collect();
    let file_name = PathBuf::from(format!("callgrind.{}.out", function_name));

    let mut output_file = target;
    output_file.push(module_path);
    output_file.push(file_name);

    let old_file = output_file.with_extension("out.old");

    std::fs::create_dir_all(output_file.parent().unwrap()).expect("Failed to create directory");

    if output_file.exists() {
        // Already run this benchmark once; move last results to .old
        std::fs::copy(&output_file, &old_file).unwrap();
    }

    let callgrind_args =
        config
            .callgrind_args
            .parse_with(&output_file, config.module.as_str(), function_name);
    debug!("Callgrind arguments: {}", callgrind_args.join(" "));
    let output = cmd
        .arg("--tool=callgrind")
        .args(callgrind_args)
        .arg(&config.executable)
        .arg("--iai-run")
        .arg(index.to_string())
        // Currently not used in iai-callgrind itself, but in `callgrind_annotate` this name is
        // shown and makes it easier to identify the benchmark under test
        .arg(format!("{}::{}", config.module, function_name))
        // valgrind doesn't output anything on stdout
        // .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(IaiCallgrindError::LaunchError)
        .and_then(|output| {
            if output.status.success() {
                let stderr = String::from_utf8_lossy(output.stderr.as_slice());
                Ok(stderr.trim_end().to_string())
            } else {
                Err(IaiCallgrindError::CallgrindLaunchError(output))
            }
        })?;

    if !output.is_empty() {
        info!("Callgrind output:\n{}", output);
    }

    let new_stats = parse_callgrind_output(
        &output_file,
        &config.bench_file,
        &config.module,
        function_name,
    );
    let old_stats = old_file.exists().then(|| {
        parse_callgrind_output(&old_file, &config.bench_file, &config.module, function_name)
    });

    Ok((new_stats, old_stats))
}

pub fn run(env_args: impl Iterator<Item = OsString>) -> Result<(), IaiCallgrindError> {
    let config = Config::from_env_args_iter(env_args);
    for (index, name) in config.benches.iter().enumerate() {
        let (stats, old_stats) = run_bench(index, name, &config)?;

        println!("{}", format!("{}::{}", config.module, name).green());
        stats.print(old_stats);
    }

    Ok(())
}
