use std::fmt::Display;

use anyhow::{anyhow, Context, Result};
use log::{trace, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::model::{Costs, Positions};
use crate::runner::DEFAULT_TOGGLE;

#[derive(Debug, Default)]
pub struct CallgrindProperties {
    pub costs_prototype: Costs,
    pub positions_prototype: Positions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentinel(#[serde(with = "serde_regex")] Regex);

impl Sentinel {
    /// Create a new Sentinel
    ///
    /// A Sentinel is converted to a regex internally which matches from line start to line end.
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
    /// The `*` are replaced with `.*` because we need the glob as regex. Additionally, the glob
    /// matches from the start to end of the string.
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
        let regex = glob.as_ref().replace('*', ".*");
        Self::new(regex)
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

    for line in iter {
        if line.is_empty() || line.starts_with('#') {
            // skip empty lines or comments
            continue;
        }
        match line.split_once(':').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("version", version)) if version != "1" => {
                return Err(anyhow!(
                    "Version mismatch: Requires callgrind format version '1' but was '{version}'"
                ));
            }
            Some(("positions", positions)) => {
                positions_prototype = Some(positions.split_ascii_whitespace().collect());
                trace!("Using positions: '{:?}'", positions_prototype);
            }
            // The events line is the last line in the header which is mandatory (according to
            // the source code of callgrind_annotate). The summary line is usually the last line
            // but it is only optional. So, we break out of the loop here and stop the parsing.
            Some(("events", events)) => {
                trace!("Using events from line: '{line}'");
                costs_prototype = Some(events.split_ascii_whitespace().collect());
                break;
            }
            // None is actually a malformed header line we just ignore here
            None | Some(_) => {
                continue;
            }
        }
    }

    Ok(CallgrindProperties {
        costs_prototype: costs_prototype
            .ok_or_else(|| anyhow!("Header field 'events' must be present"))?,
        positions_prototype: positions_prototype.unwrap_or_default(),
    })
}
