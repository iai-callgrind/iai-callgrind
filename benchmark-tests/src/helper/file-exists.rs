use std::path::PathBuf;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let path = PathBuf::from(args.next().unwrap());
    let true_or_false = args.next().unwrap();

    let expected_exists = match true_or_false.to_string_lossy().to_string().as_str() {
        "true" => true,
        "false" => false,
        value => panic!("Unexpected value: '{value}'"),
    };

    assert_eq!(path.exists(), expected_exists);
}
