// spell-checker: ignore rustified iquote

#[cfg(feature = "client_requests_defs")]
use std::io::{BufRead, BufReader, Cursor};
#[cfg(feature = "client_requests_defs")]
use std::path::PathBuf;

#[cfg(feature = "client_requests_defs")]
use bindgen::builder;

#[cfg(feature = "client_requests_defs")]
struct Target {
    arch: String,
    env: String,
    os: String,
    vendor: String,
}

#[cfg(feature = "client_requests_defs")]
enum Support {
    X86,
    X86_64,
    Native,
    No,
}

#[cfg(feature = "client_requests_defs")]
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

#[cfg(feature = "client_requests_defs")]
fn print_client_requests_support(value: &str) {
    println!("cargo:rustc-cfg=client_requests_support=\"{value}\"");
}

#[cfg(not(feature = "client_requests_defs"))]
fn main() {}

#[cfg(feature = "client_requests_defs")]
fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }

    println!("cargo:rerun-if-changed=valgrind/wrapper.h");
    println!("cargo:rerun-if-changed=valgrind/native.c");

    let mut builder = builder();

    if let Ok(env) = std::env::var("IAI_CALLGRIND_VALGRIND_INCLUDE") {
        builder = builder.clang_arg(format!("-iquote{env}"))
    }

    let bindings = builder
        .clang_arg("-iquote/usr/include")
        .header("valgrind/wrapper.h")
        .allowlist_var("IC_IS_PLATFORM_SUPPORTED_BY_VALGRIND")
        // valgrind.h
        .allowlist_var("IC_VALGRIND_MAJOR")
        .allowlist_var("IC_VALGRIND_MINOR")
        .allowlist_type("IC_ValgrindClientRequest")
        .rustified_enum("IC_ValgrindClientRequest")
        // callgrind.h
        .allowlist_type("IC_CallgrindClientRequest")
        .rustified_enum("IC_CallgrindClientRequest")
        // dhat.h
        .allowlist_type("IC_DHATClientRequest")
        .rustified_enum("IC_DHATClientRequest")
        // memcheck.h
        .allowlist_type("IC_MemcheckClientRequest")
        .rustified_enum("IC_MemcheckClientRequest")
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Generating binding should succeed");

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
    } else {
        let reader = BufReader::new(Cursor::new(bindings.to_string()));
        let mut support = None;
        for line in reader.lines().map(Result::unwrap) {
            // The bindings are formatted, so we can expect a strict format of the
            // `IS_PLATFORM_SUPPORTED_BY_VALGRIND` variable
            if let Some(suffix) = line
                .trim()
                .strip_prefix("pub const IC_IS_PLATFORM_SUPPORTED_BY_VALGRIND: bool =")
            {
                let suffix = suffix.trim();
                if suffix == "false;" {
                    support = Some(Support::No);
                } else if suffix == "true;" {
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
        }
        Some(Support::X86) => {
            print_client_requests_support("x86");
        }
        Some(Support::Native) => {
            print_client_requests_support("native");

            let mut builder = cc::Build::new();
            if let Ok(env) = std::env::var("IAI_CALLGRIND_VALGRIND_INCLUDE") {
                builder.include(env);
            }
            builder.file("valgrind/native.c").compile("native");
        }
        Some(Support::No) => {
            print_client_requests_support("no");
        }
        None => {
            eprintln!("{bindings}");
            panic!("Unable to set cfg value for client_requests_support");
        }
    }

    // Write the generated bindings to an output file.
    let out_dir = std::env::var("OUT_DIR").map(PathBuf::from).unwrap();
    let path = out_dir.join("bindings.rs");
    bindings.write_to_file(path).unwrap();
}
