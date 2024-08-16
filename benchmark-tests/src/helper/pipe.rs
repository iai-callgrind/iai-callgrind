use std::io::{stdin, stdout, Read, Write};

fn main() {
    let mut stdin = stdin().lock();

    let mut content = vec![];
    stdin.read_to_end(&mut content).unwrap();

    println!("STDIN was:");
    stdout().write_all(&content).unwrap();
}
