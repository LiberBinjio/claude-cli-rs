//! Bridge JWT authentication and credential management.

use base64::Engine;
use serde::{Deserialize, Serialize};

/// Credentials for authenticating to the bridge WebSocket service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeCredentials {
    /// The JWT token.
    pub jwt: String,
    /// Remote environment id.
    pub environment_id: String,
    /// Unix timestamp (seconds) when this credential expires.
    pub expires_at: u64,
}

impl BridgeCredentials {
    /// Returns `true` when the current wall-clock time is past `expires_at`.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }
}

/// Decode the payload (claims) section of a JWT **without** verifying the
/// signature.  This is useful for extracting metadata such as `exp` or `sub`.
///
/// # Errors
///
/// Returns an error if the JWT does not have three dot-separated parts or if
/// the payload is not valid base64 / JSON.
pub fn decode_jwt_claims(jwt: &str) -> anyhow::Result<serde_json::Value> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        anyhow::bail!("invalid JWT format: expected 3 dot-separated parts");
    }

    // Try URL_SAFE_NO_PAD first, then URL_SAFE (with padding).
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(parts[1]))?;

    let claims: serde_json::Value = serde_json::from_slice(&payload)?;
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_already_expired() {
        let creds = BridgeCredentials {
            jwt: "tok".to_owned(),
            environment_id: "env1".to_owned(),
            expires_at: 0,
        };
        assert!(creds.is_expired());
    }

    #[test]
    fn credentials_far_future_not_expired() {
        let creds = BridgeCredentials {
            jwt: "tok".to_owned(),
            environment_id: "env1".to_owned(),
            expires_at: u64::MAX,
        };
        assert!(!creds.is_expired());
    }

    #[test]
    fn decode_jwt_claims_valid() {
        // header.payload.signature  — payload = {"sub":"123","exp":9999999999}
        let payload_json = r#"{"sub":"123","exp":9999999999}"#;
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(payload_json.as_bytes());
        let jwt = format!("eyJhbGciOiJIUzI1NiJ9.{encoded}.signature");
        let claims = decode_jwt_claims(&jwt).unwrap();
        assert_eq!(claims["sub"], "123");
        assert_eq!(claims["exp"], 9_999_999_999_u64);
    }

    #[test]
    fn decode_jwt_claims_invalid_format() {
        assert!(decode_jwt_claims("not-a-jwt").is_err());
        assert!(decode_jwt_claims("a.b").is_err());
    }

    #[test]
    fn credentials_serde_roundtrip() {
        let creds = BridgeCredentials {
            jwt: "abc.def.ghi".to_owned(),
            environment_id: "env-42".to_owned(),
            expires_at: 1_700_000_000,
        };
        let json = serde_json::to_string(&creds).unwrap();
        let parsed: BridgeCredentials = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.jwt, creds.jwt);
        assert_eq!(parsed.environment_id, creds.environment_id);
        assert_eq!(parsed.expires_at, creds.expires_at);
    }
}
