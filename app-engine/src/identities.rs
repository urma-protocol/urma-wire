use std::collections::BTreeMap;
use wire_app_core::feed::PublicIdentity;

pub enum Named {
    Nickname(String),
    Avatar(Vec<u8>),
}

pub struct Naming {
    pub height: u64,
    pub position: u32,
    pub author: String,
    pub named: Named,
}

pub fn gathered(mut namings: Vec<Naming>) -> BTreeMap<String, PublicIdentity> {
    namings.sort_by(|left, right| {
        (left.author.as_str(), left.height, left.position).cmp(&(
            right.author.as_str(),
            right.height,
            right.position,
        ))
    });
    let mut identities = BTreeMap::new();
    for group in namings.chunk_by(|left, right| left.author == right.author) {
        let [first, ..] = group else {
            continue;
        };
        let mut identity = PublicIdentity::default();
        for naming in group {
            match &naming.named {
                Named::Nickname(nickname) => identity.nickname = Some(nickname.clone()),
                Named::Avatar(pixels) => identity.avatar = Some(pixels.clone()),
            }
        }
        identities.insert(first.author.clone(), identity);
    }
    identities
}
