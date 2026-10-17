use anyhow::{Context, Result};
use oo7::Keyring;
use std::collections::HashMap;
use tokio::sync::OnceCell;

pub const SERVICE_NAME: &str = "ver_remote_connection_manager";

static CACHED_KEYRING: OnceCell<Keyring> = OnceCell::const_new();

async fn get_cached_keyring() -> Result<&'static Keyring> {
    // Failed initialization is retried, allowing a keyring started later to recover.
    CACHED_KEYRING
        .get_or_try_init(|| async {
            Keyring::new()
                .await
                .context("Secret Service keyring unavailable")
        })
        .await
}

/// Retrieves password for connection ID from Secret Service (oo7 keyring).
async fn get_password_inner(id: &str) -> Result<Option<String>> {
    let keyring = get_cached_keyring().await?;

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
        .context("Failed to search Secret Service keyring")?;

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
async fn set_password_inner(id: &str, password: &str) -> Result<()> {
    let keyring = get_cached_keyring().await?;

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
async fn delete_password_inner(id: &str) -> Result<()> {
    let keyring = get_cached_keyring().await?;

    let items = keyring
        .search_items(&HashMap::from([
            ("service", SERVICE_NAME),
            ("connection_id", id),
        ]))
        .await
        .context("Failed to search Secret Service keyring")?;

    for item in items {
        item.delete()
            .await
            .context("Failed to delete password from keyring")?;
    }

    let legacy_items = keyring
        .search_items(&HashMap::from([
            ("service", SERVICE_NAME),
            ("username", id),
        ]))
        .await
        .context("Failed to search Secret Service keyring")?;

    for item in legacy_items {
        item.delete()
            .await
            .context("Failed to delete password from keyring")?;
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

/// Run keyring work on the persistent runtime so GUI callers never block its event loop.
async fn run_async<F, T>(future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    get_shared_runtime()
        .spawn(async move {
            tokio::time::timeout(std::time::Duration::from_secs(30), async move {
                static OPERATIONS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
                let _operation = OPERATIONS.lock().await;
                future.await
            })
            .await
            .context("Keyring request timed out")?
        })
        .await
        .context("Keyring worker failed")?
}

pub async fn get_password(id: &str) -> Result<Option<String>> {
    let id = id.to_string();
    run_async(async move { get_password_inner(&id).await }).await
}

pub async fn set_password(id: &str, password: &str) -> Result<()> {
    let id = id.to_string();
    let password = password.to_string();
    run_async(async move { set_password_inner(&id, &password).await }).await
}

pub async fn delete_password(id: &str) -> Result<()> {
    let id = id.to_string();
    run_async(async move { delete_password_inner(&id).await }).await
}

fn run_sync<F, T>(future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::spawn(move || get_shared_runtime().block_on(future))
            .join()
            .map_err(|_| anyhow::anyhow!("Keyring worker thread panicked"))?
    } else {
        get_shared_runtime().block_on(future)
    }
}

/// Synchronous wrapper around get_password for non-async contexts.
pub fn get_password_sync(id: &str) -> Result<Option<String>> {
    let id_owned = id.to_string();
    run_sync(async move { get_password(&id_owned).await })
}

/// Synchronous wrapper around set_password for non-async contexts.
pub fn set_password_sync(id: &str, password: &str) -> Result<()> {
    let id_owned = id.to_string();
    let pass_owned = password.to_string();
    run_sync(async move { set_password(&id_owned, &pass_owned).await })
}

/// Synchronous wrapper around delete_password for non-async contexts.
pub fn delete_password_sync(id: &str) -> Result<()> {
    let id_owned = id.to_string();
    run_sync(async move { delete_password(&id_owned).await })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn worker_propagates_keyring_errors_without_blocking_the_caller() {
        let error = run_async(async { Err::<(), _>(anyhow::anyhow!("keyring unavailable")) })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("keyring unavailable"));
        assert_eq!(run_async(async { Ok(42) }).await.unwrap(), 42);
    }

    #[test]
    fn unavailable_keyring_returns_errors_without_touching_user_secrets() {
        const CHILD: &str = "VER_KEYRING_FAILURE_TEST_CHILD";
        if std::env::var_os(CHILD).is_some() {
            assert!(get_password_sync("missing").is_err());
            assert!(set_password_sync("missing", "password").is_err());
            assert!(delete_password_sync("missing").is_err());
            // A failed initialization must remain an error on subsequent reads.
            assert!(get_password_sync("missing").is_err());
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "secrets::tests::unavailable_keyring_returns_errors_without_touching_user_secrets",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={}/no-bus", dir.path().display()),
            )
            .env("XDG_DATA_HOME", dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_service_name_constant() {
        assert_eq!(SERVICE_NAME, "ver_remote_connection_manager");
    }

    #[tokio::test]
    #[ignore = "Requires an unlocked Secret Service keyring"]
    async fn test_keyring_password_handling_graceful() {
        let test_id = uuid::Uuid::new_v4().to_string();
        let test_id = test_id.as_str();
        let test_pass = "super_secret_p@ssw0rd";

        let get_res = get_password(test_id).await;
        assert!(get_res.is_ok());

        let set_res = set_password(test_id, test_pass).await;
        assert!(set_res.is_ok());
        assert_eq!(
            get_password(test_id).await.unwrap().as_deref(),
            Some(test_pass)
        );

        let del_res = delete_password(test_id).await;
        assert!(del_res.is_ok());
        assert_eq!(get_password(test_id).await.unwrap(), None);
    }

    #[test]
    fn sync_bridge_propagates_results() {
        assert_eq!(run_sync(async { Ok(42) }).unwrap(), 42);
        assert!(run_sync(async { Err::<(), _>(anyhow::anyhow!("failed")) }).is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn sync_bridge_supports_a_current_thread_runtime() {
        assert_eq!(run_sync(async { Ok(42) }).unwrap(), 42);
    }
}
