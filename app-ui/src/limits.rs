use crate::data::topics::Remembered;
use wire_app_shared::identity::MAX_IDENTITY_VAULT_BYTES;

pub const MAX_VAULT_BYTES: usize = MAX_IDENTITY_VAULT_BYTES;

pub const VAULT_INPUT_ID: &str = "identity-vault-file";

pub const MAX_POST_BYTES: usize = 32_760;

pub const MAX_REPLY_BYTES: usize = 32_728;

pub const MAX_DISPLAY_BYTES: usize = 64 * 1024 * 1024;

pub const MAX_IMAGES_PER_POST: usize = 2;

pub const CONCURRENT_IMAGE_FETCHES: usize = 2;

pub const FEED_PAGE_SIZE: usize = 100;

pub const PUBLICATION_POLL_MS: u32 = 2000;

pub const AVATAR_PREVIEW_PIXELS: u32 = 128;

pub const PROFILE_AVATAR_PIXELS: u32 = 72;

pub const CARD_AVATAR_PIXELS: u32 = 40;

pub const IMAGE_PREFERENCE_KEY: &str = "wire-disable-external-images-v1";

pub const UNREADABLE_VAULT: &str = "Cannot read the selected vault";

pub const TOPICS_STORAGE_KEY: &str = "wire-topics-v1";

pub const MAX_REMEMBERED_LABELS: usize = 24;

pub const MAX_SUGGESTIONS: usize = 6;

pub fn topics_when_unset() -> Remembered {
    Remembered::default()
}

pub fn autoload_images_when_unset() -> bool {
    false
}

pub const NO_INDEXED_RANGE: &str = "None";

pub const NEVER_CHECKED: &str = "Never";

pub const UNKNOWN_HEIGHT: &str = "Unknown";
