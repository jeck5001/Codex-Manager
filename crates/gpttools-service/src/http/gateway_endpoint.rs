use tiny_http::Request;

pub fn handle_gateway(request: Request) {
    if let Err(err) = crate::gateway::handle_gateway_request(request) {
        log::error!("gateway request error: {err}");
    }
}
