use tiny_http::Request;

use crate::http::route_dispatch::{resolve_backend_route, BackendRoute};

pub(crate) fn dispatch_backend_request(request: Request) {
    let route = resolve_backend_route(request.method().as_str(), request.url());

    match route {
        BackendRoute::Rpc => crate::http::rpc_endpoint::handle_rpc(request),
        BackendRoute::AuthCallback => crate::http::callback_endpoint::handle_callback(request),
        BackendRoute::Metrics => crate::http::gateway_endpoint::handle_metrics(request),
        BackendRoute::Gateway => crate::http::gateway_endpoint::handle_gateway(request),
    }
}
