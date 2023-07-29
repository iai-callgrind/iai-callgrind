use std::ffi::{OsStr, OsString};
use std::io::{self, BufWriter, Write};

pub fn join_os_string(slice: &[OsString], sep: &OsStr) -> OsString {
    if let Some((first, suffix)) = slice.split_first() {
        suffix.iter().fold(first.to_owned(), |mut a, b| {
            a.push(sep);
            a.push(b);
            a
        })
    } else {
        OsString::new()
    }
}

pub fn concat_os_string<T: AsRef<OsStr>>(mut first: OsString, second: T) -> OsString {
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

pub fn yesno_to_bool(value: &str) -> bool {
    value == "yes"
}

fn trim(bytes: &[u8]) -> &[u8] {
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
    let stdout = io::stdout();
    let stdout = stdout.lock();
    let mut writer = BufWriter::new(stdout);
    writer
        .write_all(trim(bytes))
        .and_then(|_| writer.flush())
        .unwrap();
}

pub fn write_all_to_stderr(bytes: &[u8]) {
    let stderr = io::stderr();
    let stderr = stderr.lock();
    let mut writer = BufWriter::new(stderr);
    writer
        .write_all(trim(bytes))
        .and_then(|_| writer.flush())
        .unwrap();
}
