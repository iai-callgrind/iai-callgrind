pub fn bubble_sort(mut array: Vec<i32>) -> Vec<i32> {
    for i in 0..array.len() {
        for j in 0..array.len() - i - 1 {
            if array[j + 1] < array[j] {
                array.swap(j, j + 1);
            }
        }
    }
    array
}

pub fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

pub fn print_env(args: &[&str]) {
    for arg in args {
        let (key, value) = match arg.split_once('=') {
            Some((key, value)) => {
                let actual_value =
                    std::env::var(key).expect("Environment variable must be present");
                assert_eq!(&actual_value, value, "Environment variable value differs");
                (key.to_owned(), actual_value)
            }
            None => {
                let value =
                    std::env::var(arg).expect("Pass-through environment variable must be present");
                (arg.to_string(), value)
            }
        };
        println!("{key}={value}");
    }
}
