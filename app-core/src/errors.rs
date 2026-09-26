use serde::{Deserialize, Serialize};
use std::num::TryFromIntError;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum IdError {
    #[error("hex parsing error")]
    Hex(#[from] hex::FromHexError),
    #[error("invalid length: expected {expected}, got {got}")]
    Length { expected: usize, got: usize },
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ComposeError {
    #[error("text offset overflow")]
    Offset,
    #[error("caret precedes its token")]
    Caret,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ImageUrlError {
    #[error("Image URL exceeds {limit} bytes")]
    TooLong { limit: usize },
    #[error("Invalid image URL")]
    Unparsable(#[from] url::ParseError),
    #[error("External images require HTTP or HTTPS without credentials")]
    Unsafe,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ImageSizeError {
    #[error("image dimension does not fit this platform")]
    Dimension(#[from] TryFromIntError),
    #[error("image display size overflows this platform")]
    Overflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaultKind {
    Identity,
    Source,
    Storage,
    Publication,
    Media,
    Refused,
    Unsupported,
    Transport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{message}")]
pub struct Fault {
    pub kind: FaultKind,
    pub message: String,
}

impl Fault {
    pub fn new(kind: FaultKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn unanswered(asked: &str, answered: &str) -> Self {
        Self::new(
            FaultKind::Transport,
            format!("The Wire engine answered {answered} to {asked}"),
        )
    }
}
