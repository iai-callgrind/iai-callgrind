//! The build script

fn main() {
    println!(
        "cargo:rustc-env=IC_BUILD_TRIPLE={}",
        std::env::var("TARGET").unwrap()
    );
}
