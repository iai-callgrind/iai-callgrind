// spell-checker: ignore rustified iquote iquotevalgrind

#[cfg(feature = "client_requests_defs")]
mod imp {
    use std::borrow::Cow;
    use std::ffi::OsString;
    use std::fmt::Display;
    use std::io::{BufRead, BufReader, Cursor};
    use std::path::PathBuf;

    use bindgen::{builder, Bindings};
    use strum::{EnumIter, IntoEnumIterator};
    use version_compare::Cmp;

    #[derive(Debug)]
    struct Target {
        arch: String,
        env: String,
        os: String,
        vendor: String,
        triple: String,
    }

    #[derive(EnumIter, Debug, PartialEq, Eq)]
    enum Support {
        Arm,
        Aarch64,
        X86,
        X86_64,
        Native,
        No,
    }

    impl Display for Support {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let support = format!("{:?}", self).to_lowercase();
            f.write_str(&support)
        }
    }

    impl Target {
        fn from_env() -> Self {
            Self {
                arch: std::env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
                env: std::env::var("CARGO_CFG_TARGET_ENV").unwrap(),
                os: std::env::var("CARGO_CFG_TARGET_OS").unwrap(),
                vendor: std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap(),
                triple: std::env::var("TARGET").unwrap(),
            }
        }
    }

    // Return the rust version if running rustc was successful
    fn get_rust_version() -> Option<String> {
        let output = std::process::Command::new(
            std::env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc")),
        )
        .arg("--version")
        .output();

        output.ok().map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split(' ')
                .nth(1)
                .expect("The rust version should be present")
                .to_string()
        })
    }

    fn print_client_requests_support(value: &Support) {
        println!("cargo:rustc-cfg=client_requests_support=\"{value}\"");
    }

    fn include_dirs(target: &Target) -> impl Iterator<Item = String> {
        [
            Cow::Owned(format!(
                "IAI_CALLGRIND_{}_VALGRIND_INCLUDE",
                target.triple.replace('-', "_").to_ascii_uppercase()
            )),
            Cow::Borrowed("IAI_CALLGRIND_VALGRIND_INCLUDE"),
        ]
        .into_iter()
        .filter_map(|env| std::env::var(env.as_ref()).ok())
    }

    fn build_native(target: &Target) {
        let mut builder = cc::Build::new();

        for env in include_dirs(target) {
            builder.include(env);
        }

        if target.os == "freebsd" {
            builder.include("/usr/local/include");
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

    fn build_bindings(target: &Target) -> Bindings {
        let mut builder = builder();

        for env in include_dirs(target) {
            builder = builder.clang_arg(format!("-iquote{env}"))
        }

        if let Ok(env) = std::env::var("IAI_CALLGRIND_CROSS_TARGET") {
            let path = PathBuf::from("/valgrind/target/valgrind")
                .join(env)
                .join("include");
            builder = builder.clang_arg(format!("-iquote{}", path.display()))
        }

        if target.os == "freebsd" {
            builder = builder.clang_arg("-iquote/usr/local/include");
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
        println!("cargo:rerun-if-env-changed=IAI_CALLGRIND_VALGRIND_INCLUDE");
        println!("cargo:rerun-if-env-changed=IAI_CALLGRIND_CROSS_TARGET");
        println!("cargo:rerun-if-env-changed=TARGET");

        // rustc-check-cfg is introduced in rust with version 1.80 and avoids the compiler warnings
        // in version >= 1.80.0. Printing it when compiling with versions < 1.80 triggers a warning,
        // too. To get the best of both worlds we check against the currently active rust version.
        if let Some(rust_version) = get_rust_version() {
            if version_compare::compare_to(rust_version, "1.80", Cmp::Ge).unwrap() {
                let values = Support::iter()
                    .map(|s| format!("\"{s}\""))
                    .collect::<Vec<String>>()
                    .join(",");
                println!("cargo:rustc-check-cfg=cfg(client_requests_support,values({values}))");
            }
        }

        let target = Target::from_env();

        // When building the docs on docs.rs we can take a shortcut
        if std::env::var("DOCS_RS").is_ok() {
            print_client_requests_support(&Support::X86_64);
            build_bindings(&target);
            build_native(&target);
            return;
        }

        let bindings = build_bindings(&target);

        // These guards mirror the checks in the `valgrind.h` header file
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
        } else if target.arch == "aarch64"
            && (target.os == "freebsd" || (target.os == "linux" && target.env == "gnu"))
        {
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

        if let Some(support) = support {
            print_client_requests_support(&support);
            if support != Support::No {
                build_native(&target);
            }
        } else {
            eprintln!("{bindings}");
            panic!("Unable to set cfg value for client_requests_support");
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
