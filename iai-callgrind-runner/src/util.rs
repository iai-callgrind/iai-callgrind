use std::ffi::{OsStr, OsString};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::Command;

use log::{log_enabled, trace, Level};
use which::which;

use crate::IaiCallgrindError;

pub fn join_os_string(slice: &[OsString], sep: &OsStr) -> OsString {
    if let Some((first, suffix)) = slice.split_first() {
        suffix.iter().fold(first.clone(), |mut a, b| {
            a.push(sep);
            a.push(b);
            a
        })
    } else {
        OsString::new()
    }
}

pub fn concat_os_string<T: AsRef<OsStr>>(mut first: OsString, second: T) -> OsString {
    first.push(second);
    first
}

pub fn bool_to_yesno(value: bool) -> String {
    if value {
        "yes".to_owned()
    } else {
        "no".to_owned()
    }
}

pub fn yesno_to_bool(value: &str) -> bool {
    value == "yes"
}

fn trim(bytes: &[u8]) -> &[u8] {
    let from = match bytes.iter().position(|x| !x.is_ascii_whitespace()) {
        Some(i) => i,
        None => return &bytes[0..0],
    };
    let to = bytes
        .iter()
        .rposition(|x| !x.is_ascii_whitespace())
        .unwrap();
    &bytes[from..=to]
}

pub fn write_all_to_stdout(bytes: &[u8]) {
    let stdout = io::stdout();
    let stdout = stdout.lock();
    let mut writer = BufWriter::new(stdout);
    writer
        .write_all(trim(bytes))
        .and_then(|_| writer.flush())
        .unwrap();
    println!();
}

pub fn write_all_to_stderr(bytes: &[u8]) {
    let stderr = io::stderr();
    let stderr = stderr.lock();
    let mut writer = BufWriter::new(stderr);
    writer
        .write_all(trim(bytes))
        .and_then(|_| writer.flush())
        .unwrap();
    println!();
}

pub fn copy_directory(
    source: &Path,
    into: &Path,
    follow_symlinks: bool,
) -> Result<(), IaiCallgrindError> {
    let cp = which("cp").map_err(|error| {
        IaiCallgrindError::Other(format!(
            "Unable to locate 'cp' command to copy directories: '{error}'"
        ))
    })?;
    let mut command = Command::new(&cp);
    if follow_symlinks {
        command.args(["-H", "--dereference"]);
    }
    command.args([
        "--verbose",
        "--recursive",
        "--preserve=mode,ownership,timestamps",
    ]);
    command.arg(source);
    command.arg(into);
    let (stdout, stderr) = command
        .output()
        .map_err(|error| IaiCallgrindError::LaunchError(cp, error))
        .and_then(|output| {
            if output.status.success() {
                Ok((output.stdout, output.stderr))
            } else {
                Err(IaiCallgrindError::BenchmarkLaunchError(output))
            }
        })?;

    if !stdout.is_empty() {
        trace!("copy fixtures: stdout:");
        if log_enabled!(Level::Trace) {
            write_all_to_stdout(&stdout);
        }
    }
    if !stderr.is_empty() {
        trace!("copy fixtures: stderr:");
        if log_enabled!(Level::Trace) {
            write_all_to_stderr(&stderr);
        }
    }
    Ok(())
}
