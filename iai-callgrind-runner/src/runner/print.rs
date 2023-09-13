use std::fmt::{Display, Write};

use colored::Colorize;

use crate::util::truncate_str_utf8;

pub struct Header {
    pub module_path: String,
    pub id: Option<String>,
    pub description: Option<String>,
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

    pub fn to_title(&self) -> String {
        let mut output = String::new();
        write!(&mut output, "{}", self.module_path).unwrap();
        if let Some(id) = &self.id {
            if let Some(description) = &self.description {
                let truncated = truncate_str_utf8(description, 37);
                write!(
                    &mut output,
                    " {id}:{truncated}{}",
                    if truncated.len() < description.len() {
                        "..."
                    } else {
                        ""
                    }
                )
                .unwrap();
            } else {
                write!(&mut output, " {id}").unwrap();
            }
        }
        output
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
