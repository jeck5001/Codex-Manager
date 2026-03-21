fn main() {
    codexmanager_service::portable::bootstrap_current_process();
    let configured_addr = std::env::var("CODEXMANAGER_SERVICE_ADDR")
        .unwrap_or_else(|_| codexmanager_service::default_listener_bind_addr());
    let addr = codexmanager_service::listener_bind_addr(&configured_addr);
    println!("codexmanager-service listening on {addr}");
    if let Err(err) = codexmanager_service::start_server(&addr) {
        eprintln!("service stopped: {err}");
        std::process::exit(1);
    }
}
