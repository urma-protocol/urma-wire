use rand::rngs::OsRng;
use urma_identity::error::IdentityError;
use urma_identity::keyring::Keyring;
use urma_identity::phrase::IdentityPhrase;
use urma_identity::vault::UnlockedVault;
use zeroize::Zeroizing;

pub enum Words {
    Fresh,
    Given(Zeroizing<String>),
}

pub struct Minted {
    pub vault: UnlockedVault,
    pub sealed: Vec<u8>,
    pub words: Zeroizing<String>,
}

pub fn minted(words: Words, password: &str) -> Result<Minted, IdentityError> {
    let phrase = match words {
        Words::Fresh => IdentityPhrase::generate(&mut OsRng)?,
        Words::Given(text) => IdentityPhrase::parse(text.trim())?,
    };
    let exported = phrase.export_words()?;
    let vault = UnlockedVault::create(Keyring::create(&phrase, "wire")?, password, &mut OsRng)?;
    let sealed = vault.seal(&mut OsRng)?.as_bytes().to_vec();
    Ok(Minted {
        vault,
        sealed,
        words: exported,
    })
}
