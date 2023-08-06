//! This is an example for setting up a benchmark with expensive setup costs and a usage example for
//! the alternative main! macro which allows passing arguments to callgrind.
//!
//! For a detailed explanation see the comments of `setup_array` and
//! `bench_bubble_with_expensive_setup`

use iai_callgrind::{black_box, main};

// Bubble sort
#[inline(never)]
fn bubble(mut array: Vec<i32>) -> Vec<i32> {
    for i in 0..array.len() {
        for j in 0..array.len() - i - 1 {
            if array[j + 1] < array[j] {
                array.swap(j, j + 1);
            }
        }
    }
    array
}

// Per default, the exported name by the compiler is not as expected
// `test_with_skip_setup::setup_array` but just `setup_array` and located in the top-level. Defining
// `toggle-collect=setup_array` would have worked but choosing an own name for this function is a
// safer bet because we need this exact name for the `toggle-collect` argument.
#[export_name = "some_special_id::setup_array"]
// To ensure that the toggle is effective we're telling the rust compiler to not inline this
// function.
#[inline(never)]
fn setup_array(start: i32) -> Vec<i32> {
    (0..start).rev().collect()
}

// This is a bad example for setting up a benchmark with iai-callgrind. The setup code is added to
// the event counts of the call to `bubble` and we have a messy result. See the benchmark below
// (bench_bubble_with_expensive_setup) for a better solution.
#[inline(never)]
fn bench_bubble_bad() -> Vec<i32> {
    let array = black_box(vec![6, 5, 4, 3, 2, 1]);
    bubble(array)
}

// Together with passing the `toggle-collect=some_special_id::setup_array` argument to callgrind
// in the main! macro, this is the best solution to eliminate setup code from the benchmark event
// counts.
//
// This is working because the default toggle is set to this benchmark function
// (*test_with_skip_setup::bench_bubble_with_expensive setup), so callgrind starts counting events
// when entering it and stops counting when leaving it. The additional toggle causes callgrind to
// stop counting when entering `some_special_id::setup_array` and start counting again when
// leaving it.
//
// Additionally, it's needed to tell the rust compiler to not inline this benchmark function or else
// the default toggle may not work as expected.
#[inline(never)]
fn bench_bubble_with_expensive_setup() -> Vec<i32> {
    bubble(black_box(setup_array(4000)))
}

main!(
    callgrind_args = "toggle-collect=some_special_id::setup_array";
    functions = bench_bubble_bad, bench_bubble_with_expensive_setup
);
