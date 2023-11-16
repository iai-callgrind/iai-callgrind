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
