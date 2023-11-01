use std::fmt::Display;

use anyhow::{anyhow, Result};
use log::{trace, warn};
use serde::{Deserialize, Serialize};

use super::model::{Costs, Positions};
use crate::runner::common::ToolOutput;

#[derive(Debug, Default)]
pub struct CallgrindProperties {
    pub costs_prototype: Costs,
    pub positions_prototype: Positions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sentinel(String);

pub trait Parser {
    type Output;

    fn parse(&self, output: &ToolOutput) -> Result<Self::Output>
    where
        Self: std::marker::Sized;
}
impl Sentinel {
    pub fn new<T>(value: T) -> Self
    where
        T: Into<String>,
    {
        Self(value.into())
    }

    pub fn from_path(module: &str, function: &str) -> Self {
        Self(format!("{module}::{function}"))
    }

    #[allow(unused)]
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
        Self(joined)
    }

    pub fn to_fn(&self) -> String {
        format!("fn={}", self.0)
    }

    pub fn matches(&self, string: &str) -> bool {
        string.starts_with(self.0.as_str())
    }
}

impl AsRef<Sentinel> for Sentinel {
    fn as_ref(&self) -> &Sentinel {
        self
    }
}

impl Display for Sentinel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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
