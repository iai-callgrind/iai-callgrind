use std::fmt::Display;
use std::io::{stderr, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::str::FromStr;

use client_request_tests::MARKER;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref STRIP_PREFIX_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--)\s*(?<rest>.*)$")
            .expect("Regex should compile");
    static ref CALLGRIND_EXCLUDED_LINES_RE: Regex =
        regex::Regex::new(r"^(For interactive control,)").expect("Regex should compile");
    // The following regex are almost 1:1 taken from valgrind.git/callgrind/tests/filter_stderr
    // to filter the numbers from stderr
    static ref CALLGRIND_RM_NUM_REFS_RE: Regex =
        regex::Regex::new(r"((I|D|LL)\s*refs:)[ 0-9,()+rdw]*$").expect("Regex should compile");
    static ref CALLGRIND_RM_NUM_MISS_RE: Regex =
        regex::Regex::new(r"((I1|D1|LL|LLi|LLd)\s*(misses|miss rate):)[ 0-9,()+rdw%.]*$")
            .expect("Regex should compile");
    static ref CALLGRIND_RM_NUM_RATE_RE: Regex =
        regex::Regex::new(r"((Branches|Mispredicts|Mispred rate):)[ 0-9,()+condi%.]*$")
            .expect("Regex should compile");
    static ref CALLGRIND_RM_NUM_COLLECTED_RE: Regex =
        regex::Regex::new(r"^(Collected\s*:)[ 0-9]*$").expect("Regex should compile");
    static ref CALLGRIND_RM_DUMP_TO_RE: Regex =
        regex::Regex::new(r"^(Dump to).*$").expect("Regex should compile");
    static ref CALLGRIND_RM_BB_NUM_RE: Regex =
        regex::Regex::new(r"^(.*at BB\s*)([0-9]*)(.*)$").expect("Regex should compile");
}

#[derive(Debug)]
enum Tool {
    Callgrind,
}

impl FromStr for Tool {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "callgrind" => Ok(Tool::Callgrind),
            tool => Err(format!("Unsupported tool: {tool}")),
        }
    }
}

impl Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).to_ascii_lowercase().as_str())
    }
}

fn callgrind_filter(bytes: &[u8], writer: &mut impl Write) {
    #[derive(Debug, PartialEq, Eq)]
    enum State {
        Header,
        Body,
    }
    let mut state = State::Header;
    for line in BufReader::new(bytes).lines().map(Result::unwrap) {
        if state == State::Header {
            if line.contains(MARKER) {
                state = State::Body;
            }
            continue;
        }
        let rest = STRIP_PREFIX_RE
            .captures(&line)
            .expect("Callgrind output line should be a valid output line")
            .name("rest")
            .unwrap()
            .as_str();
        if !CALLGRIND_EXCLUDED_LINES_RE.is_match(rest) {
            let rest = if let Some(caps) = CALLGRIND_RM_NUM_REFS_RE.captures(rest) {
                caps.get(1).unwrap().as_str()
            } else if let Some(caps) = CALLGRIND_RM_NUM_MISS_RE.captures(rest) {
                caps.get(1).unwrap().as_str()
            } else if let Some(caps) = CALLGRIND_RM_NUM_RATE_RE.captures(rest) {
                caps.get(1).unwrap().as_str()
            } else if let Some(caps) = CALLGRIND_RM_NUM_COLLECTED_RE.captures(rest) {
                caps.get(1).unwrap().as_str()
            } else if let Some(caps) = CALLGRIND_RM_DUMP_TO_RE.captures(rest) {
                caps.get(1).unwrap().as_str()
            } else {
                rest
            };
            writeln!(writer, "{rest}").unwrap();
        }
    }
}

fn main() {
    let mut env_args = std::env::args();
    let base_dir = {
        let exe = PathBuf::from(env_args.next().unwrap());
        exe.parent().unwrap().to_owned()
    };

    let tool = {
        let tool = env_args.next().unwrap();
        Tool::from_str(tool.strip_prefix("--tool=").unwrap())
            .expect("Tool should be a valid valgrind tool")
    };

    let valgrind_args = {
        let args = env_args.next().unwrap();
        shlex::split(args.strip_prefix("--valgrind-args=").unwrap()).unwrap()
    };

    let bin = {
        let bin = env_args.next().unwrap();
        base_dir.join(bin.strip_prefix("--bin=").unwrap())
    };

    let bin_args = {
        if let Some(args) = env_args.next() {
            shlex::split(args.strip_prefix("--bin-args=").unwrap()).unwrap()
        } else {
            vec![]
        }
    };

    let output = std::process::Command::new("valgrind")
        .arg(format!("--tool={tool}"))
        .args(valgrind_args)
        .arg(bin)
        .args(bin_args)
        .output()
        .unwrap();

    match tool {
        Tool::Callgrind => callgrind_filter(&output.stderr, &mut stderr()),
    }

    std::process::exit(output.status.code().unwrap());
}
