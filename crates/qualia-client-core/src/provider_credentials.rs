//! OS-keychain backed credentials for user-configured inference connections.
//!
//! Agent rosters deliberately store only a stable connection identifier.  The
//! bearer value is entered by the principal, written to the platform keychain,
//! and is never returned through an API, included in diagnostics, or persisted
//! in JSON alongside the endpoint.

use keyring::Entry;

const SERVICE: &str = "qualia_db_provider_credentials";

fn valid_connection_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn entry(id: &str) -> Result<Entry, String> {
    if !valid_connection_id(id) {
        return Err(
            "connection ID must use lowercase letters, digits, and hyphens (max 80)".into(),
        );
    }
    Entry::new(SERVICE, id).map_err(|error| format!("OS keychain unavailable: {error}"))
}

/// Store a user-supplied credential.  The value is deliberately not returned.
pub fn store_bearer_credential(id: &str, secret: &str) -> Result<(), String> {
    if secret.trim().is_empty() {
        return Err("credential cannot be empty".into());
    }
    entry(id)?
        .set_password(secret)
        .map_err(|error| format!("could not save credential in the OS keychain: {error}"))
}

/// Remove a connection credential from the operating-system keychain.
pub fn remove_bearer_credential(id: &str) -> Result<(), String> {
    match entry(id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("could not remove OS-keychain credential: {error}")),
    }
}

/// Retrieve a secret for the narrow duration of an authorized outbound call.
/// This function is crate-visible so no UI/API layer can accidentally return it.
pub(crate) fn bearer_credential(id: &str) -> Result<String, String> {
    entry(id)?.get_password().map_err(|error| match error {
        keyring::Error::NoEntry => "no credential is saved for this connection".to_string(),
        other => format!("could not read OS-keychain credential: {other}"),
    })
}

#[cfg(test)]
mod tests {
    use super::valid_connection_id;

    #[test]
    fn connection_ids_are_bounded_and_path_safe() {
        assert!(valid_connection_id("openai-research"));
        assert!(!valid_connection_id("OpenAI"));
        assert!(!valid_connection_id("../../escape"));
        assert!(!valid_connection_id(""));
    }
}
