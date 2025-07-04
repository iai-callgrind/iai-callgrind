//! This module provides common utility functions
use std::ffi::OsStr;
use std::io::{self, BufWriter, Write};
use std::ops::Neg;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};
use log::{debug, log_enabled, trace, Level};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use which::which;

use crate::error::Error;
use crate::runner::metrics::Metric;

// # Developer notes
//
// EitherOrBoth is not considered complete in terms of possible functionality. Simply extend and add
// new methods by need.

/// Either left or right or both can be present
///
/// Most of the time, this enum is used to store (new, old) output, metrics, etc. Per convention
/// left is `new` and right is `old`.
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum EitherOrBoth<T> {
    /// The left or `new` value
    Left(T),
    /// The right or `old` value
    Right(T),
    /// Both values (`new` and `old`) are present
    Both(T, T),
}

impl<T> EitherOrBoth<T> {
    /// Try to return the left (`new`) value
    pub fn left(&self) -> Option<&T> {
        match self {
            EitherOrBoth::Right(_) => None,
            EitherOrBoth::Both(left, _) | EitherOrBoth::Left(left) => Some(left),
        }
    }

    /// Try to return the right (`old`) value
    pub fn right(&self) -> Option<&T> {
        match self {
            EitherOrBoth::Left(_) => None,
            EitherOrBoth::Right(right) | EitherOrBoth::Both(_, right) => Some(right),
        }
    }

    /// Apply the function `f` to the inner value of `EitherOrBoth` and return a new `EitherOrBoth`
    pub fn map<F, N>(self, f: F) -> EitherOrBoth<N>
    where
        F: Fn(T) -> N,
    {
        match self {
            Self::Left(left) => EitherOrBoth::Left(f(left)),
            Self::Right(right) => EitherOrBoth::Right(f(right)),
            Self::Both(l, r) => EitherOrBoth::Both(f(l), f(r)),
        }
    }

    pub fn as_ref(&self) -> EitherOrBoth<&T> {
        match self {
            Self::Left(left) => EitherOrBoth::Left(left),
            Self::Right(right) => EitherOrBoth::Right(right),
            Self::Both(left, right) => EitherOrBoth::Both(left, right),
        }
    }
}

impl<T> TryFrom<(Option<T>, Option<T>)> for EitherOrBoth<T> {
    type Error = String;

    fn try_from(value: (Option<T>, Option<T>)) -> std::result::Result<Self, Self::Error> {
        match value {
            (None, None) => Err("Either the left, right or both values must be present".to_owned()),
            (None, Some(right)) => Ok(Self::Right(right)),
            (Some(left), None) => Ok(Self::Left(left)),
            (Some(left), Some(right)) => Ok(Self::Both(left, right)),
        }
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
/// exactly (case-sensitive).
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
        if !bytes.last().is_some_and(|l| *l == b'\n') {
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
        if !bytes.last().is_some_and(|l| *l == b'\n') {
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

/// Format a float as string depending on the number of digits of the integer-part without sign
///
/// Same as [`to_string_signed_short`] but without a sign.
pub fn to_string_unsigned_short(n: f64) -> String {
    to_string_signed_short(n)[1..].to_owned()
}

/// Calculate the difference between `new` and `old` as percentage
pub fn percentage_diff(new: Metric, old: Metric) -> f64 {
    if new == old {
        return 0f64;
    }

    let new: f64 = new.into();
    let old: f64 = old.into();

    let diff = (new - old) / old;
    diff * 100.0f64
}

/// Calculate the difference between `new` and `old` as factor
pub fn factor_diff(new: Metric, old: Metric) -> f64 {
    if new == old {
        return 1f64;
    }

    let new_float: f64 = new.into();
    let old_float: f64 = old.into();

    if new > old {
        if old == Metric::Int(0) {
            f64::INFINITY
        } else {
            new_float / old_float
        }
    } else if new == Metric::Int(0) {
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
    #[case::float_zero_int_zero(0, 0f64, 1f64)]
    #[case::int_zero_float_zero(0f64, 0, 1f64)]
    #[case::float_zero(0f64, 0f64, 1f64)]
    #[case::infinity_int(1, 0, f64::INFINITY)]
    #[case::infinity_div_int(1f64, 0, f64::INFINITY)]
    #[case::infinity_float(1f64, 0f64, f64::INFINITY)]
    #[case::infinity_float_mixed(1f64, 0, f64::INFINITY)]
    #[case::infinity_div_float(1, 0f64, f64::INFINITY)]
    #[case::negative_infinity(0, 1, f64::NEG_INFINITY)]
    #[case::negative_infinity_float(0f64, 1, f64::NEG_INFINITY)]
    #[case::factor_one(1, 1, 1f64)]
    #[case::factor_minus_two(1, 2, -2f64)]
    #[case::factor_two(2, 1, 2f64)]
    fn test_factor_diff_eq<L, R>(#[case] a: L, #[case] b: R, #[case] expected: f64)
    where
        L: Into<Metric>,
        R: Into<Metric>,
    {
        assert_eq!(factor_diff(a.into(), b.into()), expected);
    }
}
