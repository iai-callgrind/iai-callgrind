use iai_callgrind::library_benchmark;

#[library_benchmark]
#[bench::id1(42)]
#[bench::id2(0)]
#[bench::id3(255)]
fn bench1(my: u8) -> u8 {
    my
}

#[library_benchmark]
fn bench2() {}

#[library_benchmark]
#[bench::id1()]
#[bench::id2()]
fn bench3() {}

#[library_benchmark]
#[bench::id1(55, 200)]
pub fn bench4(my: u8, other: u8) -> u8 {
    my + other
}

fn some_setup(my: u8) -> u64 {
    my as u64 + 1
}

// check that some_setup is accessible and executed
#[library_benchmark]
#[bench::id1(some_setup(8))]
pub fn bench5(my: u64) -> u64 {
    if my != 9 {
        panic!("Should pass");
    } else {
        my
    }
}

fn main() {
    // quickly check that everything's set up correctly
    assert_eq!(bench4::FUNCTIONS.len(), 1);

    let (id, args, func) = bench4::FUNCTIONS[0];
    assert_eq!(id, &"id1");
    assert_eq!(args, &"55, 200");
    assert_eq!(func(), ());
    assert_eq!(bench4::id1(), ());

    assert_eq!(bench5::FUNCTIONS[0].2(), ());
    assert_eq!(bench5::id1(), ());
}
