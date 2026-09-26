use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{Manager, State};
use wire_app_core::contract::{Request, Response};
use wire_app_core::errors::{Fault, FaultKind};
use wire_app_engine::config::Settings;
use wire_app_engine::errors::NativeError;
use wire_app_engine::native::media::fetch;
use wire_app_engine::native::service::Service;

struct Engine(Arc<Mutex<Service>>);

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

fn serve(service: &Mutex<Service>, request: Request) -> Result<Response, Fault> {
    match request {
        Request::FetchImage { url } => Ok(Response::Image(
            fetch(&url).map_err(NativeError::from)?,
        )),
        locked => {
            let mut held: MutexGuard<Service> = match service.lock() {
                Ok(held) => held,
                Err(poisoned) => {
                    tracing::error!(error = %poisoned, "Wire state lock is poisoned");
                    return Err(Fault::new(FaultKind::Storage, "Wire state unavailable"));
                }
            };
            Ok(held.handle(locked)?)
        }
    }
}

#[cfg(mobile)]
fn settings(data_dir: &Path) -> Result<Settings, NativeError> {
    Ok(Settings::mobile(data_dir))
}

#[cfg(not(mobile))]
fn settings(_data_dir: &Path) -> Result<Settings, NativeError> {
    Settings::load()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launched = tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let service = Service::open(&data_dir, settings(&data_dir)?)?;
            app.manage(Engine(Arc::new(Mutex::new(service))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![engine])
        .run(tauri::generate_context!());
    match launched {
        Ok(()) => (),
        Err(error) => panic!("error while running URMA Wire application: {error}"),
    }
}
