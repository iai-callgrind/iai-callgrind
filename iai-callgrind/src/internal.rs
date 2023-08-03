//! This module exists only for internal usage and does not contain structures which are directly
//! usable in the main! macro.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

use crate::Options;

#[derive(Debug, Serialize, Deserialize)]
pub struct BinaryBenchmark {
    pub sandbox: bool,
    pub fixtures: Option<Fixtures>,
    pub assists: Vec<Assistant>,
    pub options: Vec<String>,
    pub runs: Vec<Run>,
}

impl Default for BinaryBenchmark {
    fn default() -> Self {
        Self {
            sandbox: true,
            fixtures: Option::default(),
            assists: Vec::default(),
            options: Vec::default(),
            runs: Vec::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Assistant {
    pub id: String,
    pub name: String,
    pub bench: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub cmd: String,
    pub args: Vec<Vec<String>>,
    pub opts: Option<Options>,
    pub envs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixtures {
    pub path: String,
    pub follow_symlinks: bool,
}
