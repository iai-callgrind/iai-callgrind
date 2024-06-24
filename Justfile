# spell-checker: ignore nofile nocapture

prettier_bin := if `command -V prettier || true` =~ 'not found' { "npx prettier" } else { "prettier" }
cspell_bin := if `command -V cspell || true` =~ 'not found' { "npx cspell" } else { "cspell" }
schema_path := 'summary.schema.json'
args := ''
msrv := '1.66.0'
required_tools := 'valgrind|the essential tool
clang|to be able to build iai-callgrind with the client-requests feature'
cargo_tools := 'cargo-hack
cargo-minimal-versions'
tools := 'docker|to be able to run the client request tests
cross|to be able to run the client request tests
cspell|check spelling
taplo|formatting of *.toml files
prettier|formatting of *.json and *.yml files'
ide_recommends := 'Depending on your IDE you can use rust-analyzer overrides to adjust clippy
execution to `cargo +stable clippy` and rustfmt execution to
`cargo +nightly rustfmt`. If possible, it is better and recommended to run the
respective `just` rules which run these tools with the best set of options
(which might change in the future, so you stay updated).'
client_request_recommends := 'If you want to start working on the client requests interface you need the
following tools installed:

* cross
* docker

to run the tests and to be able to build the client requests:

* clang >= 5

On debian based linuxes you need to install the following packages:

`apt install llvm-dev libclang-dev clang`

Please consult the requirements of the `bindgen` and `cc` crates for more
details.'
schema_gen_recommends := 'If you make changes to structs which derive `JsonSchema` you most likely need to
run `just schema-gen`. You need

* prettier

installed to be able to format the schema file. All `json` and `yaml` files in
this project are formatted with `prettier`. The pre-push hook runs
`just schema-gen-diff` to check if there are changes. Run `just schema-gen-move`
to replace the old schema file with the new one.'

# Check and fix format of rust files
fmt:
    cargo +nightly fmt --all

# Check and fix format of toml files
fmt-toml:
    taplo fmt

# Check and fix format of json and yaml files
fmt-prettier:
    {{ prettier_bin }} --write '**/*.json' '**/*.yml'

# Run all fmt rules
fmt-all: fmt fmt-toml fmt-prettier

# Check format of rust files
check-fmt:
    cargo +nightly fmt --all --check

# Check format of toml files with `taplo`
check-fmt-toml:
    taplo fmt --check --verbose

# Check format of json and yaml files
check-fmt-prettier:
    {{ prettier_bin }} --check --log-level warn '**/*.json' '**/*.yml'

# Check spelling with cspell
check-spelling:
    {{ cspell_bin }} lint .

# Run all format checkers
check-fmt-all: check-fmt check-fmt-toml check-fmt-prettier check-spelling

# Run clippy
lint:
    cargo +stable clippy --all-features --all-targets -- -D warnings

# Run cargo deny check
deny +check='all':
    cargo deny check {{ if args != '' { args } else { '' } }} {{ check }}

# Install git hooks
install-hooks:
    cp -v hooks/* .git/hooks/

# Install rust toolchains and necessary components
install-toolchains:
    rustup toolchain install stable --component clippy
    rustup toolchain install nightly --component rustfmt
    rustup toolchain install {{ msrv }} --profile default

# Show some introductory words and recommendations
show-tips:
    @echo '################################################################################'
    @echo '# RECOMMENDATIONS                                                              #'
    @echo '################################################################################'
    @echo '{{ ide_recommends }}'
    @echo
    @echo '{{ client_request_recommends }}'
    @echo
    @echo '{{ schema_gen_recommends }}'

# Check the availability of required and optional tools
install-checks:
    #!/usr/bin/env sh
    echo '################################################################################'
    echo '# CHECK AVAILABILITY OF REQUIRED AND OPTIONAL TOOLS                            #'
    echo '################################################################################'
    echo
    echo '##### Checking for required tools'
    failed=0
    while IFS='|' read -r tool reason; do if command -V $tool &>/dev/null; then
            echo "Installed: YES [$tool] ($reason)"
        else \
            echo "Installed: NO  [$tool] ($reason)"
            failed=1
    fi; done <<<$(echo "{{ required_tools }}")
    echo
    echo '##### Checking for recommended cargo tools'
    for tool in `echo -e "{{ cargo_tools }}"`; do if cargo install --list | grep -q $tool; then
            echo "Installed: YES [$tool]"
        else \
            echo "Installed: NO  [$tool]"
    fi; done
    echo
    echo '##### Checking for optional tools'
    echo "{{ tools }}" | while IFS='|' read -r tool reason; do if command -V $tool &>/dev/null; then
            echo "Installed: YES [$tool] ($reason)"
        else \
            echo "Installed: NO  [$tool] ($reason)"
    fi; done
    [ $failed -eq 1 ] && echo "!!! A required tool was not installed !!! Aborting..."
    exit $failed

# Install everything needed to start working on iai-callgrind
install-workspace: install-hooks install-toolchains show-tips install-checks

# Build iai-callgrind-runner
build-runner:
    cargo build -p iai-callgrind-runner --release

# Build the documentation
build-docs:
    DOCS_RS=1 cargo doc --all-features --no-deps --document-private-items

# A thorough build of all packages with `cargo hack` and the feature powerset
build-hack:
    cargo hack --workspace --feature-powerset build

# Delete all iai benchmarks
clean:
    rm -rf target/iai

# Run a single benchmark test
bench-test bench: build-runner
    IAI_CALLGRIND_RUNNER=$(readlink -e target/release/iai-callgrind-runner) cargo bench -p benchmark-tests --bench {{ bench }} {{ if args != '' { '-- ' + args } else { '' } }}

# Run all benchmark tests
bench-test-all: build-runner
    IAI_CALLGRIND_RUNNER=$(readlink -e target/release/iai-callgrind-runner) cargo bench -p benchmark-tests {{ if args != '' { '-- ' + args } else { '' } }}

# Note: A single benchmark may run multiple times depending on the test
#       configuration. See the `benchmark-tests/benches` folder.

# Run a single benchmark test verifying the output
full-bench-test bench:
    cargo run --package benchmark-tests --profile=bench --bin bench -- {{ bench }}

# Run all benchmark test verifying the output
full-bench-test-all:
    cargo run --package benchmark-tests --profile=bench --bin bench

# Run the json summary schema generator and format the resulting file
schema-gen:
    cargo run --package iai-callgrind-runner --release --features schema --bin schema-gen
    {{ prettier_bin }} --write {{ schema_path }}

# Run the json summary schema generator and diff the generated file with the latest schema file
schema-gen-diff: schema-gen
    diff {{ schema_path }} `find iai-callgrind-runner/schemas -iname 'summary.*.schema.json' | sort -n | tail -1` && rm {{ schema_path }}

# Run the json summary schema generator and replace the old schema file
schema-gen-move: schema-gen
    mv {{ schema_path }} `ls -1 iai-callgrind-runner/schemas/summary.*.schema.json | sort -n | tail -1`

# Run all tests in a package
test package:
    {{ if package == 'iai-callgrind' { "cargo test --package " + package + " --features ui_tests" } else { "cargo test --package " + package } }}

# Run all doc tests
test-doc:
    DOCS_RS=1 cargo test --all-features --doc

# Run only UI tests
test-ui:
    cargo test --package iai-callgrind --test ui_tests --features ui_tests

# Test all packages. This excludes client request tests which can be run separately with "just reqs-test"
test-all:
    cargo test --features ui_tests --workspace --exclude client-request-tests

# List supported targets of client request tests
reqs-test-targets:
    @sed -En 's/\[target\.([^.]+)\]/\1/p' Cross.toml

# Run the client request tests for a specific target
reqs-test target:
    @just reqs-test-targets | grep -q '{{ target }}' \
        || { echo "Unsupported target: '{{ target }}'. Run 'just reqs-test-targets' to get a list of supported targets"; exit 1; }
    CROSS_CONTAINER_OPTS='--ulimit nofile=1024:4096' cross test -p client-request-tests --test tests --target {{ target }} --release -- --nocapture

# Check minimal version requirements of dependencies
minimal-versions:
    cargo minimal-versions check --workspace --all-targets --ignore-private --direct
