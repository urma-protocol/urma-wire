use crate::data::engine::ask;
use crate::data::failure::Failure;
use serde_json::json;
use wire_app_shared::markdown::FetchedImage;

pub async fn external_image(url: String) -> Result<FetchedImage, Failure> {
    ask("fetch_external_image", &json!({ "url": url })).await
}
