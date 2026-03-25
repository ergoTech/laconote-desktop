use std::io::{Read, Write};
use std::net::TcpListener;
use tracing::{info, warn};

const LISTEN_PORT: u16 = 19847;

/// Starts a one-shot HTTP server on localhost to capture the Google OAuth redirect.
/// Blocks until a request with `?code=...` arrives, then returns the code.
pub fn wait_for_oauth_code() -> Result<String, String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{LISTEN_PORT}"))
        .map_err(|e| format!("Failed to bind OAuth listener on port {LISTEN_PORT}: {e}"))?;

    // Set a timeout so we don't block forever
    listener
        .set_nonblocking(false)
        .map_err(|e| format!("Failed to set blocking mode: {e}"))?;

    info!("OAuth callback server listening on http://localhost:{LISTEN_PORT}");

    let (mut stream, _addr) = listener
        .accept()
        .map_err(|e| format!("Failed to accept OAuth callback connection: {e}"))?;

    let mut buf = [0u8; 4096];
    let n = stream
        .read(&mut buf)
        .map_err(|e| format!("Failed to read OAuth callback: {e}"))?;

    let request = String::from_utf8_lossy(&buf[..n]);

    // Extract the code from GET /calendar/callback?code=...&scope=...
    let code = extract_code_from_request(&request);

    // Send a nice HTML response
    let (status, body) = if code.is_some() {
        (
            "200 OK",
            "<html><body style='font-family:system-ui;text-align:center;padding:60px'>\
             <h2>Google Calendar connected!</h2>\
             <p>You can close this tab and return to Laconote.</p>\
             <script>setTimeout(()=>window.close(),2000)</script>\
             </body></html>",
        )
    } else {
        (
            "400 Bad Request",
            "<html><body style='font-family:system-ui;text-align:center;padding:60px'>\
             <h2>Something went wrong</h2>\
             <p>No authorization code received. Please try again.</p>\
             </body></html>",
        )
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    drop(stream);
    drop(listener);

    match code {
        Some(c) => {
            info!("OAuth code received from callback");
            Ok(c)
        }
        None => {
            warn!("OAuth callback received but no code found in request");
            Err("No authorization code in callback".into())
        }
    }
}

fn extract_code_from_request(request: &str) -> Option<String> {
    // Parse: GET /calendar/callback?code=XXXX&scope=... HTTP/1.1
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;

    for param in query.split('&') {
        let mut kv = param.splitn(2, '=');
        if kv.next()? == "code" {
            return kv.next().map(|v| urlencoding::decode(v).unwrap_or_default().into_owned());
        }
    }
    None
}
