use crate::ValgrindTool;

type MacroLibBenches<'a> = &'a [&'a (
    &'static str,
    fn() -> Option<crate::__internal::InternalLibraryBenchmarkConfig>,
    &'a [crate::__internal::InternalMacroLibBench],
)];

pub struct GroupsBuilder(crate::__internal::InternalLibraryBenchmarkGroups);

impl GroupsBuilder {
    #[cfg(feature = "cachegrind")]
    pub fn new(
        config: Option<crate::__internal::InternalLibraryBenchmarkConfig>,
        args: Vec<String>,
        has_setup: bool,
        has_teardown: bool,
    ) -> Self {
        Self(crate::__internal::InternalLibraryBenchmarkGroups {
            config: config.unwrap_or_default(),
            groups: Vec::default(),
            command_line_args: args,
            has_setup,
            has_teardown,
            default_tool: ValgrindTool::Cachegrind,
        })
    }

    #[cfg(not(feature = "cachegrind"))]
    pub fn new(
        config: Option<crate::__internal::InternalLibraryBenchmarkConfig>,
        args: Vec<String>,
        has_setup: bool,
        has_teardown: bool,
    ) -> Self {
        Self(crate::__internal::InternalLibraryBenchmarkGroups {
            config: config.unwrap_or_default(),
            groups: Vec::default(),
            command_line_args: args,
            has_setup,
            has_teardown,
            default_tool: ValgrindTool::Callgrind,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_group(
        &mut self,
        id: String,
        config: Option<crate::__internal::InternalLibraryBenchmarkConfig>,
        compare_by_id: Option<bool>,
        has_setup: bool,
        has_teardown: bool,
        benches: MacroLibBenches,
    ) {
        let mut internal_group = crate::__internal::InternalLibraryBenchmarkGroup {
            id,
            config,
            has_setup,
            has_teardown,
            compare_by_id,
            ..Default::default()
        };

        for (function_name, get_config, macro_lib_benches) in benches {
            let mut benches = crate::__internal::InternalLibraryBenchmarkBenches {
                benches: vec![],
                config: get_config(),
            };
            for macro_lib_bench in *macro_lib_benches {
                let bench = crate::__internal::InternalLibraryBenchmarkBench {
                    id: macro_lib_bench.id_display.map(ToString::to_string),
                    args: macro_lib_bench.args_display.map(ToString::to_string),
                    function_name: (*function_name).to_owned(),
                    config: macro_lib_bench.config.map(|f| f()),
                    iter_count: match macro_lib_bench.func {
                        super::InternalFunctionKind::Iter(func) => Some(func(None)),
                        super::InternalFunctionKind::Default(_) => None,
                    },
                };
                benches.benches.push(bench);
            }
            internal_group.library_benchmarks.push(benches);
        }

        self.0.groups.push(internal_group);
    }

    pub fn build(self) -> crate::__internal::InternalLibraryBenchmarkGroups {
        self.0
    }
}
