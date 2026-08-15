use anyhow::{Context, Result};
use oo7::Keyring;
use std::collections::HashMap;

pub const SERVICE_NAME: &str = "ver_remote_connection_manager";

/// Retrieves password for connection ID from Secret Service (oo7 keyring).
pub async fn get_password(id: &str) -> Result<Option<String>> {
    let keyring = match Keyring::new().await {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Warning: Secret Service keyring unavailable: {}", e);
            return Ok(None);
        }
    };

    // Primary search using "service" and "connection_id"
    let items = keyring
        .search_items(&HashMap::from([
            ("service", SERVICE_NAME),
            ("connection_id", id),
        ]))
        .await
        .context("Failed to search secret keyring for connection password")?;

    if let Some(item) = items.first() {
        let secret_bytes = item
            .secret()
            .await
            .context("Failed to retrieve secret bytes")?;
        let password =
            String::from_utf8(secret_bytes.to_vec()).context("Secret is not valid UTF-8")?;
        return Ok(Some(password));
    }

    // Legacy fallback search matching Python keyring attributes ("username" = id)
    let legacy_items = keyring
        .search_items(&HashMap::from([
            ("service", SERVICE_NAME),
            ("username", id),
        ]))
        .await
        .unwrap_or_default();

    if let Some(item) = legacy_items.first() {
        let secret_bytes = item
            .secret()
            .await
            .context("Failed to retrieve secret bytes")?;
        let password =
            String::from_utf8(secret_bytes.to_vec()).context("Secret is not valid UTF-8")?;
        return Ok(Some(password));
    }

    Ok(None)
}

/// Stores password for connection ID in Secret Service (oo7 keyring).
pub async fn set_password(id: &str, password: &str) -> Result<()> {
    let keyring = match Keyring::new().await {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Warning: Secret Service keyring unavailable: {}", e);
            return Ok(());
        }
    };

    let label = format!("VER Connection Password ({})", id);

    keyring
        .create_item(
            &label,
            &HashMap::from([
                ("service", SERVICE_NAME),
                ("connection_id", id),
                ("username", id),
            ]),
            password.as_bytes(),
            true,
        )
        .await
        .context("Failed to store password in Secret Service keyring")?;

    Ok(())
}

/// Deletes stored password for connection ID from Secret Service.
pub async fn delete_password(id: &str) -> Result<()> {
    let keyring = match Keyring::new().await {
        Ok(k) => k,
        Err(_) => return Ok(()),
    };

    let items = keyring
        .search_items(&HashMap::from([
            ("service", SERVICE_NAME),
            ("connection_id", id),
        ]))
        .await
        .unwrap_or_default();

    for item in items {
        let _ = item.delete().await;
    }

    let legacy_items = keyring
        .search_items(&HashMap::from([
            ("service", SERVICE_NAME),
            ("username", id),
        ]))
        .await
        .unwrap_or_default();

    for item in legacy_items {
        let _ = item.delete().await;
    }

    Ok(())
}

use std::sync::OnceLock;

static SHARED_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn get_shared_runtime() -> &'static tokio::runtime::Runtime {
    SHARED_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .thread_name("ver-keyring-runtime")
            .build()
            .expect("Failed to initialize background Tokio runtime for secrets")
    })
}

/// Synchronous wrapper around get_password for non-async contexts.
pub fn get_password_sync(id: &str) -> Result<Option<String>> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
            let id = id.to_string();
            std::thread::spawn(move || handle.block_on(get_password(&id)))
                .join()
                .unwrap_or_else(|_| Ok(None))
        } else {
            tokio::task::block_in_place(|| handle.block_on(get_password(id)))
        }
    } else {
        let rt = get_shared_runtime();
        rt.block_on(get_password(id))
    }
}

/// Synchronous wrapper around set_password for non-async contexts.
pub fn set_password_sync(id: &str, password: &str) -> Result<()> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
            let id = id.to_string();
            let password = password.to_string();
            std::thread::spawn(move || handle.block_on(set_password(&id, &password)))
                .join()
                .unwrap_or_else(|_| Ok(()))
        } else {
            tokio::task::block_in_place(|| handle.block_on(set_password(id, password)))
        }
    } else {
        let rt = get_shared_runtime();
        rt.block_on(set_password(id, password))
    }
}

/// Synchronous wrapper around delete_password for non-async contexts.
pub fn delete_password_sync(id: &str) -> Result<()> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
            let id = id.to_string();
            std::thread::spawn(move || handle.block_on(delete_password(&id)))
                .join()
                .unwrap_or_else(|_| Ok(()))
        } else {
            tokio::task::block_in_place(|| handle.block_on(delete_password(id)))
        }
    } else {
        let rt = get_shared_runtime();
        rt.block_on(delete_password(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_name_constant() {
        assert_eq!(SERVICE_NAME, "ver_remote_connection_manager");
    }

    #[tokio::test]
    async fn test_keyring_password_handling_graceful() {
        let test_id = "test-uuid-unit-test-999";
        let test_pass = "super_secret_p@ssw0rd";

        let get_res = get_password(test_id).await;
        assert!(get_res.is_ok());

        let set_res = set_password(test_id, test_pass).await;
        assert!(set_res.is_ok());

        let del_res = delete_password(test_id).await;
        assert!(del_res.is_ok());
    }

    #[test]
    fn test_sync_wrappers_no_panic() {
        let test_id = "test-sync-uuid-123";
        let _ = get_password_sync(test_id);
        let _ = set_password_sync(test_id, "pass123");
        let _ = delete_password_sync(test_id);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_sync_wrappers_current_thread_runtime() {
        let test_id = "test-sync-current-thread-uuid-123";
        let _ = get_password_sync(test_id);
        let _ = set_password_sync(test_id, "pass123");
        let _ = delete_password_sync(test_id);
    }
}
