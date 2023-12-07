// pub mod bindings {
//     extern "C" {
//         pub fn running_on_valgrind() -> cty::size_t;
//     }
// }

pub fn running_on_valgrind() -> usize {
    iai_callgrind::client_requests::valgrind::running_on_valgrind()
}
