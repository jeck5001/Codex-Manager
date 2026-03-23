fn main() {
    codexmanager_service::portable::bootstrap_current_process();
    codexmanager_service::initialize_process_logging();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match parse_transport(args.as_slice()) {
        Ok(Transport::Stdio) => codexmanager_service::mcp::stdio::run_stdio_server(),
        Ok(Transport::HttpSse) => codexmanager_service::mcp::http_sse::run_http_sse_server(),
        Err(err) => Err(err),
    };
    if let Err(err) = result {
        eprintln!("codexmanager-mcp stopped: {err}");
        std::process::exit(1);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Transport {
    Stdio,
    HttpSse,
}

fn parse_transport(args: &[String]) -> Result<Transport, String> {
    let index = 0_usize;
    while index < args.len() {
        match args[index].as_str() {
            "stdio" => return Ok(Transport::Stdio),
            "http-sse" | "--http-sse" => return Ok(Transport::HttpSse),
            "--transport" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "missing value after --transport; use stdio or http-sse".to_string()
                    );
                };
                return parse_transport(&[value.clone()]);
            }
            value if value.starts_with("--transport=") => {
                let Some((_, transport)) = value.split_once('=') else {
                    break;
                };
                return parse_transport(&[transport.to_string()]);
            }
            other => {
                return Err(format!(
                    "unsupported transport argument: {other}; use stdio or http-sse"
                ));
            }
        }
    }
    Ok(Transport::Stdio)
}

#[cfg(test)]
mod tests {
    use super::{parse_transport, Transport};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parse_transport_defaults_to_stdio() {
        assert_eq!(
            parse_transport(&[]).expect("default transport"),
            Transport::Stdio
        );
    }

    #[test]
    fn parse_transport_accepts_http_sse_aliases() {
        assert_eq!(
            parse_transport(args(&["http-sse"]).as_slice()).expect("http-sse alias"),
            Transport::HttpSse
        );
        assert_eq!(
            parse_transport(args(&["--http-sse"]).as_slice()).expect("--http-sse alias"),
            Transport::HttpSse
        );
        assert_eq!(
            parse_transport(args(&["--transport", "http-sse"]).as_slice())
                .expect("--transport http-sse"),
            Transport::HttpSse
        );
        assert_eq!(
            parse_transport(args(&["--transport=http-sse"]).as_slice())
                .expect("--transport=http-sse"),
            Transport::HttpSse
        );
    }

    #[test]
    fn parse_transport_rejects_unknown_value() {
        let err = parse_transport(args(&["websocket"]).as_slice()).expect_err("unknown transport");
        assert!(err.contains("unsupported transport argument"));
    }
}
