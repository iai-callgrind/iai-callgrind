fn main() {
    let status = std::env::args().nth(1).expect("Exit status");
    let code = status.parse::<i32>().expect("Valid status code");
    std::process::exit(code);
}
