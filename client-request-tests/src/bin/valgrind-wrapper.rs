use std::fmt::Display;
use std::io::{stderr, BufRead, BufReader, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use client_request_tests::{CROSS_TARGET, MARKER};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref STRIP_PREFIX_RE: Regex =
        regex::Regex::new(r"^\s*(==|--|\*\*)([0-9:.]+\s+)?[0-9]+(==|--|\*\*)\s*(?<rest>.*)$")
            .expect("Regex should compile");
    static ref CALLGRIND_EXCLUDED_LINES_RE: Regex =
        regex::Regex::new(r"^(For interactive control,)").expect("Regex should compile");
    static ref CALLGRIND_RM_DUMP_TO_RE: Regex =
        regex::Regex::new(r"^(Dump to).*$").expect("Regex should compile");
    static ref CALLGRIND_RM_BB_NUM_RE: Regex =
        regex::Regex::new(r"(at BB\s+)([0-9]+)(\s+.*)$").expect("Regex should compile");
    static ref CALLGRIND_RM_ADDR_RE: Regex =
        regex::Regex::new(r"((at|by)\s+)(0x[0-9A-Za-z]+\s*:.*)$").expect("Regex should compile");
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
    static ref CALLGRIND_RM_LINE_NUM_RE: Regex =
        regex::Regex::new(r"(\(.*:)([0-9]+)(\))\s*$").expect("Regex should compile");
    static ref BACKTRACE_RE: Regex =
        regex::Regex::new(r"^((at|by)\s*0x[0-9A-Za-z]+\s*:)").expect("Regex should compile");
    static ref MEMCHECK_CHECKED_RE: Regex =
        regex::Regex::new(r"^(\s*Checked\s*)([0-9,.]+)(\s*bytes)\s*$").expect("Regex should compile");
    static ref MEMCHECK_TOTAL_HEAP_USAGE_RE: Regex =
        regex::Regex::new(r"^(?i:(\s*total heap usage:\s*))(.*)$").expect("Regex should compile");
    static ref MEMCHECK_LEAK_SUMMARY_RE: Regex =
        regex::Regex::new(r"(?i:(\s*(definitely lost|indirectly lost|possibly lost|still reachable|suppressed):\s*))([ 0-9,()+.]*)(\s*bytes in\s*)([ 0-9,()+.]*)(\s*blocks\s*)$")
            .expect("Regex should compile");
    static ref MEMORY_ADDRESS_RE: Regex =
        regex::Regex::new(r"0x[0-9A-Za-z]+").expect("Regex should compile");
}

#[derive(Debug)]
enum Tool {
    Callgrind,
    Memcheck,
    Helgrind,
}

impl FromStr for Tool {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "callgrind" => Ok(Tool::Callgrind),
            "memcheck" => Ok(Tool::Memcheck),
            "helgrind" => Ok(Tool::Helgrind),
            tool => Err(format!("Unsupported tool: {tool}")),
        }
    }
}

impl Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).to_ascii_lowercase().as_str())
    }
}

fn callgrind_filter(path: &Path, bytes: &[u8], writer: &mut impl Write) {
    let path_re =
        regex::Regex::new(format!(r"({})", path.display()).as_str()).expect("Regex should compile");

    #[derive(Debug, PartialEq, Eq)]
    enum State {
        Header,
        Body,
    }
    let mut state = State::Header;
    let mut is_backtrace = false;
    for line in BufReader::new(bytes).lines().map(Result::unwrap) {
        if state == State::Header {
            if line.contains(MARKER) {
                state = State::Body;
            }
            continue;
        }
        let rest = STRIP_PREFIX_RE
            .captures(&line)
            .unwrap_or_else(|| {
                panic!("Callgrind output line should be a valid output line: was {line}")
            })
            .name("rest")
            .unwrap()
            .as_str();

        // backtraces are too different on different targets
        if BACKTRACE_RE.is_match(rest) {
            if !is_backtrace {
                writeln!(writer, "<__BACKTRACE__>").unwrap();
                is_backtrace = true;
            }
            continue;
        } else {
            is_backtrace = false;
        }
        let replaced = path_re.replace_all(rest, "<__FILTER__>");
        let replaced = CALLGRIND_RM_ADDR_RE.replace_all(&replaced, "$1<__FILTER__>");
        let replaced = CALLGRIND_RM_BB_NUM_RE.replace_all(&replaced, "$1<__FILTER__>$3");
        let replaced = CALLGRIND_RM_LINE_NUM_RE.replace_all(&replaced, "$1<__FILTER__>$3");
        let rest = replaced.deref();
        if !CALLGRIND_EXCLUDED_LINES_RE.is_match(rest) {
            if let Some(caps) = CALLGRIND_RM_NUM_REFS_RE.captures(rest) {
                writeln!(writer, "{}", caps.get(1).unwrap().as_str()).unwrap();
            } else if let Some(caps) = CALLGRIND_RM_NUM_MISS_RE.captures(rest) {
                writeln!(writer, "{}", caps.get(1).unwrap().as_str()).unwrap();
            } else if let Some(caps) = CALLGRIND_RM_NUM_RATE_RE.captures(rest) {
                writeln!(writer, "{}", caps.get(1).unwrap().as_str()).unwrap();
            } else if let Some(caps) = CALLGRIND_RM_NUM_COLLECTED_RE.captures(rest) {
                writeln!(writer, "{}", caps.get(1).unwrap().as_str()).unwrap();
            } else if let Some(caps) = CALLGRIND_RM_DUMP_TO_RE.captures(rest) {
                writeln!(writer, "{}", caps.get(1).unwrap().as_str()).unwrap();
            } else {
                writeln!(writer, "{rest}").unwrap();
            }
        }
    }
}

fn memcheck_filter(bytes: &[u8], writer: &mut impl Write) {
    #[derive(Debug, PartialEq, Eq)]
    enum State {
        Header,
        Body,
    }
    let mut state = State::Header;
    let mut is_backtrace = false;
    for line in BufReader::new(bytes).lines().map(Result::unwrap) {
        if state == State::Header {
            if line.contains(MARKER) {
                state = State::Body;
            }
            continue;
        }
        let rest = STRIP_PREFIX_RE
            .captures(&line)
            .unwrap_or_else(|| {
                panic!("Memcheck output line should be a valid output line: was {line}")
            })
            .name("rest")
            .unwrap()
            .as_str();

        // backtraces are too different on different targets
        if BACKTRACE_RE.is_match(rest) {
            if !is_backtrace {
                writeln!(writer, "<__BACKTRACE__>").unwrap();
                is_backtrace = true;
            }
            continue;
        } else {
            is_backtrace = false;
        }

        let replaced = MEMCHECK_CHECKED_RE.replace_all(rest, "$1<__FILTER__>$3");
        let replaced = MEMCHECK_TOTAL_HEAP_USAGE_RE.replace_all(&replaced, "$1<__FILTER__>");
        let replaced =
            MEMCHECK_LEAK_SUMMARY_RE.replace_all(&replaced, "$1<__FILTER__> $4<__FILTER__> $6");
        let replaced = MEMORY_ADDRESS_RE.replace_all(&replaced, "<__FILTER__>");
        writeln!(writer, "{replaced}").unwrap();
    }
}

pub fn get_valgrind_command() -> Command {
    let valgrind = PathBuf::from("/target/valgrind")
        .join(CROSS_TARGET)
        .join("bin/valgrind");
    if !valgrind.exists() {
        panic!("Running the tests without cross is unsupported");
    }
    Command::new(valgrind)
}

fn main() {
    let mut env_args = std::env::args();
    let base_dir = {
        let exe = PathBuf::from(env_args.next().unwrap());
        exe.parent().unwrap().to_owned()
    };

    let expected_exit_code = env_args.next().unwrap().parse::<i32>().unwrap();

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

    let mut cmd = get_valgrind_command();
    cmd.arg(format!("--tool={tool}"))
        .args(valgrind_args)
        .arg(&bin)
        .args(bin_args);
    let output = cmd.output().unwrap();

    if let Some(code) = output.status.code() {
        if code == expected_exit_code {
            match tool {
                Tool::Callgrind => callgrind_filter(&bin, &output.stderr, &mut stderr()),
                Tool::Memcheck => memcheck_filter(&output.stderr, &mut stderr()),
                _ => (),
            }
        } else {
            let stderr = stderr();
            let mut stderr = stderr.lock();
            eprintln!("Unexpected exit code '{code}' when running {cmd:?}");
            eprintln!("STDOUT:");
            stderr.write_all(&output.stdout).unwrap();
            eprintln!("STDERR:");
            stderr.write_all(&output.stderr).unwrap();
        }

        std::process::exit(code);
    } else {
        eprintln!("{:?}", output);
        std::process::exit(-1);
    }
}
