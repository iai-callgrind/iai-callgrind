//! This module provides common utility functions
use std::ffi::OsStr;
use std::io::{self, BufWriter, Write};
use std::ops::Neg;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};
use derive_more::AsRef;
use log::{debug, log_enabled, trace, Level};
use which::which;

use crate::error::Error;

#[derive(Debug)]
pub enum EitherOrBoth<T> {
    Left(T),
    Right(T),
    Both((T, T)),
}

/// A vector with at least one element
#[derive(Debug, PartialEq, Eq, Clone, AsRef)]
pub struct Vec1<T>(Vec<T>);

impl<T> Vec1<T> {
    pub fn try_from_vec(inner: Vec<T>) -> Result<Self> {
        if inner.is_empty() {
            Err(anyhow!("The inner vector should have at least one element"))
        } else {
            Ok(Self(inner))
        }
    }

    pub fn from_vec(inner: Vec<T>) -> Self {
        Self(inner)
    }
}

/// Convert a boolean value to a `yes` or `no` string
pub fn bool_to_yesno(value: bool) -> String {
    if value {
        "yes".to_owned()
    } else {
        "no".to_owned()
    }
}

/// Convert a `yes` or `no` string to a boolean value
///
/// This method is the counterpart to [`bool_to_yesno`] and can fail if the string doesn't match
/// exactly (case sensitive).
pub fn yesno_to_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

/// Truncate a utf-8 [`std::str`] to a given `len`
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

/// Trim a slice of `u8` from ascii whitespace
pub fn trim(bytes: &[u8]) -> &[u8] {
    let Some(from) = bytes.iter().position(|x| !x.is_ascii_whitespace()) else {
        return &bytes[0..0];
    };
    let to = bytes
        .iter()
        .rposition(|x| !x.is_ascii_whitespace())
        .unwrap();
    &bytes[from..=to]
}

/// Dump all data to `stdout`
pub fn write_all_to_stdout(bytes: &[u8]) {
    if !bytes.is_empty() {
        let stdout = io::stdout();
        let stdout = stdout.lock();
        let mut writer = BufWriter::new(stdout);
        writer
            .write_all(bytes)
            .and_then(|()| writer.flush())
            .unwrap();
        if !bytes.last().map_or(false, |l| *l == b'\n') {
            println!();
        }
    }
}

/// Dump all data to `stderr`
pub fn write_all_to_stderr(bytes: &[u8]) {
    if !bytes.is_empty() {
        let stderr = io::stderr();
        let stderr = stderr.lock();
        let mut writer = BufWriter::new(stderr);
        writer
            .write_all(bytes)
            .and_then(|()| writer.flush())
            .unwrap();
        if !bytes.last().map_or(false, |l| *l == b'\n') {
            eprintln!();
        }
    }
}

/// Copy a directory recursively from `source` to `dest` preserving mode, ownership and timestamps
///
/// If `follow_symlinks` is true copy the symlinked file or directory instead of the symlink itself
pub fn copy_directory(source: &Path, dest: &Path, follow_symlinks: bool) -> Result<()> {
    let cp = resolve_binary_path("cp")?;
    let mut command = Command::new(&cp);

    // Using short options ensures compatibility with FreeBSD and Linux
    if follow_symlinks {
        // -H: Follow command-line symbolic links
        // -L: always follow symbolic links in SOURCE
        command.args(["-H", "-L"]);
    }

    // -v: Verbose
    // -R: Recursive
    // -p: preserve timestamps, file mode, ownership
    command.args(["-v", "-R", "-p"]);
    command.arg(source);
    command.arg(dest);
    let (stdout, stderr) = command
        .output()
        .map_err(|error| Error::LaunchError(cp.clone(), error.to_string()))
        .and_then(|output| {
            if output.status.success() {
                Ok((output.stdout, output.stderr))
            } else {
                let status = output.status;
                Err(Error::ProcessError((
                    cp.to_string_lossy().to_string(),
                    Some(output),
                    status,
                    None,
                )))
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

/// Try to resolve the absolute path of a binary from the `PATH` and relative paths
///
/// If the binary is a name without path separators the PATH is tried, otherwise if not absolute
/// a relative path is tried. If the path is already absolute checks if it is executable.
pub fn resolve_binary_path<T>(binary: T) -> Result<PathBuf>
where
    T: AsRef<OsStr>,
{
    let binary = binary.as_ref();
    match which(binary) {
        Ok(path) => {
            debug!("Found '{}': '{}'", binary.to_string_lossy(), path.display());
            Ok(path)
        }
        Err(error) => Err(
            anyhow! {"{error}: '{0}' could not be found. Is '{0}' installed, executable and in the PATH?",
                binary.to_string_lossy()
            },
        ),
    }
}

/// Format a float as string depending on the number of digits of the integer-part
///
/// The higher the number of integer-part digits the lower the number of fractional-part digits.
/// This procedure accounts for the fractional-part being less significant the higher the value of
/// the floating point number gets.
pub fn to_string_signed_short(n: f64) -> String {
    let n_abs = n.abs();

    if n_abs < 10.0f64 {
        format!("{n:+.5}")
    } else if n_abs < 100.0f64 {
        format!("{n:+.4}")
    } else if n_abs < 1000.0f64 {
        format!("{n:+.3}")
    } else if n_abs < 10000.0f64 {
        format!("{n:+.2}")
    } else if n_abs < 100_000.0_f64 {
        format!("{n:+.1}")
    } else {
        format!("{n:+.0}")
    }
}

/// Calculate the difference between `new` and `old` as percentage
pub fn percentage_diff(new: u64, old: u64) -> f64 {
    if new == old {
        return 0f64;
    }

    #[allow(clippy::cast_precision_loss)]
    let new = new as f64;
    #[allow(clippy::cast_precision_loss)]
    let old = old as f64;

    let diff = (new - old) / old;
    diff * 100.0f64
}

/// Calculate the difference between `new` and `old` as factor
pub fn factor_diff(new: u64, old: u64) -> f64 {
    if new == old {
        return 1f64;
    }

    #[allow(clippy::cast_precision_loss)]
    let new_float = new as f64;
    #[allow(clippy::cast_precision_loss)]
    let old_float = old as f64;

    if new > old {
        if old == 0 {
            f64::INFINITY
        } else {
            new_float / old_float
        }
    } else if new == 0 {
        f64::NEG_INFINITY
    } else {
        (old_float / new_float).neg()
    }
}

/// Make a `path` relative to the `base_dir`
pub fn make_relative<B, T>(base_dir: B, path: T) -> PathBuf
where
    B: AsRef<Path>,
    T: AsRef<Path>,
{
    let (base_dir, path) = (base_dir.as_ref(), path.as_ref());
    path.strip_prefix(base_dir).unwrap_or(path).to_owned()
}

/// Make a `path` absolute with the `base_dir` as prefix
pub fn make_absolute<B, T>(base_dir: B, path: T) -> PathBuf
where
    B: AsRef<Path>,
    T: AsRef<Path>,
{
    let (base_dir, path) = (base_dir.as_ref(), path.as_ref());
    if path.strip_prefix(base_dir).is_ok() {
        path.to_owned()
    } else {
        base_dir.join(path)
    }
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

    #[rstest]
    #[case::zero(0, 0, 1f64)]
    #[case::infinity(1, 0, f64::INFINITY)]
    #[case::negative_infinity(0, 1, f64::NEG_INFINITY)]
    #[case::factor_one(1, 1, 1f64)]
    #[case::factor_minus_two(1, 2, -2f64)]
    #[case::factor_two(2, 1, 2f64)]
    fn test_factor_diff_eq(#[case] a: u64, #[case] b: u64, #[case] expected: f64) {
        assert_eq!(factor_diff(a, b), expected);
    }
}
