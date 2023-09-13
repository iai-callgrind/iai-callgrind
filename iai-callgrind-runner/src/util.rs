use std::ffi::{OsStr, OsString};
use std::io::{self, stdin, BufWriter, Read, Write};
use std::path::Path;
use std::process::Command;

use log::{log_enabled, trace, Level};
use which::which;

use crate::error::{IaiCallgrindError, Result};

pub fn receive_benchmark<T>(num_bytes: usize) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let mut encoded = vec![];
    let mut stdin = stdin();
    stdin.read_to_end(&mut encoded).map_err(|error| {
        IaiCallgrindError::Other(format!("Failed to read encoded configuration: {error}"))
    })?;
    assert!(
        encoded.len() == num_bytes,
        "Bytes mismatch when decoding configuration: Expected {num_bytes} bytes but received: {} \
         bytes",
        encoded.len()
    );

    let benchmark: T = bincode::deserialize(&encoded).map_err(|error| {
        IaiCallgrindError::Other(format!("Failed to decode configuration: {error}"))
    })?;

    Ok(benchmark)
}

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

pub fn concat_os_string<T, U>(first: T, second: U) -> OsString
where
    T: Into<OsString>,
    U: AsRef<OsStr>,
{
    let mut first = first.into();
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

pub fn yesno_to_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

pub fn truncate_str_utf8(string: &str, len: usize) -> &str {
    if let Some((pos, c)) = string
        .char_indices()
        .take_while(|(i, c)| i + c.len_utf8() <= len)
        .last()
    {
        &string[..pos + c.len_utf8()]
    } else {
        &string[..0]
    }
}

pub fn trim(bytes: &[u8]) -> &[u8] {
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
    if !bytes.is_empty() {
        let stdout = io::stdout();
        let stdout = stdout.lock();
        let mut writer = BufWriter::new(stdout);
        writer
            .write_all(bytes)
            .and_then(|_| writer.flush())
            .unwrap();
        if !bytes.last().map_or(false, |l| *l == b'\n') {
            println!();
        }
    }
}

pub fn write_all_to_stderr(bytes: &[u8]) {
    if !bytes.is_empty() {
        let stderr = io::stderr();
        let stderr = stderr.lock();
        let mut writer = BufWriter::new(stderr);
        writer
            .write_all(bytes)
            .and_then(|_| writer.flush())
            .unwrap();
        if !bytes.last().map_or(false, |l| *l == b'\n') {
            eprintln!();
        }
    }
}

pub fn copy_directory(source: &Path, into: &Path, follow_symlinks: bool) -> Result<()> {
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
            write_all_to_stderr(&stdout);
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::empty_0("", 0, "")]
    #[case::empty_1("", 1, "")]
    #[case::single_0("a", 0, "")]
    #[case::single_1("a", 1, "a")]
    #[case::single_2("a", 2, "a")]
    #[case::two_0("ab", 0, "")]
    #[case::two_1("ab", 1, "a")]
    #[case::two_2("ab", 2, "ab")]
    #[case::two_3("ab", 3, "ab")]
    #[case::two_usize_max("ab", usize::MAX, "ab")]
    #[case::hundred_0(&"a".repeat(100), 0, "")]
    #[case::hundred_99(&"ab".repeat(50), 99, &"ab".repeat(50)[..99])]
    #[case::hundred_100(&"a".repeat(100), 100, &"a".repeat(100))]
    #[case::hundred_255(&"a".repeat(100), 255, &"a".repeat(100))]
    #[case::multi_byte_0("µ", 0, "")]
    #[case::multi_byte_1("µ", 1, "")]
    #[case::multi_byte_2("µ", 2, "µ")]
    #[case::multi_byte_3("µ", 3, "µ")]
    #[case::uni_then_multi_byte_0("aµ", 0, "")]
    #[case::uni_then_multi_byte_1("aµ", 1, "a")]
    #[case::uni_then_multi_byte_2("aµ", 2, "a")]
    #[case::uni_then_multi_byte_3("aµ", 3, "aµ")]
    #[case::uni_then_multi_byte_4("aµ", 4, "aµ")]
    #[case::multi_byte_then_uni_0("µa", 0, "")]
    #[case::multi_byte_then_uni_1("µa", 1, "")]
    #[case::multi_byte_then_uni_2("µa", 2, "µ")]
    #[case::multi_byte_then_uni_3("µa", 3, "µa")]
    #[case::multi_byte_then_uni_4("µa", 4, "µa")]
    fn test_truncate_str(#[case] input: &str, #[case] len: usize, #[case] expected: &str) {
        assert_eq!(truncate_str_utf8(input, len), expected);
    }
}
