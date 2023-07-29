use std::ffi::OsString;
use std::path::PathBuf;

use colored::Colorize;
use iai_callgrind::Options;
use log::debug;

use crate::callgrind::{CallgrindArgs, CallgrindCommand, CallgrindOutput};
use crate::{get_arch, IaiCallgrindError};

#[derive(Debug)]
struct Config {
    bench_file: PathBuf,
    benches: Vec<String>,
    executable: PathBuf,
    module: String,
    callgrind_args: CallgrindArgs,
    allow_aslr: bool,
    arch: String,
}

impl Config {
    fn with_env_args_iter(env_args_iter: impl Iterator<Item = OsString>) -> Self {
        let mut env_args_iter = env_args_iter.peekable();

        let bench_file = PathBuf::from(env_args_iter.next().unwrap());
        let module = env_args_iter.next().unwrap().to_str().unwrap().to_owned();
        let executable = PathBuf::from(env_args_iter.next().unwrap());

        let mut benches = vec![];
        while let Some(arg) = env_args_iter.peek() {
            match arg.to_str().unwrap().split_once('=') {
                Some((key, value)) if key == "--iai-bench" => benches.push(value.to_string()),
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
            executable,
            module,
            callgrind_args,
            allow_aslr,
            arch,
        }
    }
}

pub fn run(env_args: impl Iterator<Item = OsString>) -> Result<(), IaiCallgrindError> {
    let config = Config::with_env_args_iter(env_args);
    for (index, function_name) in config.benches.iter().enumerate() {
        let command = CallgrindCommand::new(config.allow_aslr, &config.arch);
        let args = vec![
            "--iai-run".to_owned(),
            index.to_string(),
            format!("{}::{}", config.module, function_name),
        ];
        let mut callgrind_args = config.callgrind_args.clone();
        callgrind_args.insert_toggle_collect(&format!("*{}::{}", &config.module, function_name));

        let output = CallgrindOutput::create(&config.module, function_name);
        callgrind_args.set_output_file(&output.file.display().to_string());
        command.run(
            &callgrind_args,
            &config.executable,
            args,
            vec![],
            &Options::default().env_clear(false),
        )?;

        let new_stats = output.parse(&config.bench_file, &config.module, function_name);

        let old_output = output.old_output();
        let old_stats = old_output
            .exists()
            .then(|| old_output.parse(&config.bench_file, &config.module, function_name));

        println!(
            "{}",
            format!("{}::{}", config.module, function_name).green()
        );
        new_stats.print(old_stats);
    }

    Ok(())
}
