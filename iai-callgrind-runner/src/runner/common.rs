use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::api;
use crate::util::truncate_str_utf8;

pub struct ToolOutputPath {
    pub tool: ValgrindTool,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValgrindTool {
    Callgrind,
    Memcheck,
    Helgrind,
    DRD,
    Massif,
    DHAT,
    BBV,
}

impl ToolOutputPath {
    pub fn new(tool: ValgrindTool, base_dir: &Path, module: &str, name: &str) -> Self {
        let current = base_dir;
        let module_path: PathBuf = module.split("::").collect();
        let sanitized_name = sanitize_filename::sanitize_with_options(
            name,
            sanitize_filename::Options {
                windows: false,
                truncate: false,
                replacement: "_",
            },
        );
        let file_name = PathBuf::from(format!(
            "{}.{}.out",
            // callgrind. + .out.old = 18 + 37 bytes headroom for extensions with more than 3
            // bytes. max length is usually 255 bytes
            tool.id(),
            truncate_str_utf8(&sanitized_name, 200)
        ));

        let path = current.join(base_dir).join(module_path).join(file_name);
        Self { tool, path }
    }

    pub fn from_existing<T>(tool: ValgrindTool, path: T) -> Result<Self>
    where
        T: Into<PathBuf>,
    {
        let path: PathBuf = path.into();
        if !path.is_file() {
            return Err(anyhow!(
                "The callgrind output file '{}' did not exist or is not a valid file",
                path.display()
            ));
        }
        Ok(Self { tool, path })
    }

    /// Initialize and create the output directory and organize files
    ///
    /// This method moves the old output to `$TOOL_ID.*.out.old`
    /// TODO: RETURN Result
    pub fn with_init(tool: ValgrindTool, base_dir: &Path, module: &str, name: &str) -> Self {
        let output = Self::new(tool, base_dir, module, name);
        output.init();
        output
    }

    // TODO: RETURN Result
    // TODO: MOVE all output files in case of a modifier
    pub fn init(&self) {
        std::fs::create_dir_all(self.path.parent().unwrap()).expect("Failed to create directory");

        if self.exists() {
            let old_output = self.to_old_output();
            // Already run this benchmark once; move last results to .old
            std::fs::copy(&self.path, old_output.path).unwrap();
        }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn with_extension<T>(&self, extension: T) -> Self
    where
        T: AsRef<OsStr>,
    {
        Self {
            tool: self.tool,
            path: self.path.with_extension(extension),
        }
    }

    pub fn to_old_output(&self) -> Self {
        Self {
            tool: self.tool,
            path: self.path.with_extension("out.old"),
        }
    }

    pub fn to_tool_output(&self, tool: ValgrindTool) -> Self {
        let file_name: &str = std::str::from_utf8(
            self.path
                .file_name()
                .unwrap()
                .as_bytes()
                .strip_prefix(self.tool.id().as_bytes())
                .unwrap(),
        )
        .unwrap();
        let path = self
            .path
            .with_file_name(format!("{}{file_name}", tool.id()));
        Self { tool, path }
    }

    pub fn open(&self) -> Result<File> {
        File::open(&self.path).with_context(|| {
            format!(
                "Error opening callgrind output file '{}'",
                self.path.display()
            )
        })
    }

    pub fn lines(&self) -> Result<impl Iterator<Item = String>> {
        let file = self.open()?;
        Ok(BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap))
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }
}

impl Display for ToolOutputPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path.display()))
    }
}

impl ValgrindTool {
    /// Return the id used by the `valgrind --tool` option
    pub fn id(&self) -> String {
        match self {
            ValgrindTool::DHAT => "dhat".to_owned(),
            ValgrindTool::Callgrind => "callgrind".to_owned(),
            ValgrindTool::Memcheck => "memcheck".to_owned(),
            ValgrindTool::Helgrind => "helgrind".to_owned(),
            ValgrindTool::DRD => "drd".to_owned(),
            ValgrindTool::Massif => "massif".to_owned(),
            ValgrindTool::BBV => "exp-bbv".to_owned(),
        }
    }

    pub fn has_output_file(&self) -> bool {
        matches!(
            self,
            ValgrindTool::Callgrind | ValgrindTool::DHAT | ValgrindTool::BBV | ValgrindTool::Massif
        )
    }
}

impl From<api::ValgrindTool> for ValgrindTool {
    fn from(value: api::ValgrindTool) -> Self {
        match value {
            api::ValgrindTool::Memcheck => ValgrindTool::Memcheck,
            api::ValgrindTool::Helgrind => ValgrindTool::Helgrind,
            api::ValgrindTool::DRD => ValgrindTool::DRD,
            api::ValgrindTool::Massif => ValgrindTool::Massif,
            api::ValgrindTool::DHAT => ValgrindTool::DHAT,
            api::ValgrindTool::BBV => ValgrindTool::BBV,
        }
    }
}
