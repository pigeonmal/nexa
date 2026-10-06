use std::io::{self, BufRead, Read, Write};

use nexa_lsp::protocol::JsonRpcRequest;
use nexa_lsp::server::LspServer;
use serde_json::json;

const MAX_MESSAGE_BYTES: usize = 64 * 1024 * 1024;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut server = LspServer::new();

    while let Some(length) = read_content_length(&mut input)? {
        if length > MAX_MESSAGE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "LSP message exceeds 64 MiB",
            ));
        }
        let mut payload = vec![0; length];
        input.read_exact(&mut payload)?;
        let request: JsonRpcRequest = match serde_json::from_slice(&payload) {
            Ok(request) => request,
            Err(_) => continue,
        };
        if request.method == "exit" {
            break;
        }

        let (response, notifications) = server.handle_request(request);
        for notification in notifications {
            send_json_rpc(
                &mut output,
                &json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/publishDiagnostics",
                    "params": notification
                }),
            )?;
        }
        if let Some(response) = response {
            let value = serde_json::to_value(response)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            send_json_rpc(&mut output, &value)?;
        }
    }
    Ok(())
}

fn read_content_length(reader: &mut impl BufRead) -> io::Result<Option<usize>> {
    let mut content_length = None;
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            return Ok(None);
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
        let header = String::from_utf8_lossy(&line);
        if let Some((name, value)) = header.trim().split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    content_length.map(Some).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "missing or invalid Content-Length header",
        )
    })
}

fn send_json_rpc(writer: &mut impl Write, value: &serde_json::Value) -> io::Result<()> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write!(writer, "Content-Length: {}\r\n\r\n", encoded.len())?;
    writer.write_all(&encoded)?;
    writer.flush()
}
