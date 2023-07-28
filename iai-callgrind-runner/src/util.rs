use std::ffi::{OsStr, OsString};

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
