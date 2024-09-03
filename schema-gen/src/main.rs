//! A utility binary to create the json schema for the summary.json file
//!
//! This binary is not considered a part of the published `iai-callgrind-runner` package and is only
//! used during the development of `iai-callgrind`.
use std::fs::File;

use iai_callgrind_runner::runner::summary::BenchmarkSummary;
use schemars::schema_for;

fn main() {
    serde_json::to_writer_pretty(
        File::create("summary.schema.json").unwrap(),
        &schema_for!(BenchmarkSummary),
    )
    .expect("Schema creation should be successful");
}
