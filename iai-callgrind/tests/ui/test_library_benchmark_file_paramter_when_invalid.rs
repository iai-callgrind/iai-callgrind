use iai_callgrind::library_benchmark;

#[library_benchmark]
#[benches::my_id(file = "iai-callgrind/tests/fixtures/empty.fix")]
fn bench10(value: String) -> u64 {
    value.parse::<u64>().unwrap()
}

#[library_benchmark]
#[benches::my_id(file = "iai-callgrind/tests/fixtures/does_not_exist")]
fn bench20(value: String) -> u64 {
    value.parse::<u64>().unwrap()
}

#[library_benchmark]
#[benches::my_id(file = "iai-callgrind/tests/fixtures/invalid-utf8.fix")]
fn bench30(value: String) -> u64 {
    value.parse::<u64>().unwrap()
}

// both parameters are valid but specifying both is invalid
#[library_benchmark]
#[benches::my_id(file = "iai-callgrind/tests/fixtures/numbers.fix", args = [("valid_arg".to_owned()), "another".to_owned()])]
fn bench40(value: String) -> u64 {
    value.parse::<u64>().unwrap()
}

#[library_benchmark]
#[benches::my_id(file = "iai-callgrind/tests/fixtures/numbers.fix")]
fn bench50(value: u64) -> String {
    value.to_string()
}

#[library_benchmark]
#[benches::my_id(file = ("iai-callgrind/tests/fixtures/numbers.fix", String))]
fn bench60(value: String) -> u64 {
    value.parse::<u64>().unwrap()
}

#[library_benchmark]
#[benches::my_id(file = String::from("iai-callgrind/tests/fixtures/numbers.fix"))]
fn bench70(value: String) -> u64 {
    value.parse::<u64>().unwrap()
}

fn main() {}
