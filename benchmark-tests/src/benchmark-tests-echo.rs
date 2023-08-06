fn main() -> Result<(), std::io::Error> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let fixture = args.pop().unwrap();
    let expected = std::fs::read_to_string(fixture).unwrap();

    for (num, line) in expected.lines().enumerate() {
        let actual = &args[num];
        assert_eq!(actual, line);
        println!("{}", actual);
    }

    Ok(())
}
