use wire_app_core::contract::{Request, Response};
use wire_app_core::errors::Fault;

#[cfg(target_arch = "wasm32")]
pub async fn ask(request: Request) -> Result<Response, Fault> {
    wire_app_engine::client::ask(request).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn ask(request: Request) -> Result<Response, Fault> {
    Err(Fault::new(
        wire_app_core::errors::FaultKind::Unsupported,
        format!("The Wire engine serves {} only inside the WASM frontend", request.name()),
    ))
}
