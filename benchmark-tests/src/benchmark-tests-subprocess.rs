use benchmark_tests::subprocess;

fn main() {
    let mut iter = std::env::args().skip(1);
    let exe = iter.next().unwrap();
    subprocess(exe, iter).unwrap();
}
