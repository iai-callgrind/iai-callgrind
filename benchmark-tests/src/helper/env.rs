fn main() {
    let mut args = std::env::args().skip(1);
    let next = args.next().unwrap();
    if next == "--check" {
        for arg in args {
            let (key, value) = arg.split_once("=").unwrap();
            assert_eq!(std::env::var(key).unwrap(), value);
            println!("Found env: '{key}' with value '{value}'")
        }
    } else if next == "--is-cleared=true" {
        assert!(!std::env::vars().any(|(key, _)| key == "PATH"));
        println!("The environment variables look like they have been cleared");
    } else if next == "--is-cleared=false" {
        assert!(std::env::vars().any(|(key, _)| key == "PATH"));
        println!("The environment variables look like they have not been cleared");
    } else {
        panic!("Invalid argument: '{next}'");
    }
}
