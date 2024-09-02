use std::fs::{remove_file, File};
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::path::Path;
use std::time::Duration;
use std::{env, thread};

use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Command, Delay,
    DelayKind,
};

#[binary_benchmark]
#[bench::delay()]
fn delay_duration() -> Command {
    let path = env!("CARGO_BIN_EXE_echo");
    Command::new(path)
        .setup_parallel(true)
        .delay(Duration::from_secs(2))
        .build()
}

fn setup_path() {
    let base_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    assert!(base_dir.ends_with("iai-callgrind/benchmark-tests"));
    let path = format!("{base_dir}/../target/tests/test_bin_bench_command_delay.pid");
    let file_path = Path::new(&path);
    if file_path.exists() {
        remove_file(file_path).unwrap();
    }

    println!("Waiting to create file...");
    thread::sleep(Duration::from_millis(300));
    println!("Creating file...");
    File::create(file_path).unwrap();
    println!("File created...");
}

#[binary_benchmark]
#[bench::delay(setup = setup_path())]
fn delay_path() -> Command {
    let cmd_path = env!("CARGO_BIN_EXE_echo");

    let base_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    assert!(base_dir.ends_with("iai-callgrind/benchmark-tests"));
    let path = format!("{base_dir}/../target/tests/test_bin_bench_command_delay.pid");
    let file_path = Path::new(&path);
    if file_path.exists() {
        remove_file(file_path).unwrap();
    }

    Command::new(cmd_path)
        .setup_parallel(true)
        .delay(
            Delay::new(DelayKind::PathExists(file_path.into())).timeout(Duration::from_millis(500)),
        )
        .build()
}

fn setup_tcp_server() {
    println!("Waiting to start server...");
    thread::sleep(Duration::from_millis(300));
    println!("Starting server...");
    let _listener = TcpListener::bind("127.0.0.1:31000".parse::<SocketAddr>().unwrap()).unwrap();
    thread::sleep(Duration::from_secs(1));
    println!("Stopped server...");
}

#[binary_benchmark]
#[bench::delay(setup = setup_tcp_server())]
fn delay_tcp() -> Command {
    let path = env!("CARGO_BIN_EXE_echo");

    Command::new(path)
        .setup_parallel(true)
        .delay(
            Delay::new(DelayKind::TcpConnect(
                "127.0.0.1:31000".parse::<SocketAddr>().unwrap(),
            ))
            .timeout(Duration::from_millis(500)),
        )
        .build()
}

fn setup_udp_server() {
    println!("Waiting to start server...");
    thread::sleep(Duration::from_millis(300));
    println!("Starting server...");
    let remote_addr = "127.0.0.1:34000".parse::<SocketAddr>().unwrap();
    let server = UdpSocket::bind(remote_addr).unwrap();
    server
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    server
        .set_write_timeout(Some(Duration::from_millis(100)))
        .unwrap();

    loop {
        let mut buf = [0; 1];

        match server.recv_from(&mut buf) {
            Ok((_size, from)) => {
                server.send_to(&[2], from).unwrap();
                break;
            }
            Err(_e) => {}
        }
    }
    println!("Stopped server...");
}

#[binary_benchmark]
#[bench::delay(setup = setup_udp_server())]
fn delay_udp() -> Command {
    let path = env!("CARGO_BIN_EXE_echo");

    Command::new(path)
        .setup_parallel(true)
        .delay(
            Delay::new(DelayKind::UdpResponse(
                "127.0.0.1:34000".parse::<SocketAddr>().unwrap(),
                vec![1],
            ))
            .timeout(Duration::from_millis(500)),
        )
        .build()
}

binary_benchmark_group!(
    name = delay;
    config = BinaryBenchmarkConfig::default();
    benchmarks = delay_duration, delay_path, delay_tcp, delay_udp
);

main!(binary_benchmark_groups = delay);
