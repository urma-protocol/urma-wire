use crate::logging;
use std::path::Path;
use std::sync::Arc;
use tauri::{Manager, State};
use wire_app_core::contract::{Request, Response};
use wire_app_core::errors::{Fault, FaultKind};
use wire_app_engine::config::Settings;
use wire_app_engine::errors::NativeError;
use wire_app_engine::native::media::fetch;
use wire_app_engine::native::service::Service;

struct Engine(Arc<Service>);

#[tauri::command]
async fn engine(request: Request, state: State<'_, Engine>) -> Result<Response, Fault> {
    let service = state.0.clone();
    match tauri::async_runtime::spawn_blocking(move || serve(&service, request)).await {
        Ok(answer) => answer,
        Err(error) => {
            tracing::error!(error = %error, "engine task could not complete");
            Err(Fault::new(
                FaultKind::Transport,
                "Wire engine task could not complete",
            ))
        }
    }
}

fn serve(service: &Service, request: Request) -> Result<Response, Fault> {
    let op = request.name();
    match answer(service, request) {
        Ok(response) => {
            if network_bound(op) {
                tracing::info!(op, "request served");
            }
            Ok(response)
        }
        Err(fault) => {
            tracing::warn!(op, kind = ?fault.kind, message = %fault.message, "request failed");
            Err(fault)
        }
    }
}

fn network_bound(op: &str) -> bool {
    matches!(
        op,
        "history sync start"
            | "wallet"
            | "connection health"
            | "chain source change"
            | "publication quote"
            | "publication advance"
    )
}

fn answer(service: &Service, request: Request) -> Result<Response, Fault> {
    match request {
        Request::FetchImage { url } => Ok(Response::Image(fetch(&url).map_err(NativeError::from)?)),
        engine => Ok(service.handle(engine)?),
    }
}

#[cfg(mobile)]
fn settings(data_dir: &Path) -> Result<Settings, NativeError> {
    Ok(Settings::mobile(data_dir))
}

#[cfg(not(mobile))]
fn settings(data_dir: &Path) -> Result<Settings, NativeError> {
    Settings::load(data_dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launched = tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            match logging::install(&data_dir) {
                Ok(path) => tracing::info!(log = %path.display(), "file logging installed"),
                Err(error) => tracing::warn!(error = %error, "file logging unavailable"),
            }
            tracing::info!(
                version = %app.package_info().version,
                data_dir = %data_dir.display(),
                "URMA Wire starting"
            );
            let service = match Service::open(&data_dir, settings(&data_dir)?) {
                Ok(service) => {
                    tracing::info!("engine service opened");
                    service
                }
                Err(error) => {
                    tracing::error!(error = %error, "engine service failed to open");
                    return Err(error.into());
                }
            };
            app.manage(Engine(Arc::new(service)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![engine])
        .run(tauri::generate_context!());
    match launched {
        Ok(()) => (),
        Err(error) => panic!("error while running URMA Wire application: {error}"),
    }
}
