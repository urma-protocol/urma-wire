use crate::data::failure::Failure;
use crate::platform::tauri_call;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub async fn ask<T: DeserializeOwned>(command: &str, arguments: &Value) -> Result<T, Failure> {
    match tauri_call::<T>(command, arguments).await {
        Ok(answer) => Ok(answer),
        Err(message) => Err(Failure::Engine(message)),
    }
}
