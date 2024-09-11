use std::cmp::Ordering;
use std::fmt::Display;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use log::{trace, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::model::{Costs, Positions};
use crate::runner::tool::ToolOutputPath;
use crate::runner::DEFAULT_TOGGLE;

lazy_static! {
    static ref GLOB_TO_REGEX_RE: Regex =
        Regex::new(r"(\\)([*]|[?])").expect("Regex should compile");
}

pub type ParserOutput = Vec<(PathBuf, CallgrindProperties, Costs)>;

pub trait CallgrindParser {
    type Output;

    fn parse_single(&self, path: &Path) -> Result<(CallgrindProperties, Self::Output)>;
    fn parse(
        &self,
        output: &ToolOutputPath,
    ) -> Result<Vec<(PathBuf, CallgrindProperties, Self::Output)>> {
        let paths = output.real_paths()?;
        let mut results: Vec<(PathBuf, CallgrindProperties, Self::Output)> =
            Vec::with_capacity(paths.len());
        for path in paths {
            let parsed = self.parse_single(&path).map(|(p, c)| (path, p, c))?;

            let position = results
                .binary_search_by(|probe| probe.1.compare_target_ids(&parsed.1))
                .unwrap_or_else(|e| e);

            results.insert(position, parsed);
        }

        Ok(results)
    }
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct CallgrindProperties {
    pub costs_prototype: Costs,
    pub positions_prototype: Positions,
    pub pid: Option<i32>,
    pub thread: Option<usize>,
    pub part: Option<u64>,
    pub desc: Vec<String>,
    pub cmd: Option<String>,
    pub creator: Option<String>,
}

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentinel(#[serde(with = "serde_regex")] Regex);

impl CallgrindProperties {
    /// Compare by target ids `pid`, `part` and `thread`
    ///
    /// Highest precedence takes `pid`. Second is `part` and third is `thread` all sorted ascending.
    /// See also [Callgrind Format](https://valgrind.org/docs/manual/cl-format.html#cl-format.reference.grammar)
    pub fn compare_target_ids(&self, other: &Self) -> Ordering {
        self.pid.cmp(&other.pid).then_with(|| {
            self.part
                .cmp(&other.part)
                .then_with(|| self.thread.cmp(&other.thread))
        })
    }
}

impl Sentinel {
    /// Create a new Sentinel
    ///
    /// The value is converted to a regex internally which matches from line start to line end.
    ///
    /// Do not use this method if the input is a glob pattern or cannot be trusted! Use
    /// [`Sentinel::from_glob`] instead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind_runner::runner::callgrind::parser::Sentinel;
    ///
    /// let sentinel = Sentinel::new("main").unwrap();
    /// assert_eq!(sentinel.to_string(), String::from("^main$"));
    /// ```
    pub fn new<T>(value: T) -> Result<Self>
    where
        T: AsRef<str>,
    {
        Regex::new(&format!("^{}$", value.as_ref()))
            .map(Self)
            .with_context(|| "Invalid sentinel")
    }

    /// Create a new Sentinel from a glob pattern
    ///
    /// Any `*` is replaced with `.*` and `?` with `.?` because we need the glob as regex
    /// internally. A Character will be [escaped](https://docs.rs/regex/latest/regex/fn.escape.html)
    /// if it is a regex meta character so this method produces a safe regular expression.
    /// Additionally, the glob matches from the start to end of the string
    ///
    /// The glob pattern is defined in more detail
    /// [here](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.optionshttps://valgrind.org/docs/manual/cl-manual.html#cl-manual.options)
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind_runner::runner::callgrind::parser::Sentinel;
    ///
    /// let sentinel = Sentinel::from_glob("*::main").unwrap();
    /// assert_eq!(sentinel.to_string(), String::from("^.*::main$"));
    /// ```
    pub fn from_glob<T>(glob: T) -> Result<Self>
    where
        T: AsRef<str>,
    {
        let escaped = regex::escape(glob.as_ref());
        let replaced = GLOB_TO_REGEX_RE.replace_all(&escaped, ".$2");
        Self::new(replaced)
    }

    pub fn from_path(module: &str, function: &str) -> Self {
        Self::new(format!("{module}::{function}")).expect("Regex should compile")
    }

    pub fn from_segments<I, T>(segments: T) -> Self
    where
        I: AsRef<str>,
        T: AsRef<[I]>,
    {
        let joined = if let Some((first, suffix)) = segments.as_ref().split_first() {
            suffix.iter().fold(first.as_ref().to_owned(), |mut a, b| {
                a.push_str("::");
                a.push_str(b.as_ref());
                a
            })
        } else {
            String::new()
        };
        Self::new(joined).expect("Regex should compile")
    }

    pub fn matches(&self, haystack: &str) -> bool {
        self.0.is_match(haystack)
    }
}

impl AsRef<Sentinel> for Sentinel {
    fn as_ref(&self) -> &Sentinel {
        self
    }
}

impl Default for Sentinel {
    fn default() -> Self {
        Self::from_glob(DEFAULT_TOGGLE).expect("Default toggle should compile as regex")
    }
}

impl Display for Sentinel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Eq for Sentinel {}

impl From<Sentinel> for String {
    fn from(value: Sentinel) -> Self {
        value.0.as_str().to_owned()
    }
}

impl PartialEq for Sentinel {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

pub fn parse_header(iter: &mut impl Iterator<Item = String>) -> Result<CallgrindProperties> {
    if !iter
        .by_ref()
        .find(|l| !l.trim().is_empty())
        .ok_or(anyhow!("Empty file"))?
        .contains("callgrind format")
    {
        warn!("Missing file format specifier. Assuming callgrind format.");
    };

    let mut positions_prototype: Option<Positions> = None;
    let mut costs_prototype: Option<Costs> = None;
    let mut pid: Option<i32> = None;
    let mut thread: Option<usize> = None;
    let mut part: Option<u64> = None;
    let mut desc: Vec<String> = vec![];
    let mut cmd: Option<String> = None;
    let mut creator: Option<String> = None;

    for line in iter.filter(|line| {
        let line = line.trim();
        !line.is_empty() && !line.starts_with('#')
    }) {
        match line.split_once(':').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("version", version)) if version != "1" => {
                return Err(anyhow!(
                    "Version mismatch: Requires callgrind format version '1' but was '{version}'"
                ));
            }
            Some(("pid", value)) => {
                trace!("Using pid '{value}' from line: '{line}'");
                pid = Some(value.parse::<i32>().unwrap());
            }
            Some(("thread", value)) => {
                trace!("Using thread '{value}' from line: '{line}'");
                thread = Some(value.parse::<usize>().unwrap());
            }
            Some(("part", value)) => {
                trace!("Using part '{value}' from line: '{line}'");
                part = Some(value.parse::<u64>().unwrap());
            }
            Some(("desc", value)) if !value.starts_with("Option:") => {
                trace!("Using description '{value}' from line: '{line}'");
                desc.push(value.to_owned());
            }
            Some(("cmd", value)) => {
                trace!("Using cmd '{value}' from line: '{line}'");
                cmd = Some(value.to_owned());
            }
            Some(("creator", value)) => {
                trace!("Using creator '{value}' from line: '{line}'");
                creator = Some(value.to_owned());
            }
            Some(("positions", positions)) => {
                trace!("Using positions '{positions}' from line: '{line}'");
                positions_prototype = Some(positions.split_ascii_whitespace().collect());
            }
            // The events line is the last line in the header which is mandatory (according to
            // the source code of callgrind_annotate). The summary line is usually the last line
            // but it is only optional. So, we break out of the loop here and stop the parsing.
            Some(("events", events)) => {
                trace!("Using events '{events}' from line: '{line}'");
                costs_prototype = Some(events.split_ascii_whitespace().collect());
                break;
            }
            // None is actually a malformed header line we just ignore here
            // Some(_) includes `^event:` lines
            None | Some(_) => {
                continue;
            }
        }
    }

    Ok(CallgrindProperties {
        costs_prototype: costs_prototype
            .ok_or_else(|| anyhow!("Header field 'events' must be present"))?,
        positions_prototype: positions_prototype.unwrap_or_default(),
        pid,
        thread,
        part,
        desc,
        cmd,
        creator,
    })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::simple("foo", "^foo$")]
    #[case::glob(r"?*", r"^?*$")] // Does not interpret glob patterns
    fn test_sentinel_new(#[case] input: &str, #[case] expected: &str) {
        let expected_sentinel = Sentinel(Regex::new(expected).unwrap());
        let sentinel = Sentinel::new(input).unwrap();

        assert_eq!(sentinel, expected_sentinel);
    }

    #[rstest]
    #[case::simple("foo", "^foo$")]
    #[case::only_star("*", "^.*$")]
    #[case::with_star_in_the_middle("f*oo", "^f.*oo$")]
    #[case::star_at_start("*foo", "^.*foo$")]
    #[case::star_at_end("foo*", "^foo.*$")]
    #[case::two_stars("f**o", "^f.*.*o$")]
    #[case::only_question_mark("?", "^.?$")]
    #[case::with_question_mark("f?o", "^f.?o$")]
    #[case::two_question_marks("f??o", "^f.?.?o$")]
    #[case::question_mark_at_start("?foo", "^.?foo$")]
    #[case::question_mark_at_end("foo?", "^foo.?$")]
    #[case::mixed("f*o?o", "^f.*o.?o$")]
    fn test_sentinel_from_glob(#[case] input: &str, #[case] expected: &str) {
        let expected_sentinel = Sentinel(Regex::new(expected).unwrap());
        let sentinel = Sentinel::from_glob(input).unwrap();

        assert_eq!(sentinel, expected_sentinel);
    }

    /// These are some non-exhaustive real world examples which a sentinel should be able to match
    #[rstest]
    #[case::main_binary("*::main", "by_binary::main")]
    #[case::below_main_exact("(below_main)", "(below_main)")]
    #[case::below_main_with_glob("?below_main?", "(below_main)")]
    #[case::exit("*exit*", "exit")]
    #[case::with_at_sign("__cpu_indicator_init*", "__cpu_indicator_init@GCC_4.8.0")]
    #[case::simple_function(
        "*::stack_overflow::*",
        "std::sys::unix::stack_overflow::imp::make_handler"
    )]
    #[case::generic(
        "std::sync::once_lock::OnceLock<*>*",
        "std::sync::once_lock::OnceLock<T>::initialize"
    )]
    #[case::generic_with_as(
        "<* as core::fmt::Write>::write_str",
        "<std::io::Write::write_fmt::Adapter<T> as core::fmt::Write>::write_str"
    )]
    #[case::generic_with_as_reference(
        "<&*>::write_fmt",
        "<&std::io::stdio::Stdout as std::io::Write>::write_fmt"
    )]
    #[case::generic_match_all(
        "*::write_fmt",
        "<&std::io::stdio::Stdout as std::io::Write>::write_fmt"
    )]
    #[case::hex("0x*", "0x00000000000083f0")]
    fn test_sentinel_from_glob_matches(#[case] input: &str, #[case] haystack: &str) {
        assert!(Sentinel::from_glob(input).unwrap().matches(haystack));
    }
}
