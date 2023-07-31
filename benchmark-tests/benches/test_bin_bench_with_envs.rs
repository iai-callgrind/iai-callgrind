use iai_callgrind::main;

#[export_name = "__iai::printenv"]
#[inline(never)]
fn print_env() {
    for (key, value) in std::env::vars() {
        println!("{key}={value}")
    }
}

#[inline(never)]
fn before() {
    print_env();
}

main!(
    options = "--toggle-collect=benchmark_tests_printenv::main", "--toggle-collect=__iai::printenv";
    before = before, bench = true;
    run = cmd = "benchmark-tests-printenv", envs = ["PATH"], args = ["PATH", "LD_PRELOAD"];
    run = cmd = "benchmark-tests-printenv", envs = ["PATH"], opts = Options::default().env_clear(true), args = ["PATH", "LD_PRELOAD"];
    run = cmd = "benchmark-tests-printenv", envs = ["HELLO=WORLD"], opts = Options::default().env_clear(true), args = ["HELLO=WORLD", "LD_PRELOAD"];
);
