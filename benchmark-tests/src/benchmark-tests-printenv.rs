fn main() {
    for arg in std::env::args().skip(1) {
        let (key, value) = match arg.split_once('=') {
            Some((key, value)) => {
                let actual_value =
                    std::env::var(key).expect("Environment variable must be present");
                assert_eq!(&actual_value, value, "Environment variable value differs");
                (key, actual_value)
            }
            None => {
                let value =
                    std::env::var(&arg).expect("Pass-through environment variable must be present");
                (arg.as_str(), value)
            }
        };
        println!("{key}={value}");
    }
}
