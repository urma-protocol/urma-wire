use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use wire_app_shared::{
    connection::ConnectionHealth,
    live::Service,
    live_model::{Feed, Publication},
    IdentityStatus, Post, WalletSummary,
};

struct AppData(Arc<Mutex<Service>>);

async fn work<T: Send + 'static>(
    state: &AppData,
    task: impl FnOnce(&mut Service) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut service = service
            .lock()
            .map_err(|_| "Wire state unavailable".to_string())?;
        task(&mut service)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn testnet(chain: &str) -> Result<(), String> {
    if chain == "ltc" {
        Ok(())
    } else {
        Err("This Wire V0 flow supports Litecoin Testnet only".into())
    }
}

#[tauri::command]
async fn get_feed(chain: String, state: State<'_, AppData>) -> Result<Feed, String> {
    testnet(&chain)?;
    work(&state, |service| service.sync()).await
}

#[tauri::command]
async fn get_connection_health(
    chain: String,
    state: State<'_, AppData>,
) -> Result<ConnectionHealth, String> {
    testnet(&chain)?;
    work(&state, |service| Ok(service.connection_health())).await
}

#[tauri::command]
async fn get_post(
    chain: String,
    txid: String,
    state: State<'_, AppData>,
) -> Result<Option<Post>, String> {
    testnet(&chain)?;
    work(&state, move |service| service.post(&txid)).await
}

#[tauri::command]
async fn get_replies(
    chain: String,
    txid: String,
    state: State<'_, AppData>,
) -> Result<Vec<Post>, String> {
    testnet(&chain)?;
    work(&state, move |service| service.replies(&txid)).await
}

#[tauri::command]
async fn get_wallet_summary(
    chain: String,
    state: State<'_, AppData>,
) -> Result<WalletSummary, String> {
    testnet(&chain)?;
    work(&state, |service| service.wallet()).await
}

#[tauri::command]
async fn identity_status(state: State<'_, AppData>) -> Result<IdentityStatus, String> {
    work(&state, |service| service.identity_status()).await
}

#[tauri::command]
async fn import_identity(
    vault_bytes: Vec<u8>,
    password: String,
    state: State<'_, AppData>,
) -> Result<IdentityStatus, String> {
    work(&state, move |service| {
        service.import_identity(vault_bytes, password)
    })
    .await
}

#[tauri::command]
async fn unlock_wallet(
    password: String,
    use_unlock_file: bool,
    state: State<'_, AppData>,
) -> Result<(), String> {
    work(&state, move |service| {
        service.unlock(password, use_unlock_file)
    })
    .await
}

#[tauri::command]
async fn lock_wallet(state: State<'_, AppData>) -> Result<(), String> {
    work(&state, |service| {
        service.lock();
        Ok(())
    })
    .await
}

#[tauri::command]
async fn quote_publication(
    chain: String,
    text: String,
    target: Option<String>,
    wire: Option<String>,
    hashtags: Option<Vec<String>>,
    state: State<'_, AppData>,
) -> Result<Publication, String> {
    testnet(&chain)?;
    work(&state, move |service| {
        service.prepare_topics(
            text,
            target,
            wire.unwrap_or_default(),
            hashtags.unwrap_or_default(),
        )
    })
    .await
}

#[tauri::command]
async fn publication_status(state: State<'_, AppData>) -> Result<Option<Publication>, String> {
    work(&state, |service| service.status()).await
}

#[tauri::command]
async fn advance_publication(
    id: String,
    approve: bool,
    state: State<'_, AppData>,
) -> Result<Publication, String> {
    work(&state, move |service| service.advance(&id, approve)).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            #[cfg(mobile)]
            let settings = wire_app_shared::config::Settings::mobile(&data_dir);
            #[cfg(not(mobile))]
            let settings = wire_app_shared::config::Settings::load()?;
            let service = Service::open(&data_dir, settings).map_err(std::io::Error::other)?;
            app.manage(AppData(Arc::new(Mutex::new(service))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_feed,
            get_connection_health,
            get_post,
            get_replies,
            get_wallet_summary,
            identity_status,
            import_identity,
            unlock_wallet,
            lock_wallet,
            quote_publication,
            publication_status,
            advance_publication
        ])
        .run(tauri::generate_context!())
        .expect("error while running URMA Wire application");
}
