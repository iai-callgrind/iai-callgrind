//! A utility binary to create the json schema for the summary.json file
//!
//! This binary is not considered a part of the published `iai-callgrind-runner` package and is only
//! used during the development of `iai-callgrind`.
use std::fs::File;

use iai_callgrind_runner::runner::summary::BenchmarkSummary;
use schemars::generate::SchemaSettings;

fn main() {
    let generator = SchemaSettings::draft07().into_generator();
    serde_json::to_writer_pretty(
        File::create("summary.schema.json").unwrap(),
        &generator.into_root_schema_for::<BenchmarkSummary>(),
    )
    .expect("Schema creation should be successful");
}
