use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    GroupError(String, String, String),
    BinaryBenchmarkError(String, String, String, String),
    BenchError(String, String, String, String, String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::GroupError(module_path, id, message) => {
                f.write_fmt(format_args!("Error in {module_path}::{id}: {message}"))
            }
            Error::BinaryBenchmarkError(module_path, group_id, binary_benchmark_id, message) => f
                .write_fmt(format_args!(
                    "Error in {module_path}::{group_id}::{binary_benchmark_id}: {message}"
                )),
            Error::BenchError(module_path, group_id, binary_benchmark_id, bench_id, message) => f
                .write_fmt(format_args!(
                    "Error in {module_path}::{group_id}::{binary_benchmark_id}::{bench_id}: \
                     {message}"
                )),
        }
    }
}

#[derive(Debug, Default)]
pub struct Errors(Vec<Error>);

impl Errors {
    pub fn add(&mut self, error: Error) {
        self.0.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Error in at least one benchmark group: The following errors occurred:\n")?;

        for error in &self.0 {
            f.write_fmt(format_args!("  {error}\n"))?;
        }

        Ok(())
    }
}
