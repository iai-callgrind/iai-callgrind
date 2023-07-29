use std::ffi::OsString;
use std::path::PathBuf;

mod macros;

/// A function that is opaque to the optimizer, used to prevent the compiler from
/// optimizing away computations in a benchmark.
///
/// This variant is stable-compatible, but it may cause some performance overhead
/// or fail to prevent code from being eliminated.
pub fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = std::ptr::read_volatile(&dummy);
        std::mem::forget(dummy);
        ret
    }
}

#[derive(Debug)]
pub struct Options {
    pub env_clear: bool,
    pub current_dir: Option<PathBuf>,
    pub entry_point: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    pub fn new() -> Self {
        Self {
            env_clear: true,
            current_dir: None,
            entry_point: None,
        }
    }

    pub fn env_clear(mut self, value: bool) -> Self {
        self.env_clear = value;
        self
    }

    pub fn current_dir(mut self, value: PathBuf) -> Self {
        self.current_dir = Some(value);
        self
    }

    pub fn entry_point(mut self, value: &str) -> Self {
        self.entry_point = Some(value.to_owned());
        self
    }
}

pub struct OptionsParser {
    pub options: Options,
}

impl OptionsParser {
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    pub fn into_arg(self) -> OsString {
        let mut arg = OsString::new();
        if !self.options.env_clear {
            arg.push(format!("'env_clear={}'", self.options.env_clear));
        }
        if let Some(dir) = self.options.current_dir {
            if !arg.is_empty() {
                arg.push(",");
            }
            arg.push("'current_dir=");
            arg.push(dir);
            arg.push("'");
        }
        if let Some(entry_point) = self.options.entry_point {
            if !arg.is_empty() {
                arg.push(",");
            }
            arg.push("'entry_point=");
            arg.push(entry_point);
            arg.push("'");
        }
        arg
    }

    pub fn from_arg(self, arg: &str) -> Option<Options> {
        let mut options = Options::new();
        for opt in arg.strip_prefix('\'')?.strip_suffix('\'')?.split("','") {
            match opt.split_once('=') {
                Some(("env_clear", value)) => options.env_clear = value.parse().unwrap(),
                Some(("current_dir", value)) => options.current_dir = Some(PathBuf::from(value)),
                Some(("entry_point", value)) => options.entry_point = Some(value.to_owned()),
                Some(_) | None => return None,
            }
        }
        Some(options)
    }
}
