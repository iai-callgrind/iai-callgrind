use std::fmt::Display;

use colored::Colorize;

use crate::util::truncate_str_utf8;

pub struct Header {
    module_path: String,
    id: Option<String>,
    description: Option<String>,
}

impl Header {
    pub fn new<T, U, V>(module_path: T, id: U, description: V) -> Self
    where
        T: Into<String>,
        U: Into<Option<String>>,
        V: Into<Option<String>>,
    {
        Self {
            module_path: module_path.into(),
            id: id.into(),
            description: description.into(),
        }
    }

    pub fn from_segments<I, T, U, V>(module_path: T, id: U, description: V) -> Self
    where
        I: AsRef<str>,
        T: AsRef<[I]>,
        U: Into<Option<String>>,
        V: Into<Option<String>>,
    {
        Self {
            module_path: module_path
                .as_ref()
                .iter()
                .map(|s| s.as_ref().to_owned())
                .collect::<Vec<String>>()
                .join("::"),
            id: id.into(),
            description: description.into(),
        }
    }

    pub fn print(&self) {
        println!("{self}");
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.module_path.green()))?;
        if let Some(id) = &self.id {
            if let Some(description) = &self.description {
                let truncated = truncate_str_utf8(description, 37);
                f.write_fmt(format_args!(
                    " {}{}{}{}",
                    id.cyan(),
                    ":".cyan(),
                    truncated.bold().blue(),
                    if truncated.len() < description.len() {
                        "..."
                    } else {
                        ""
                    }
                ))?;
            } else {
                f.write_fmt(format_args!(" {}", id.cyan()))?;
            }
        }
        Ok(())
    }
}
