fn main() {
    let joined = std::env::args().skip(1).collect::<Vec<String>>().join(" ");
    println!("{joined}");
}
