fn main() {
    iai_callgrind::client_requests::callgrind::start_instrumentation();
    println!("Hello World.");
    iai_callgrind::client_requests::callgrind::stop_instrumentation();
}
