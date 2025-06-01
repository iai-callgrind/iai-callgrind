use std::collections::HashSet;

use crate::BenchmarkId;
use crate::__internal::{
    InternalBinaryBenchmark, InternalBinaryBenchmarkBench, InternalBinaryBenchmarkConfig,
    InternalBinaryBenchmarkGroup, InternalBinaryBenchmarkGroups, InternalMacroBinBench, ModulePath,
};
use crate::error::{Error, Errors};

pub type InternalMacroBinBenches = &'static [&'static (
    &'static str,
    fn() -> Option<InternalBinaryBenchmarkConfig>,
    &'static [InternalMacroBinBench],
)];

#[derive(Debug)]
pub struct GroupsBuilder {
    groups: InternalBinaryBenchmarkGroups,
    errors: Errors,
}

impl GroupsBuilder {
    pub fn new(
        config: Option<InternalBinaryBenchmarkConfig>,
        args: Vec<String>,
        has_setup: bool,
        has_teardown: bool,
    ) -> Self {
        Self {
            groups: InternalBinaryBenchmarkGroups {
                config: config.unwrap_or_default(),
                command_line_args: args,
                has_setup,
                has_teardown,
                ..Default::default()
            },
            errors: Errors::default(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_group(
        &mut self,
        group: crate::BinaryBenchmarkGroup,
        id: String,
        module_path: &str,
        is_attribute: bool,
        config: Option<InternalBinaryBenchmarkConfig>,
        has_setup: bool,
        has_teardown: bool,
        compare_by_id: Option<bool>,
        benches: InternalMacroBinBenches,
    ) {
        let mut internal_group = InternalBinaryBenchmarkGroup {
            id,
            config,
            has_setup,
            has_teardown,
            compare_by_id,
            ..Default::default()
        };

        if is_attribute {
            Self::high_level(&mut internal_group, benches);
        } else {
            self.low_level(&mut internal_group, group, module_path);
        }

        self.groups.groups.push(internal_group);
    }

    /// Add a high-level api benchmark to the `group` parsing the `benches`
    fn high_level(group: &mut InternalBinaryBenchmarkGroup, benches: InternalMacroBinBenches) {
        for (function_name, get_config, macro_bin_benches) in benches {
            let mut internal_binary_benchmark = InternalBinaryBenchmark {
                benches: vec![],
                config: get_config(),
            };
            for macro_bin_bench in *macro_bin_benches {
                let bench = InternalBinaryBenchmarkBench {
                    id: macro_bin_bench.id_display.map(ToString::to_string),
                    args: macro_bin_bench.args_display.map(ToString::to_string),
                    function_name: (*function_name).to_owned(),
                    command: (macro_bin_bench.func)().into(),
                    config: macro_bin_bench.config.map(|f| f()),
                    has_setup: macro_bin_bench.setup.is_some(),
                    has_teardown: macro_bin_bench.teardown.is_some(),
                };
                internal_binary_benchmark.benches.push(bench);
            }
            group.binary_benchmarks.push(internal_binary_benchmark);
        }
    }

    /// Add a low-level api benchmark to the `internal_group` parsing the `group`
    ///
    /// In contrast to the high-level api, we need to check for duplicate ids, missing commands ...
    /// The errors are collected and then printed instead of returning on first error and printing
    /// errors one by one.
    fn low_level(
        &mut self,
        internal_group: &mut InternalBinaryBenchmarkGroup,
        group: crate::BinaryBenchmarkGroup,
        module_path: &str,
    ) {
        let module_path = ModulePath::new(module_path).join(&internal_group.id);

        if group.binary_benchmarks.is_empty() {
            self.errors.add(Error::new(
                &module_path,
                "This group needs at least one benchmark",
            ));
            return;
        }

        let mut binary_benchmark_ids = HashSet::<BenchmarkId>::new();
        for binary_benchmark in group.binary_benchmarks {
            let module_path = module_path.join(&binary_benchmark.id.to_string());

            if let Err(message) = binary_benchmark.id.validate() {
                self.errors.add(Error::new(&module_path, &message));
                continue;
            }
            if !binary_benchmark_ids.insert(binary_benchmark.id.clone()) {
                self.errors
                    .add(Error::new(&module_path, "Duplicate binary benchmark id"));
                continue;
            }

            let mut internal_binary_benchmark = InternalBinaryBenchmark {
                benches: vec![],
                config: binary_benchmark.config,
            };

            let mut bench_ids = HashSet::<BenchmarkId>::new();

            if binary_benchmark.benches.is_empty() {
                self.errors.add(Error::new(
                    &module_path,
                    "This binary benchmark needs at least one bench",
                ));
            }

            for bench in binary_benchmark.benches {
                let module_path = module_path.join(&bench.id.to_string());

                match bench.commands.as_slice() {
                    [] => {
                        self.errors.add(Error::new(&module_path, "Missing command"));
                    }
                    [command] => {
                        if let Err(message) = bench.id.validate() {
                            self.errors.add(Error::new(&module_path, &message));
                        }
                        if !bench_ids.insert(bench.id.clone()) {
                            self.errors.add(Error::new(
                                &module_path,
                                &format!("Duplicate id: '{}'", bench.id),
                            ));
                        }
                        let internal_bench = InternalBinaryBenchmarkBench {
                            id: Some(bench.id.into()),
                            args: None,
                            function_name: binary_benchmark.id.clone().into(),
                            command: command.into(),
                            config: bench.config.clone(),
                            has_setup: bench.setup.is_some() || binary_benchmark.setup.is_some(),
                            has_teardown: bench.teardown.is_some()
                                || binary_benchmark.teardown.is_some(),
                        };
                        internal_binary_benchmark.benches.push(internal_bench);
                    }
                    commands => {
                        for (index, command) in commands.iter().enumerate() {
                            let bench_id: BenchmarkId = format!("{}_{}", bench.id, index).into();
                            if let Err(message) = bench_id.validate() {
                                self.errors.add(Error::new(&module_path, &message));
                                continue;
                            }
                            if !bench_ids.insert(bench_id.clone()) {
                                self.errors.add(Error::new(
                                    &module_path,
                                    &format!("Duplicate id: '{bench_id}'"),
                                ));
                                continue;
                            }
                            let internal_bench = InternalBinaryBenchmarkBench {
                                id: Some(bench_id.into()),
                                args: None,
                                function_name: binary_benchmark.id.to_string(),
                                command: command.into(),
                                config: bench.config.clone(),
                                has_setup: bench.setup.is_some()
                                    || binary_benchmark.setup.is_some(),
                                has_teardown: bench.teardown.is_some()
                                    || binary_benchmark.teardown.is_some(),
                            };
                            internal_binary_benchmark.benches.push(internal_bench);
                        }
                    }
                }
            }

            internal_group
                .binary_benchmarks
                .push(internal_binary_benchmark);
        }
    }

    pub fn build(self) -> Result<InternalBinaryBenchmarkGroups, Errors> {
        if self.errors.is_empty() {
            Ok(self.groups)
        } else {
            Err(self.errors)
        }
    }
}
