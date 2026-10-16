use crate::config::autoload_images_when_unset;
use crate::config::replies_shown_when_unset;
use crate::config::show_mempool_when_unset;
use crate::data::storage::{read, write, Held, Recalled, Slot};
use tracing::warn;
use wire_app_core::errors::Fault;

pub fn stored_autoload() -> Recalled<bool> {
    match read(Slot::Images) {
        Held::Value(disabled) => Recalled::kept(disabled != "true"),
        Held::Unset => Recalled::kept(autoload_images_when_unset()),
        Held::Unreadable(reason) => Recalled::doubted(autoload_images_when_unset(), reason),
    }
}

pub fn remember_autoload(enabled: bool) -> Result<(), Fault> {
    write(Slot::Images, &(!enabled).to_string())
}

pub fn stored_replies_shown() -> Recalled<usize> {
    match read(Slot::Replies) {
        Held::Value(count) => match count.trim().parse::<usize>() {
            Ok(count) => Recalled::kept(count),
            Err(error) => {
                warn!(error = %error, "saved reply preview size is not a count");
                Recalled::doubted(
                    replies_shown_when_unset(),
                    "The saved reply preview size was unreadable; the default applies".to_owned(),
                )
            }
        },
        Held::Unset => Recalled::kept(replies_shown_when_unset()),
        Held::Unreadable(reason) => Recalled::doubted(replies_shown_when_unset(), reason),
    }
}

pub fn remember_replies_shown(count: usize) -> Result<(), Fault> {
    write(Slot::Replies, &count.to_string())
}

pub fn stored_show_mempool() -> Recalled<bool> {
    match read(Slot::Mempool) {
        Held::Value(shown) => Recalled::kept(shown == "true"),
        Held::Unset => Recalled::kept(show_mempool_when_unset()),
        Held::Unreadable(reason) => Recalled::doubted(show_mempool_when_unset(), reason),
    }
}

pub fn remember_show_mempool(shown: bool) -> Result<(), Fault> {
    write(Slot::Mempool, &shown.to_string())
}
