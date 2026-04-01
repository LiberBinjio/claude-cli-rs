//! System keychain integration for secure credential storage.

use anyhow::{Context, Result};
use crate::oauth::OAuthTokens;

const SERVICE_NAME: &str = "claude-code";
const API_KEY_USER: &str = "api-key";
const OAUTH_USER: &str = "oauth-tokens";

/// Store an API key in the system keychain.
pub fn store_api_key(key: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, API_KEY_USER)
        .context("Failed to create keychain entry")?;
    entry
        .set_password(key)
        .context("Failed to store API key in keychain")?;
    Ok(())
}

/// Load an API key from the system keychain. Returns `None` if not found.
pub fn load_api_key() -> Result<Option<String>> {
    let entry = keyring::Entry::new(SERVICE_NAME, API_KEY_USER)
        .context("Failed to create keychain entry")?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Keychain error: {e}")),
    }
}

/// Delete the API key from the system keychain.
pub fn delete_api_key() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, API_KEY_USER)
        .context("Failed to create keychain entry")?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("Failed to delete API key from keychain: {e}")),
    }
}

/// Store OAuth tokens (serialized as JSON) in the system keychain.
pub fn store_oauth_tokens(tokens: &OAuthTokens) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, OAUTH_USER)
        .context("Failed to create keychain entry")?;
    let json = serde_json::to_string(tokens)?;
    entry
        .set_password(&json)
        .context("Failed to store OAuth tokens in keychain")?;
    Ok(())
}

/// Load OAuth tokens from the system keychain. Returns `None` if not found.
pub fn load_oauth_tokens() -> Result<Option<OAuthTokens>> {
    let entry = keyring::Entry::new(SERVICE_NAME, OAUTH_USER)
        .context("Failed to create keychain entry")?;
    match entry.get_password() {
        Ok(json) => {
            let tokens = serde_json::from_str(&json)?;
            Ok(Some(tokens))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Keychain error: {e}")),
    }
}

/// Delete OAuth tokens from the system keychain.
pub fn delete_oauth_tokens() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, OAUTH_USER)
        .context("Failed to create keychain entry")?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("Failed to delete OAuth tokens: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires real system keychain
    fn test_api_key_roundtrip() {
        store_api_key("sk-test-keychain").unwrap();
        let loaded = load_api_key().unwrap();
        assert_eq!(loaded, Some("sk-test-keychain".into()));
        delete_api_key().unwrap();
        let after = load_api_key().unwrap();
        assert_eq!(after, None);
    }

    #[test]
    #[ignore] // Requires real system keychain
    fn test_oauth_tokens_roundtrip() {
        let tokens = OAuthTokens {
            access_token: "access_test".into(),
            refresh_token: "refresh_test".into(),
            expires_at: 99999,
        };
        store_oauth_tokens(&tokens).unwrap();
        let loaded = load_oauth_tokens().unwrap().unwrap();
        assert_eq!(loaded.access_token, "access_test");
        delete_oauth_tokens().unwrap();
    }
}
