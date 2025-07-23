use benchmark_tests::leak_memory;

fn main() {
    let mut args = std::env::args().skip(1);
    let num = args.next().unwrap().parse::<usize>().unwrap();

    leak_memory(num);
}
