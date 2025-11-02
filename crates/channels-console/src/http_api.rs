use crate::get_serializable_stats;
use tiny_http::{Response, Server};

pub(crate) fn start_metrics_server(addr: &str) {
    let server = match Server::http(addr) {
        Ok(s) => s,
        Err(e) => {
            panic!("Failed to bind metrics server to {}: {}. Customize the port using the CHANNELS_CONSOLE_METRICS_PORT environment variable.", addr, e);
        }
    };

    println!("Channel metrics server listening on http://{}", addr);

    for request in server.incoming_requests() {
        if request.url() == "/metrics" {
            let stats = get_serializable_stats();
            match serde_json::to_string(&stats) {
                Ok(json) => {
                    let response = Response::from_string(json).with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .unwrap(),
                    );
                    let _ = request.respond(response);
                }
                Err(e) => {
                    eprintln!("Failed to serialize metrics: {}", e);
                    let response = Response::from_string(format!("Internal server error: {}", e))
                        .with_status_code(500);
                    let _ = request.respond(response);
                }
            }
        } else {
            let response = Response::from_string("Not found").with_status_code(404);
            let _ = request.respond(response);
        }
    }
}
