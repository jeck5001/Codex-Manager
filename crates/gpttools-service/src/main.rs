#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    let addr = std::env::var("GPTTOOLS_SERVICE_ADDR")
        .unwrap_or_else(|_| gpttools_service::DEFAULT_ADDR.to_string());
    println!("gpttools-service listening on {addr}");
    if let Err(err) = gpttools_service::start_server(&addr) {
        eprintln!("service stopped: {err}");
        std::process::exit(1);
    }
}
