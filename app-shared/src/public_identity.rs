use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use urma_core::format::PublicRecord;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PublicIdentity {
    pub nickname: Option<String>,
    pub avatar: Option<Vec<u8>>,
}

/// Rebuild disposable identity state from the selected network's verified index.
/// Sorting explicitly makes transaction position decisive even after replay.
pub fn resolve<'a>(
    entries: impl IntoIterator<Item = (u64, u32, &'a str, &'a str)>,
) -> Result<BTreeMap<String, PublicIdentity>, String> {
    let mut entries: Vec<_> = entries.into_iter().collect();
    entries.sort_by_key(|entry| (entry.0, entry.1));
    let mut identities = BTreeMap::<String, PublicIdentity>::new();
    for (_, _, author, record) in entries {
        let record = PublicRecord::decode(&hex::decode(record).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        match record {
            PublicRecord::Profile(nickname) => {
                identities.entry(author.into()).or_default().nickname = Some(nickname)
            }
            PublicRecord::Avatar(pixels) => {
                identities.entry(author.into()).or_default().avatar = Some(pixels.to_vec())
            }
            _ => (),
        }
    }
    Ok(identities)
}
