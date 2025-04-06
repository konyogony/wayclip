use daemonize::Daemonize;

fn main() {
    let daemonize = Daemonize::new()
        .pid_file("/tmp/wayclip-daemon.pid")
        .chown_pid_file(true)
        .working_directory("/")
        .umask(0o027)
        .stdout(std::fs::File::create("/tmp/wayclip-daemon.out").unwrap())
        .stderr(std::fs::File::create("/tmp/wayclip-daemon.err").unwrap());

    match daemonize.start() {
        Ok(_) => println!("Daemonized"),
        Err(e) => eprintln!("Error: {}", e),
    }

    loop {
        // ur service logic
    }
}
