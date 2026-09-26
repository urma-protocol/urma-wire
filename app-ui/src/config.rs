pub const VAULT_INPUT_ID: &str = "identity-vault-file";

pub const MAX_NICKNAME_BYTES: usize = 32_760;

pub const MAX_DISPLAY_BYTES: usize = 64 * 1024 * 1024;

pub const MAX_IMAGES_PER_POST: usize = 2;

pub const CONCURRENT_IMAGE_FETCHES: usize = 2;

pub const FEED_PAGE_SIZE: usize = 100;

pub const PUBLICATION_POLL_MS: u32 = 2000;

pub const AVATAR_PREVIEW_PIXELS: u32 = 128;

pub const PROFILE_AVATAR_PIXELS: u32 = 72;

pub const CARD_AVATAR_PIXELS: u32 = 40;

pub const REPLY_AVATAR_PIXELS: u32 = 32;

pub const AVATAR_BACKGROUND: u8 = 15;

pub fn avatar_when_unset() -> [u8; 1024] {
    [AVATAR_BACKGROUND; 1024]
}

pub fn nickname_when_unset() -> String {
    String::new()
}

pub fn autoload_images_when_unset() -> bool {
    false
}

pub fn reveal_margin_when_unset() -> f64 {
    26.0
}

pub fn replies_shown_when_unset() -> usize {
    5
}

pub const IMAGE_PREFERENCE_KEY: &str = "wire-disable-external-images-v1";

pub const REPLIES_SHOWN_KEY: &str = "wire-replies-shown-v1";

pub const UNREADABLE_VAULT: &str = "Cannot read the selected vault";

pub const TOPICS_STORAGE_KEY: &str = "wire-topics-v1";

pub const MAX_REMEMBERED_LABELS: usize = 24;

pub const MAX_SUGGESTIONS: usize = 6;

pub const METER_NEAR_PERCENT: usize = 90;

pub const REVEALED_SELECTOR: &str = ".post-card, .composer";

pub const REVEAL_MARGIN_PROPERTY: &str = "--reveal-margin";

pub const REVEAL_DELAY_MS: u32 = 32;

pub const COPY_FEEDBACK_MS: u32 = 2400;

pub const NO_INDEXED_RANGE: &str = "None";

pub const NEVER_CHECKED: &str = "Never";

pub const UNKNOWN_HEIGHT: &str = "Unknown";
