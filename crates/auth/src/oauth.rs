//! OAuth 2.0 PKCE flow for Anthropic Console authentication.

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::debug;

const AUTHORIZE_URL: &str = "https://console.anthropic.com/oauth/authorize";
const TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
const CALLBACK_PORT: u16 = 19485;
const VERIFIER_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// OAuth tokens with expiry tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    /// Bearer access token.
    pub access_token: String,
    /// Refresh token for renewal.
    pub refresh_token: String,
    /// Unix timestamp (seconds) when the access token expires.
    pub expires_at: u64,
}

/// State for an in-progress OAuth PKCE flow.
pub struct OAuthFlow {
    client_id: String,
    redirect_uri: String,
    code_verifier: String,
}

impl OAuthFlow {
    /// Create a new OAuth flow with a random PKCE code verifier.
    #[must_use]
    pub fn new(client_id: impl Into<String>) -> Self {
        let redirect_uri = format!("http://127.0.0.1:{CALLBACK_PORT}/oauth/callback");
        let code_verifier = generate_code_verifier();
        Self {
            client_id: client_id.into(),
            redirect_uri,
            code_verifier,
        }
    }

    /// Build the authorization URL to open in a browser.
    #[must_use]
    pub fn authorization_url(&self, scope: &str) -> String {
        let challenge = generate_code_challenge(&self.code_verifier);
        format!(
            "{AUTHORIZE_URL}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&scope={}",
            urlencoding(&self.client_id),
            urlencoding(&self.redirect_uri),
            challenge,
            urlencoding(scope),
        )
    }

    /// Run the full OAuth PKCE flow: open browser, wait for callback, exchange code.
    pub async fn start_auth(&self, scope: &str) -> Result<OAuthTokens> {
        let auth_url = self.authorization_url(scope);

        // Bind local callback server on 127.0.0.1 only (security: no 0.0.0.0)
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{CALLBACK_PORT}"))
            .await
            .context("Failed to bind OAuth callback listener")?;

        debug!("OAuth callback listening on 127.0.0.1:{CALLBACK_PORT}");

        // Open browser
        if let Err(e) = open_browser(&auth_url) {
            tracing::warn!("Failed to open browser: {e}. Please open manually:\n{auth_url}");
        }

        // Wait for callback
        let (mut stream, _addr) = listener
            .accept()
            .await
            .context("Failed to accept OAuth callback connection")?;

        let mut buf = vec![0u8; 4096];
        let n = stream
            .read(&mut buf)
            .await
            .context("Failed to read callback request")?;
        let request = String::from_utf8_lossy(&buf[..n]);

        // Extract authorization code from query string
        let code = extract_code_from_request(&request)
            .context("No authorization code found in callback")?;

        // Send HTTP response to browser
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
            <html><body><h2>Authorization successful!</h2><p>You can close this tab.</p></body></html>";
        let _ = stream.write_all(response.as_bytes()).await;

        // Exchange code for tokens
        self.exchange_code(&code).await
    }

    /// Exchange an authorization code for tokens.
    async fn exchange_code(&self, code: &str) -> Result<OAuthTokens> {
        let client = reqwest::Client::new();
        let resp = client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", &self.client_id),
                ("code", code),
                ("redirect_uri", &self.redirect_uri),
                ("code_verifier", &self.code_verifier),
            ])
            .send()
            .await
            .context("Token exchange request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Token exchange failed (HTTP {status}): {body}");
        }

        parse_token_response(resp).await
    }

    /// Refresh an expired access token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens> {
        let client = reqwest::Client::new();
        let resp = client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", &self.client_id),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await
            .context("Token refresh request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Token refresh failed (HTTP {status}): {body}");
        }

        parse_token_response(resp).await
    }
}

/// Generate a random PKCE code verifier (43-128 chars from unreserved set).
fn generate_code_verifier() -> String {
    let mut rng = rand::rng();
    let len = rng.random_range(43..=128);
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..VERIFIER_CHARSET.len());
            VERIFIER_CHARSET[idx] as char
        })
        .collect()
}

/// Compute S256 code challenge: SHA-256 hash, base64url-encoded, no padding.
#[must_use]
pub fn generate_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

/// Save OAuth tokens to `~/.claude/oauth.json`.
pub fn save_tokens(tokens: &OAuthTokens) -> Result<()> {
    let path = tokens_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(tokens)?;
    std::fs::write(&path, json)?;
    // Restrict file permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    debug!("OAuth tokens saved to {}", path.display());
    Ok(())
}

/// Load OAuth tokens from `~/.claude/oauth.json`.
#[must_use]
pub fn load_tokens() -> Option<OAuthTokens> {
    let path = tokens_path().ok()?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Check whether tokens have expired (with 60s buffer).
#[inline]
#[must_use]
pub fn is_token_expired(tokens: &OAuthTokens) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now + 60 >= tokens.expires_at
}

// ---- helpers ----

fn tokens_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".claude").join("oauth.json"))
}

async fn parse_token_response(resp: reqwest::Response) -> Result<OAuthTokens> {
    #[derive(Deserialize)]
    struct TokenResp {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<u64>,
    }
    let body: TokenResp = resp.json().await.context("Failed to parse token response")?;
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        + body.expires_in.unwrap_or(3600);
    Ok(OAuthTokens {
        access_token: body.access_token,
        refresh_token: body.refresh_token.unwrap_or_default(),
        expires_at,
    })
}

fn extract_code_from_request(request: &str) -> Option<String> {
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next() == Some("code") {
            return kv.next().map(String::from);
        }
    }
    None
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .context("Failed to open browser")?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context("Failed to open browser")?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("Failed to open browser")?;
    }
    Ok(())
}

fn urlencoding(s: &str) -> String {
    // Minimal percent-encoding for URL components
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(char::from(b"0123456789ABCDEF"[(b >> 4) as usize]));
                out.push(char::from(b"0123456789ABCDEF"[(b & 0x0f) as usize]));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_challenge_known_vector() {
        // RFC 7636 Appendix B test vector
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = generate_code_challenge(verifier);
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn test_code_verifier_length() {
        let v = generate_code_verifier();
        assert!(v.len() >= 43 && v.len() <= 128, "verifier len: {}", v.len());
    }

    #[test]
    fn test_code_verifier_charset() {
        let v = generate_code_verifier();
        for c in v.chars() {
            assert!(
                c.is_ascii_alphanumeric() || "-._~".contains(c),
                "invalid char: {c}"
            );
        }
    }

    #[test]
    fn test_extract_code_from_request() {
        let req = "GET /oauth/callback?code=abc123&state=xyz HTTP/1.1\r\nHost: localhost\r\n";
        assert_eq!(extract_code_from_request(req), Some("abc123".into()));
    }

    #[test]
    fn test_extract_code_missing() {
        let req = "GET /oauth/callback?error=denied HTTP/1.1\r\n";
        assert_eq!(extract_code_from_request(req), None);
    }

    #[test]
    fn test_is_token_expired() {
        let expired = OAuthTokens {
            access_token: "x".into(),
            refresh_token: "y".into(),
            expires_at: 0,
        };
        assert!(is_token_expired(&expired));

        let future = OAuthTokens {
            access_token: "x".into(),
            refresh_token: "y".into(),
            expires_at: u64::MAX,
        };
        assert!(!is_token_expired(&future));
    }

    #[test]
    fn test_save_load_roundtrip() {
        // Only test in environments where ~/.claude/ is writable
        let tokens = OAuthTokens {
            access_token: "test_access".into(),
            refresh_token: "test_refresh".into(),
            expires_at: 9999999999,
        };
        if save_tokens(&tokens).is_ok() {
            let loaded = load_tokens().expect("should load saved tokens");
            assert_eq!(loaded.access_token, "test_access");
            assert_eq!(loaded.refresh_token, "test_refresh");
            // Cleanup
            if let Ok(p) = tokens_path() {
                let _ = std::fs::remove_file(p);
            }
        }
    }
}
