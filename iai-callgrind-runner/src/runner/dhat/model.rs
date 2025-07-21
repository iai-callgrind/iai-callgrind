//! This module contains the structs to model the dhat output file content

// spell-checker: ignore bklt bkacc bksu tuth ftbl tgmax
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

lazy_static! {
    static ref FRAME_RE: Regex = regex::Regex::new(
        r"^(?<root>\[root\])|(?<addr>0x[0-9a-fA-F]+):\s*(?<func>.*)\s\((?<in>.*)\)$"
    )
    .expect("Regex should compile");
}

/// A [`Frame`] in the [`DhatData::frame_table`]
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The root frame
    Root,
    /// All other frames than the root are leafs
    Leaf(String, String, String),
}

/// The dhat invocation mode
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// --mode=heap
    #[default]
    Heap,
    /// --mode=ad-hoc
    AdHoc,
    /// --mode=copy
    Copy,
}

/// The top-level data extracted from dhat json output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::arbitrary_source_item_ordering)]
pub struct DhatData {
    /// Version number of the format
    #[serde(rename = "dhatFileVersion")]
    pub dhat_file_version: usize,

    /// The invocation mode
    pub mode: Mode,

    /// The verb used before above stack frames
    pub verb: String,

    /// Are block lifetimes recorded? Affects whether some other fields are present.
    #[serde(rename = "bklt")]
    pub has_block_lifetimes: bool,

    /// Are block accesses recorded? Affects whether some other fields are present
    #[serde(rename = "bkacc")]
    pub has_block_accesses: bool,

    /// Byte units. "byte" is the values used if these fields are omitted.
    #[serde(rename = "bu")]
    pub byte_unit: Option<String>,

    /// Bytes units. "bytes" is the values used if these fields are omitted.
    #[serde(rename = "bsu")]
    pub bytes_unit: Option<String>,

    /// Blocks units. "blocks" is the values used if these fields are omitted.
    #[serde(rename = "bksu")]
    pub block_unit: Option<String>,

    /// Time units (individual)
    #[serde(rename = "tu")]
    pub time_unit: String,

    /// Time units (1,000,000x)
    #[serde(rename = "Mtu")]
    pub time_unit_m: String,

    /// The "short-lived" time threshold, measures in "tu"s (`time_unit`).
    /// - bklt=true: a mandatory integer.
    /// - bklt=false: omitted.
    #[serde(rename = "tuth")]
    pub time_threshold: Option<usize>,

    /// The executed command
    #[serde(rename = "cmd")]
    pub command: String,

    /// The process ID
    pub pid: i32,

    /// The time at the end of execution (t-end)
    #[serde(rename = "te")]
    pub time_end: u128,

    /// The time of the global max (t-gmax)
    /// - bklt=true: a mandatory integer.
    /// - bklt=false: omitted.
    #[serde(rename = "tg")]
    pub time_global_max: Option<u128>,

    /// The [`ProgramPoint`]s
    #[serde(rename = "pps")]
    pub program_points: Vec<ProgramPoint>,

    /// [`Frame`] table
    #[serde(rename = "ftbl")]
    pub frame_table: Vec<Frame>,
}

/// A `ProgramPoint`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::arbitrary_source_item_ordering)]
pub struct ProgramPoint {
    /// Total bytes
    #[serde(rename = "tb")]
    pub total_bytes: u64,

    /// Total blocks
    #[serde(rename = "tbk")]
    pub total_blocks: u64,

    /// Total lifetimes of all blocks allocated at this `ProgramPoint`.
    /// - bklt=true: a mandatory integer.
    /// - bklt=false: omitted.
    #[serde(rename = "tl")]
    pub total_lifetimes: Option<u128>,

    /// The maximum bytes for this `ProgramPoint`
    /// - bklt=true: mandatory integers.
    /// - bklt=false: omitted.
    #[serde(rename = "mb")]
    pub maximum_bytes: Option<u64>,

    /// The maximum blocks for this `ProgramPoint`
    /// - bklt=true: mandatory integers.
    /// - bklt=false: omitted.
    #[serde(rename = "mbk")]
    pub maximum_blocks: Option<u64>,

    /// The bytes at t-gmax for this `ProgramPoint`
    /// - bklt=true: mandatory integers.
    /// - bklt=false: omitted.
    #[serde(rename = "gb")]
    pub bytes_at_max: Option<u64>,

    /// The blocks at t-gmax for this `ProgramPoint`
    /// - bklt=true: mandatory integers.
    /// - bklt=false: omitted.
    #[serde(rename = "gbk")]
    pub blocks_at_max: Option<u64>,

    /// The bytes at t-end for this `ProgramPoint`
    /// - bklt=true: mandatory integers.
    /// - bklt=false: omitted.
    #[serde(rename = "eb")]
    pub bytes_at_end: Option<u64>,

    /// The blocks at t-end for this `ProgramPoint`
    /// - bklt=true: mandatory integers.
    /// - bklt=false: omitted.
    #[serde(rename = "ebk")]
    pub blocks_at_end: Option<u64>,

    /// The reads of blocks for this `ProgramPoint`
    /// - bkacc=true: mandatory integers.
    /// - bkacc=false: omitted.
    #[serde(rename = "rb")]
    pub blocks_read: Option<u64>,

    /// The writes of blocks for this `ProgramPoint`
    /// - bkacc=true: mandatory integers.
    /// - bkacc=false: omitted.
    #[serde(rename = "wb")]
    pub blocks_write: Option<u64>,

    /// The exact accesses of blocks for this `ProgramPoint`. Only used when all allocations are
    /// the same size and sufficiently small. A negative element indicates run-length encoding
    /// of the following integer. E.g. `-3, 4` means "three 4s in a row".
    /// - bkacc=true: an optional array of integers.
    /// - bkacc=false: omitted.
    #[serde(rename = "acc")]
    pub accesses: Option<Vec<i64>>,

    /// Frames. Each element is an index into the [`DhatData::frame_table`]
    #[serde(rename = "fs")]
    pub frames: Vec<usize>,
}

impl<'de> Deserialize<'de> for Frame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let frame = String::deserialize(deserializer)?;
        Frame::from_str(&frame).map_err(Error::custom)
    }
}

impl From<(&str, &str, &str)> for Frame {
    fn from((addr, func, loc): (&str, &str, &str)) -> Self {
        Self::Leaf(addr.to_owned(), func.to_owned(), loc.to_owned())
    }
}

impl FromStr for Frame {
    type Err = String;

    fn from_str(haystack: &str) -> Result<Self, Self::Err> {
        let caps = FRAME_RE
            .captures(haystack)
            .ok_or_else(|| "invalid frame format".to_owned())?;

        if caps.name("root").is_some() {
            Ok(Frame::Root)
        } else {
            Ok(Frame::Leaf(
                caps.name("addr")
                    .expect("An address should be present")
                    .as_str()
                    .to_owned(),
                caps.name("func")
                    .expect("A function should be present")
                    .as_str()
                    .to_owned(),
                caps.name("in")
                    .expect("A location should be present")
                    .as_str()
                    .to_owned(),
            ))
        }
    }
}

impl Serialize for Frame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = match self {
            Frame::Root => "[root]".to_owned(),
            Frame::Leaf(addr, func, loc) => format!("{addr}: {func} ({loc})"),
        };

        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let frame = String::deserialize(deserializer)?;
        let mode = match frame.to_lowercase().as_str() {
            "ad-hoc" => Mode::AdHoc,
            "heap" => Mode::Heap,
            "copy" => Mode::Copy,
            _ => return Err(Error::custom("Invalid mode")),
        };

        Ok(mode)
    }
}

impl Serialize for Mode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = match self {
            Mode::Heap => "heap",
            Mode::AdHoc => "ad-hoc",
            Mode::Copy => "copy",
        };

        serializer.serialize_str(string)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_test::{assert_tokens, Token};

    use super::*;

    #[rstest]
    #[case::short_addr("0x1234: malloc (in /usr/lib/some.so)", ("0x1234", "malloc", "in /usr/lib/some.so"))]
    #[case::no_in("0x12345678: malloc (/usr/lib/some.so)", ("0x12345678", "malloc", "/usr/lib/some.so"))]
    #[case::some("0x12345678: malloc (in /usr/lib/some.so)", ("0x12345678", "malloc", "in /usr/lib/some.so"))]
    #[case::long_with_multiple_parentheses("0x40440E3: call_once<(), (dyn core::ops::function::Fn<(), Output=i32> + core::marker::Sync + core::panic::unwind_safe::RefUnwindSafe)> (function.rs:284)", ("0x40440E3", "call_once<(), (dyn core::ops::function::Fn<(), Output=i32> + core::marker::Sync + core::panic::unwind_safe::RefUnwindSafe)>", "function.rs:284"))]
    fn test_frame_from_str(#[case] haystack: &str, #[case] frame: (&str, &str, &str)) {
        let expected = Frame::from(frame);
        let actual = haystack.parse::<Frame>().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_frame_de_and_serialize_frame() {
        let frame = Frame::from(("0x1234", "malloc", "in /usr/lib/some.so"));
        assert_tokens(
            &frame,
            &[Token::Str("0x1234: malloc (in /usr/lib/some.so)")],
        );
    }

    #[test]
    fn test_frame_de_and_serialize_root() {
        let frame = Frame::Root;
        assert_tokens(&frame, &[Token::Str("[root]")]);
    }

    #[test]
    fn test_frame_from_str_when_root() {
        let expected = Frame::Root;
        let actual = "[root]".parse::<Frame>().unwrap();
        assert_eq!(actual, expected);
    }
}
