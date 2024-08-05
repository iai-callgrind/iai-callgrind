use std::io::{stdin, Read};

fn main() {
    let mut stdin = stdin().lock();

    let mut content = vec![];
    stdin.read_to_end(&mut content).unwrap();

    println!("STDIN was: '{}'", String::from_utf8_lossy(&content));
}
