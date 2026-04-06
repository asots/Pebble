use serde::{Deserialize, Serialize};

/// A pair of OAuth tokens (access + optional refresh).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds) when the access token expires.
    pub expires_at: Option<i64>,
    pub scopes: Vec<String>,
}

impl TokenPair {
    /// Returns `true` if the token expires within the next 5 minutes (or is
    /// already expired).  Tokens without an expiry are considered valid.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                exp - now < 300 // 5 minutes buffer
            }
            None => false,
        }
    }

    /// Returns `true` when the token is expired **and** a refresh token is
    /// available.
    pub fn needs_refresh(&self) -> bool {
        self.is_expired() && self.refresh_token.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now_secs() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn make_token(expires_at: Option<i64>, refresh: Option<&str>) -> TokenPair {
        TokenPair {
            access_token: "access".into(),
            refresh_token: refresh.map(String::from),
            expires_at,
            scopes: vec!["mail".into()],
        }
    }

    #[test]
    fn not_expired_when_no_expiry() {
        let t = make_token(None, None);
        assert!(!t.is_expired());
    }

    #[test]
    fn not_expired_when_far_future() {
        let t = make_token(Some(now_secs() + 3600), None);
        assert!(!t.is_expired());
    }

    #[test]
    fn expired_when_in_past() {
        let t = make_token(Some(now_secs() - 60), None);
        assert!(t.is_expired());
    }

    #[test]
    fn expired_within_5min_buffer() {
        // Expires in 2 minutes — within the 5-minute buffer
        let t = make_token(Some(now_secs() + 120), None);
        assert!(t.is_expired());
    }

    #[test]
    fn needs_refresh_when_expired_with_refresh_token() {
        let t = make_token(Some(now_secs() - 60), Some("refresh"));
        assert!(t.needs_refresh());
    }

    #[test]
    fn no_refresh_needed_when_not_expired() {
        let t = make_token(Some(now_secs() + 3600), Some("refresh"));
        assert!(!t.needs_refresh());
    }

    #[test]
    fn no_refresh_needed_when_no_refresh_token() {
        let t = make_token(Some(now_secs() - 60), None);
        assert!(!t.needs_refresh());
    }
}
