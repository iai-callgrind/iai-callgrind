fn main() {
    if cfg!(feature = "client_requests") {
        println!("cargo:rerun-if-changed=valgrind/wrapper.c");
        cc::Build::new()
            .file("valgrind/wrapper.c")
            .compile("wrapper");
    }
}
