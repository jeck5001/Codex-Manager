fn main() {
    codexmanager_service::portable::bootstrap_current_process();
    codexmanager_service::initialize_process_logging();
    let addr = std::env::var("CODEXMANAGER_SERVICE_ADDR")
        .unwrap_or_else(|_| codexmanager_service::default_listener_bind_addr());
    println!("codexmanager-service listening on {addr}");
    if let Err(err) = codexmanager_service::start_server(&addr) {
        eprintln!("service stopped: {err}");
        std::process::exit(1);
    }
}
