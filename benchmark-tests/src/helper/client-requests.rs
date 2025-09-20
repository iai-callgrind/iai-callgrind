fn main() {
    gungraun::client_requests::callgrind::start_instrumentation();
    println!("Hello World.");
    gungraun::client_requests::callgrind::stop_instrumentation();
}
