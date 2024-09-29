use std::process::Command;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let exe = args.next().expect("A subprocess path should be present");

    Command::new(exe)
        .args(args)
        .status()
        .expect("Running the subprocess should succeed");
}
