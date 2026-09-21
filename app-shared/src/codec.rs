#[cfg(not(target_arch = "wasm32"))]
use urma_core::format::PublicRecord;

use crate::{
    model::{Txid, WireError},
    traits::ProtocolAdapter,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct UrmaProtocolAdapter;

#[cfg(not(target_arch = "wasm32"))]
impl ProtocolAdapter for UrmaProtocolAdapter {
    fn encode_post(&self, text: &str) -> Result<Vec<u8>, WireError> {
        let record = PublicRecord::Post(text.to_owned());
        record.encode().map_err(WireError::from)
    }

    fn decode_post(&self, bytes: &[u8]) -> Result<String, WireError> {
        match PublicRecord::decode(bytes).map_err(WireError::from)? {
            PublicRecord::Post(text) => Ok(text),
            _ => Err(WireError::UnsupportedKind(
                bytes.get(5).copied().unwrap_or(0),
            )),
        }
    }

    fn encode_reply(&self, target: &Txid, text: &str) -> Result<Vec<u8>, WireError> {
        use bitcoin::hashes::Hash;
        let b_txid = bitcoin::Txid::from_byte_array(target.0);
        let record = PublicRecord::Reply {
            target: b_txid,
            text: text.to_owned(),
        };
        record.encode().map_err(WireError::from)
    }

    fn decode_reply(&self, bytes: &[u8]) -> Result<(Txid, String), WireError> {
        use bitcoin::hashes::Hash;
        match PublicRecord::decode(bytes).map_err(WireError::from)? {
            PublicRecord::Reply { target, text } => Ok((Txid(target.to_byte_array()), text)),
            _ => Err(WireError::UnsupportedKind(
                bytes.get(5).copied().unwrap_or(0),
            )),
        }
    }

    fn encode_profile(&self, pseudonym: &str) -> Result<Vec<u8>, WireError> {
        let record = PublicRecord::Profile(pseudonym.to_owned());
        record.encode().map_err(WireError::from)
    }

    fn decode_profile(&self, bytes: &[u8]) -> Result<String, WireError> {
        match PublicRecord::decode(bytes).map_err(WireError::from)? {
            PublicRecord::Profile(p) => Ok(p),
            _ => Err(WireError::UnsupportedKind(
                bytes.get(5).copied().unwrap_or(0),
            )),
        }
    }

    fn encode_avatar(&self, pixels: &[u8; 512]) -> Result<Vec<u8>, WireError> {
        let record = PublicRecord::Avatar(Box::new(*pixels));
        record.encode().map_err(WireError::from)
    }

    fn decode_avatar(&self, bytes: &[u8]) -> Result<[u8; 512], WireError> {
        match PublicRecord::decode(bytes).map_err(WireError::from)? {
            PublicRecord::Avatar(pixels) => Ok(*pixels),
            _ => Err(WireError::UnsupportedKind(
                bytes.get(5).copied().unwrap_or(0),
            )),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl ProtocolAdapter for UrmaProtocolAdapter {
    fn encode_post(&self, _text: &str) -> Result<Vec<u8>, WireError> {
        Err(WireError::InvalidState(
            "protocol encoding not available on wasm".into(),
        ))
    }

    fn decode_post(&self, _bytes: &[u8]) -> Result<String, WireError> {
        Err(WireError::InvalidState(
            "protocol decoding not available on wasm".into(),
        ))
    }

    fn encode_reply(&self, _target: &Txid, _text: &str) -> Result<Vec<u8>, WireError> {
        Err(WireError::InvalidState(
            "protocol encoding not available on wasm".into(),
        ))
    }

    fn decode_reply(&self, _bytes: &[u8]) -> Result<(Txid, String), WireError> {
        Err(WireError::InvalidState(
            "protocol decoding not available on wasm".into(),
        ))
    }

    fn encode_profile(&self, _pseudonym: &str) -> Result<Vec<u8>, WireError> {
        Err(WireError::InvalidState(
            "protocol encoding not available on wasm".into(),
        ))
    }

    fn decode_profile(&self, _bytes: &[u8]) -> Result<String, WireError> {
        Err(WireError::InvalidState(
            "protocol decoding not available on wasm".into(),
        ))
    }

    fn encode_avatar(&self, _pixels: &[u8; 512]) -> Result<Vec<u8>, WireError> {
        Err(WireError::InvalidState(
            "protocol encoding not available on wasm".into(),
        ))
    }

    fn decode_avatar(&self, _bytes: &[u8]) -> Result<[u8; 512], WireError> {
        Err(WireError::InvalidState(
            "protocol decoding not available on wasm".into(),
        ))
    }
}

/// Backwards-compatible export for `UrmaProtocolAdapter`.
pub use UrmaProtocolAdapter as ProtocolCodec;
