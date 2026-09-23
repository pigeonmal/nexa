use std::io::{self, BufRead, Read, Write};

use nexa_lsp::protocol::JsonRpcRequest;
use nexa_lsp::server::LspServer;
use serde_json::json;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    let mut server = LspServer::new();

    loop {
        let mut line = String::new();
        if stdin_lock.read_line(&mut line)? == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.starts_with("Content-Length:") {
            let len_str = trimmed.trim_start_matches("Content-Length:").trim();
            let content_len: usize = match len_str.parse() {
                Ok(n) => n,
                Err(_) => continue,
            };

            // Read the empty separator line "\r\n"
            let mut separator = String::new();
            stdin_lock.read_line(&mut separator)?;

            // Read exact payload bytes
            let mut buffer = vec![0u8; content_len];
            stdin_lock.read_exact(&mut buffer)?;

            let payload_str = match String::from_utf8(buffer) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let request: JsonRpcRequest = match serde_json::from_str(&payload_str) {
                Ok(req) => req,
                Err(_) => continue,
            };

            let (response, notifications) = server.handle_request(request);

            // Send any diagnostics notifications first
            for notification in notifications {
                let notif_json = json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/publishDiagnostics",
                    "params": notification
                });
                send_json_rpc(&mut stdout_lock, &notif_json)?;
            }

            // Send response if applicable
            if let Some(resp) = response {
                let resp_json = serde_json::to_value(&resp).unwrap_or(serde_json::Value::Null);
                send_json_rpc(&mut stdout_lock, &resp_json)?;
            }
        }
    }

    Ok(())
}

fn send_json_rpc(writer: &mut impl Write, value: &serde_json::Value) -> io::Result<()> {
    let encoded = serde_json::to_string(value).unwrap_or_default();
    let header = format!("Content-Length: {}\r\n\r\n", encoded.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(encoded.as_bytes())?;
    writer.flush()?;
    Ok(())
}
