use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use urma_identity::vault::{EncryptedVault, UnlockCredential};
use wire_app_shared::{
    Chain, LedgerClient, PersistentWireStore, Post, PublicationQuote, Txid, WalletSummary,
    WireStore, WireWallet,
};
use zeroize::Zeroizing;

struct AppData {
    store: Arc<Mutex<PersistentWireStore>>,
    wallet: Arc<Mutex<Option<WireWallet>>>,
}

#[tauri::command]
fn get_feed(chain: String, limit: usize, state: State<'_, AppData>) -> Result<Vec<Post>, String> {
    let chain_enum = match chain.to_lowercase().as_str() {
        "litecoin" | "ltc" => Chain::Litecoin,
        _ => Chain::Bitcoin,
    };
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store.get_feed(chain_enum, limit).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_post(
    chain: String,
    txid: String,
    state: State<'_, AppData>,
) -> Result<Option<Post>, String> {
    let chain_enum = match chain.to_lowercase().as_str() {
        "litecoin" | "ltc" => Chain::Litecoin,
        _ => Chain::Bitcoin,
    };
    let parsed_txid = Txid::from_display_hex(&txid).map_err(|e| e.to_string())?;
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store
        .get_post(chain_enum, &parsed_txid)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_wallet_summary(chain: String, state: State<'_, AppData>) -> Result<WalletSummary, String> {
    let chain_enum = match chain.to_lowercase().as_str() {
        "litecoin" | "ltc" => Chain::Litecoin,
        _ => Chain::Bitcoin,
    };
    let wallet = state.wallet.lock().map_err(|e| e.to_string())?;
    let wallet = wallet.as_ref().ok_or("URMA identity vault is locked")?;
    let address = wallet
        .get_receive_address(chain_enum)
        .map_err(|e| e.to_string())?;

    let (confirmed, unconfirmed) =
        LedgerClient::get_balance(chain_enum, &address).unwrap_or((0, 0));
    let utxos = LedgerClient::get_utxos(chain_enum, &address).unwrap_or_default();
    let tip = LedgerClient::get_tip_height(chain_enum).unwrap_or(0);

    wallet
        .summary(chain_enum, confirmed, unconfirmed, utxos.len(), tip)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn unlock_wallet(
    vault_path: PathBuf,
    password: String,
    state: State<'_, AppData>,
) -> Result<(), String> {
    let password = Zeroizing::new(password);
    let bytes =
        urma_io::read_bounded(&vault_path, EncryptedVault::MAX_BYTES).map_err(|e| e.to_string())?;
    let opened = WireWallet::unlock(&bytes, UnlockCredential::Password(&password))
        .map_err(|e| e.to_string())?;
    let mut wallet = state.wallet.lock().map_err(|e| e.to_string())?;
    *wallet = Some(opened);
    Ok(())
}

#[tauri::command]
fn lock_wallet(state: State<'_, AppData>) -> Result<(), String> {
    let mut wallet = state.wallet.lock().map_err(|e| e.to_string())?;
    *wallet = None;
    Ok(())
}

#[tauri::command]
fn quote_publication(
    chain: String,
    text: String,
    fee_rate: Option<u64>,
    state: State<'_, AppData>,
) -> Result<PublicationQuote, String> {
    let chain_enum = match chain.to_lowercase().as_str() {
        "litecoin" | "ltc" => Chain::Litecoin,
        _ => Chain::Bitcoin,
    };
    let wallet = state.wallet.lock().map_err(|e| e.to_string())?;
    let wallet = wallet.as_ref().ok_or("URMA identity vault is locked")?;
    let address = wallet
        .get_receive_address(chain_enum)
        .map_err(|e| e.to_string())?;
    let (confirmed, _) = LedgerClient::get_balance(chain_enum, &address).unwrap_or((0, 0));
    let rate = fee_rate.unwrap_or(1);
    let signer = wallet.signer().map_err(|e| e.to_string())?;
    LedgerClient::quote_publication(chain_enum, &text, rate, confirmed, &signer)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_runtime_info() -> serde_json::Value {
    serde_json::json!({
        "platform": "Tauri 2 Native",
        "storage": "Persistent Disk Index (>= 1 Confirmed Only)",
        "protocol": "URMA V0 libraries",
        "chain_context": "Litecoin Testnet & Bitcoin Testnet4 (Strictly Isolated)",
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let store_path = app_dir.join("wire_store.json");
            let store = PersistentWireStore::open_or_create(&store_path)?;

            app.manage(AppData {
                store: Arc::new(Mutex::new(store)),
                wallet: Arc::new(Mutex::new(None)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_feed,
            get_post,
            get_wallet_summary,
            unlock_wallet,
            lock_wallet,
            quote_publication,
            get_runtime_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running URMA Wire application");
}
