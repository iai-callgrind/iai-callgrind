fn main() {
    let mut args = std::env::args().skip(1);
    let exit_code = args.next().unwrap().parse::<i32>().unwrap();

    std::process::exit(exit_code);
}
