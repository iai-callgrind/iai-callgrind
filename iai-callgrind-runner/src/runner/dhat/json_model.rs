use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dhat {
    #[serde(rename = "dhatFileVersion")]
    pub dhat_file_version: usize,
    pub mode: String,
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
    pub time_threshold: usize,
    #[serde(rename = "cmd")]
    pub command: PathBuf,
    pub pid: i32,
    #[serde(rename = "te")]
    pub time_end: u64,
    #[serde(rename = "tg")]
    pub time_global_max: u64,
    #[serde(rename = "pps")]
    pub program_points: Vec<ProgramPoint>,
    #[serde(rename = "ftbl")]
    pub frame_table: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramPoint {
    #[serde(rename = "tb")]
    pub total_bytes: u64,
    #[serde(rename = "tbk")]
    pub total_blocks: u64,
    #[serde(rename = "tl")]
    pub total_lifetimes: Option<u64>,
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
