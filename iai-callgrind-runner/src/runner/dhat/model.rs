//! spell-checker: ignore bklt bkacc bksu tuth ftbl tgmax

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DhatData {
    #[serde(rename = "dhatFileVersion")]
    pub dhat_file_version: usize,
    pub mode: Mode,
    pub verb: String,
    #[serde(rename = "bklt")]
    pub has_block_lifetimes: bool,
    #[serde(rename = "bkacc")]
    pub has_block_accesses: bool,
    #[serde(rename = "bu")]
    pub byte_unit: Option<String>,
    #[serde(rename = "bsu")]
    pub bytes_unit: Option<String>,
    #[serde(rename = "bksu")]
    pub block_unit: Option<String>,
    #[serde(rename = "tu")]
    pub time_unit: String,
    #[serde(rename = "Mtu")]
    pub time_unit_m: String,
    #[serde(rename = "tuth")]
    pub time_threshold: Option<usize>,
    #[serde(rename = "cmd")]
    pub command: String,
    pub pid: i32,
    #[serde(rename = "te")]
    pub time_end: u128,
    #[serde(rename = "tg")]
    pub time_global_max: Option<u128>,
    #[serde(rename = "pps")]
    pub program_points: Vec<ProgramPoint>,
    #[serde(rename = "ftbl")]
    pub frame_table: Vec<Frame>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Root,
    Leaf(String, String, String),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Heap,
    AdHoc,
    Copy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgramPoint {
    #[serde(rename = "tb")]
    pub total_bytes: u64,
    #[serde(rename = "tbk")]
    pub total_blocks: u64,
    #[serde(rename = "tl")]
    pub total_lifetimes: Option<u128>,
    #[serde(rename = "mb")]
    pub maximum_bytes: Option<u64>,
    #[serde(rename = "mbk")]
    pub maximum_blocks: Option<u64>,
    #[serde(rename = "gb")]
    pub bytes_at_max: Option<u64>,
    #[serde(rename = "gbk")]
    pub blocks_at_max: Option<u64>,
    #[serde(rename = "eb")]
    pub bytes_at_end: Option<u64>,
    #[serde(rename = "ebk")]
    pub blocks_at_end: Option<u64>,
    #[serde(rename = "rb")]
    pub blocks_read: Option<u64>,
    #[serde(rename = "wb")]
    pub blocks_write: Option<u64>,
    #[serde(rename = "acc")]
    pub accesses: Option<Vec<i64>>,
    #[serde(rename = "fs")]
    pub frames: Vec<usize>,
}

impl From<(&str, &str, &str)> for Frame {
    fn from((addr, func, loc): (&str, &str, &str)) -> Self {
        Self::Leaf(addr.to_owned(), func.to_owned(), loc.to_owned())
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

impl<'de> Deserialize<'de> for Frame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let frame = String::deserialize(deserializer)?;
        Frame::from_str(&frame).map_err(Error::custom)
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

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_test::{assert_tokens, Token};

    use super::*;

    #[test]
    fn test_frame_from_str_when_root() {
        let expected = Frame::Root;
        let actual = "[root]".parse::<Frame>().unwrap();
        assert_eq!(actual, expected);
    }

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
    fn test_frame_de_and_serialize_root() {
        let frame = Frame::Root;
        assert_tokens(&frame, &[Token::Str("[root]")]);
    }

    #[test]
    fn test_frame_de_and_serialize_frame() {
        let frame = Frame::from(("0x1234", "malloc", "in /usr/lib/some.so"));
        assert_tokens(
            &frame,
            &[Token::Str("0x1234: malloc (in /usr/lib/some.so)")],
        );
    }
}
