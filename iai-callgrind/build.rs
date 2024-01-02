// spell-checker: ignore rustified iquote iquotevalgrind

#[cfg(feature = "client_requests_defs")]
mod imp {
    use std::io::{BufRead, BufReader, Cursor};
    use std::path::PathBuf;

    use bindgen::{builder, Bindings};
    #[derive(Debug)]
    struct Target {
        arch: String,
        env: String,
        os: String,
        vendor: String,
    }

    #[derive(Debug)]
    enum Support {
        Arm,
        Aarch64,
        X86,
        X86_64,
        Native,
        No,
    }

    impl Target {
        fn from_env() -> Self {
            Self {
                arch: std::env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
                env: std::env::var("CARGO_CFG_TARGET_ENV").unwrap(),
                os: std::env::var("CARGO_CFG_TARGET_OS").unwrap(),
                vendor: std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap(),
            }
        }
    }

    fn print_client_requests_support(value: &str) {
        println!("cargo:rustc-cfg=client_requests_support=\"{value}\"");
    }

    fn build_native() {
        let mut builder = cc::Build::new();
        if let Ok(env) = std::env::var("IAI_CALLGRIND_VALGRIND_INCLUDE") {
            builder.include(env);
        }
        if let Ok(env) = std::env::var("IAI_CALLGRIND_CROSS_TARGET") {
            let path = PathBuf::from("/valgrind/target/valgrind")
                .join(env)
                .join("include");
            builder.include(path);
        }
        builder.include("valgrind/include");

        builder
            .debug(true)
            .file("valgrind/native.c")
            .compile("native");
    }

    fn build_bindings() -> Bindings {
        let mut builder = builder();

        if let Ok(env) = std::env::var("IAI_CALLGRIND_VALGRIND_INCLUDE") {
            builder = builder.clang_arg(format!("-iquote{env}"))
        }

        if let Ok(env) = std::env::var("IAI_CALLGRIND_CROSS_TARGET") {
            let path = PathBuf::from("/valgrind/target/valgrind")
                .join(env)
                .join("include");
            builder = builder.clang_arg(format!("-iquote{}", path.display()))
        }

        let bindings = builder
            .clang_arg("-iquote/usr/include")
            .clang_arg("-iquotevalgrind/include")
            .header("valgrind/wrapper.h")
            .allowlist_var("IC_IS_PLATFORM_SUPPORTED_BY_VALGRIND")
            .allowlist_var("IC_VALGRIND_MAJOR")
            .allowlist_var("IC_VALGRIND_MINOR")
            .allowlist_type("IC_.*ClientRequest")
            .rustified_enum("IC_.*ClientRequest")
            .layout_tests(false)
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate()
            .expect("Generating binding should succeed");

        let out_dir = std::env::var("OUT_DIR").map(PathBuf::from).unwrap();
        let path = out_dir.join("bindings.rs");
        bindings.write_to_file(path).unwrap();
        bindings
    }

    pub fn main() {
        println!("cargo:rerun-if-changed=valgrind/wrapper.h");
        println!("cargo:rerun-if-changed=valgrind/native.c");

        if std::env::var("DOCS_RS").is_ok() {
            print_client_requests_support("x86_64");
            build_bindings();
            build_native();
            return;
        }

        let bindings = build_bindings();

        let target = Target::from_env();
        let support = if target.arch == "x86_64"
            && (target.os == "linux"
                || target.os == "freebsd"
                || (target.vendor == "apple" && target.os == "darwin")
                || (target.os == "windows" && target.env == "gnu")
                || ((target.vendor == "sun") || target.vendor == "pc") && target.os == "solaris")
        {
            Some(Support::X86_64)
        } else if target.arch == "x86"
            && (target.os == "linux"
                || target.os == "freebsd"
                || (target.vendor == "apple" && target.os == "darwin")
                || (target.os == "windows" && target.env == "gnu")
                || ((target.vendor == "sun") || target.vendor == "pc") && target.os == "solaris")
        {
            Some(Support::X86)
        } else if target.arch == "arm" && target.os == "linux" && target.env == "gnu" {
            Some(Support::Arm)
        } else if target.arch == "aarch64" && target.os == "linux" && target.env == "gnu" {
            Some(Support::Aarch64)
        } else {
            let re = regex::Regex::new(
                r"IC_IS_PLATFORM_SUPPORTED_BY_VALGRIND.*?=\s*(?<value>true|false)",
            )
            .expect("Regex should compile");
            let reader = BufReader::new(Cursor::new(bindings.to_string()));
            let mut support = None;
            for line in reader.lines().map(Result::unwrap) {
                if let Some(caps) = re.captures(&line) {
                    let value = caps.name("value").unwrap().as_str();
                    if value == "false" {
                        support = Some(Support::No);
                    } else if value == "true" {
                        support = Some(Support::Native);
                    } else {
                        // do nothing
                    }
                    break;
                }
            }
            support
        };

        match support {
            Some(Support::X86_64) => {
                print_client_requests_support("x86_64");
                build_native();
            }
            Some(Support::X86) => {
                print_client_requests_support("x86");
                build_native();
            }
            Some(Support::Arm) => {
                print_client_requests_support("arm");
                build_native();
            }
            Some(Support::Aarch64) => {
                print_client_requests_support("aarch64");
                build_native();
            }
            Some(Support::Native) => {
                print_client_requests_support("native");
                build_native();
            }
            Some(Support::No) => {
                print_client_requests_support("no");
            }
            None => {
                eprintln!("{bindings}");
                panic!("Unable to set cfg value for client_requests_support");
            }
        }
    }
}

#[cfg(not(feature = "client_requests_defs"))]
mod imp {
    pub fn main() {}
}

fn main() {
    imp::main();
}
