use crate::data::engine::ask;
use wire_app_core::contract::{Request, Response};
use wire_app_core::errors::Fault;
use wire_app_core::markdown::FetchedImage;

pub async fn external_image(url: String) -> Result<FetchedImage, Fault> {
    match ask(Request::FetchImage { url }).await? {
        Response::Image(image) => Ok(image),
        other => Err(other.unexpected("external image")),
    }
}
