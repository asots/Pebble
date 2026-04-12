use std::time::Duration;

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

const REDIRECT_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthRedirect {
    pub code: String,
    pub state: String,
}

/// A bound TCP listener ready to accept the OAuth redirect.
/// Created by [`bind_redirect_listener`], consumed by [`BoundRedirectListener::wait`].
pub struct BoundRedirectListener {
    listener: TcpListener,
    /// The actual port the listener is bound to.
    pub port: u16,
}

/// Bind a TCP listener on `127.0.0.1:{port}`.
/// If port is 0, the OS assigns an available port.
pub async fn bind_redirect_listener(port: u16) -> Result<BoundRedirectListener, OAuthError> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    let port = listener
        .local_addr()
        .map_err(|e| OAuthError::Redirect(format!("Failed to get local addr: {e}")))?
        .port();
    tracing::debug!("OAuth redirect listener bound on port {}", port);
    Ok(BoundRedirectListener { listener, port })
}

impl BoundRedirectListener {
    /// Wait for the OAuth redirect callback with a 5-minute timeout.
    pub async fn wait(self) -> Result<OAuthRedirect, OAuthError> {
        let (mut stream, _addr) = tokio::time::timeout(REDIRECT_TIMEOUT, self.listener.accept())
            .await
            .map_err(|_| {
                OAuthError::Redirect(
                    "OAuth redirect timed out after 5 minutes. Please try again.".into(),
                )
            })?
            .map_err(OAuthError::Io)?;

        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).await?;
        let request = String::from_utf8_lossy(&buf[..n]);

        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .ok_or_else(|| OAuthError::Redirect("Failed to parse HTTP request line".into()))?;

        let url = Url::parse(&format!("http://127.0.0.1:{}{}", self.port, path))
            .map_err(|e| OAuthError::Redirect(format!("Failed to parse redirect URL: {}", e)))?;

        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .ok_or_else(|| {
                let error = url
                    .query_pairs()
                    .find(|(k, _)| k == "error")
                    .map(|(_, v)| v.into_owned())
                    .unwrap_or_else(|| "unknown".into());
                OAuthError::Redirect(format!("Authorization denied or missing code: {}", error))
            })?;

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .ok_or_else(|| OAuthError::Redirect("Authorization callback missing state".into()))?;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            SUCCESS_HTML.len(),
            SUCCESS_HTML
        );
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;

        tracing::debug!("OAuth redirect received authorization code");
        Ok(OAuthRedirect { code, state })
    }
}

/// Convenience wrapper: bind + wait in one call (for backward compat).
pub async fn wait_for_redirect(port: u16) -> Result<OAuthRedirect, OAuthError> {
    let bound = bind_redirect_listener(port).await?;
    bound.wait().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn redirect_extracts_code_and_state() {
        let bound = bind_redirect_listener(0).await.unwrap();
        let port = bound.port;

        let handle = tokio::spawn(async move { bound.wait().await });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let request =
            "GET /callback?code=test_code_123&state=xyz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
            .await
            .unwrap();

        let redirect = handle.await.unwrap().unwrap();
        assert_eq!(redirect.code, "test_code_123");
        assert_eq!(redirect.state, "xyz");
    }

    #[tokio::test]
    async fn redirect_returns_error_on_missing_code() {
        let bound = bind_redirect_listener(0).await.unwrap();
        let port = bound.port;

        let handle = tokio::spawn(async move { bound.wait().await });

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

    #[tokio::test]
    async fn redirect_returns_error_on_missing_state() {
        let bound = bind_redirect_listener(0).await.unwrap();
        let port = bound.port;

        let handle = tokio::spawn(async move { bound.wait().await });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let request = "GET /callback?code=test_code_123 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
            .await
            .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_err());
    }
}
