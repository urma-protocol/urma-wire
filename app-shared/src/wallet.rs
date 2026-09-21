use crate::model::{AuthorId, Chain, WalletSummary, WireError};
use urma_chain::observation::Chain as ProtocolChain;
use urma_identity::{
    identity::{IdentityKey, IdentitySlot},
    keyring::Keyring,
    vault::{EncryptedVault, UnlockCredential, UnlockedVault},
};

pub struct WireWallet {
    vault: UnlockedVault,
}

impl WireWallet {
    pub fn from_vault(vault: UnlockedVault) -> Self {
        Self { vault }
    }
    pub fn unlock(bytes: &[u8], credential: UnlockCredential<'_>) -> Result<Self, WireError> {
        let vault = EncryptedVault::from_bytes(bytes)?.unlock(credential)?;
        Ok(Self::from_vault(vault))
    }
    pub fn keyring(&self) -> &Keyring {
        self.vault.keyring()
    }
    pub fn signer(&self) -> Result<IdentityKey, WireError> {
        Ok(self.vault.keyring().active()?)
    }
    pub fn select(&mut self, slot: IdentitySlot) -> Result<(), WireError> {
        self.vault.keyring_mut().select(slot)?;
        Ok(())
    }
    pub fn encrypted_backup<R: rand::RngCore + rand::CryptoRng>(
        &self,
        rng: &mut R,
    ) -> Result<EncryptedVault, WireError> {
        Ok(self.vault.seal(rng)?)
    }
    pub fn author_id(&self) -> Result<AuthorId, WireError> {
        Ok(AuthorId(self.signer()?.author().0.serialize()))
    }
    pub fn get_receive_address(&self, chain: Chain) -> Result<String, WireError> {
        let protocol_chain = match chain {
            Chain::Bitcoin => ProtocolChain::BitcoinTestnet4,
            Chain::Litecoin => ProtocolChain::LitecoinTestnet,
        };
        Ok(urma_wallet::address::receive_address(
            &self.signer()?,
            protocol_chain,
        )?)
    }
    pub fn summary(
        &self,
        chain: Chain,
        balance_confirmed: u64,
        balance_unconfirmed: u64,
        utxo_count: usize,
        tip_height: u64,
    ) -> Result<WalletSummary, WireError> {
        let author = self.author_id()?;
        Ok(WalletSummary {
            author_pubkey: author.to_hex(),
            author_fingerprint: author.fingerprint(),
            receive_address: self.get_receive_address(chain)?,
            chain,
            network_name: chain.network_name().to_string(),
            confirmed_balance_sats: balance_confirmed,
            unconfirmed_balance_sats: balance_unconfirmed,
            utxo_count,
            tip_height,
        })
    }
}
