use crate::OAuthError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html><head><title>Pebble</title></head>
<body style="font-family:sans-serif;text-align:center;padding:3rem">
<h2>Authentication successful</h2>
<p>You can close this tab and return to Pebble.</p>
</body></html>"#;

/// Listen on `127.0.0.1:{port}` for the OAuth redirect callback.
///
/// Accepts a single connection, parses the `?code=` query parameter from the
/// request, sends a friendly HTML response, and returns the authorization code.
pub async fn wait_for_redirect(port: u16) -> Result<String, OAuthError> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::debug!("OAuth redirect listener started on port {}", port);

    let (mut stream, _addr) = listener.accept().await?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Extract the request path from the first line, e.g. "GET /callback?code=abc HTTP/1.1"
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| OAuthError::Redirect("Failed to parse HTTP request line".into()))?;

    // Parse the full URL so we can extract query params
    let url = Url::parse(&format!("http://127.0.0.1:{}{}", port, path))
        .map_err(|e| OAuthError::Redirect(format!("Failed to parse redirect URL: {}", e)))?;

    let code = url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| {
            // Check for an error parameter
            let error = url
                .query_pairs()
                .find(|(k, _)| k == "error")
                .map(|(_, v)| v.into_owned())
                .unwrap_or_else(|| "unknown".into());
            OAuthError::Redirect(format!("Authorization denied or missing code: {}", error))
        })?;

    // Send HTTP response
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        SUCCESS_HTML.len(),
        SUCCESS_HTML
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    tracing::debug!("OAuth redirect received authorization code");
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn redirect_extracts_code() {
        // Bind to port 0 to get a random available port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener); // Release so wait_for_redirect can bind

        let handle = tokio::spawn(async move { wait_for_redirect(port).await });

        // Give the listener a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Simulate browser redirect
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let request = "GET /callback?code=test_code_123&state=xyz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
            .await
            .unwrap();

        let code = handle.await.unwrap().unwrap();
        assert_eq!(code, "test_code_123");
    }

    #[tokio::test]
    async fn redirect_returns_error_on_missing_code() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let handle = tokio::spawn(async move { wait_for_redirect(port).await });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let request = "GET /callback?error=access_denied HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
            .await
            .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_err());
    }
}
