use crate::errors::NativeError;
use urma_chain::observation::Chain as Network;
use urma_identity::{
    error::IdentityError,
    identity::IdentityKey,
    vault::{EncryptedVault, UnlockCredential, UnlockedVault},
};
use wire_app_core::model::{AuthorId, Chain, WalletSummary};

pub struct WireWallet {
    vault: UnlockedVault,
}

pub struct Balance {
    pub confirmed: u64,
    pub unconfirmed: u64,
    pub outputs: usize,
    pub tip_height: u64,
}

impl WireWallet {
    pub fn from_vault(vault: UnlockedVault) -> Self {
        Self { vault }
    }

    pub fn unlock(bytes: &[u8], credential: UnlockCredential<'_>) -> Result<Self, IdentityError> {
        let vault = EncryptedVault::from_bytes(bytes)?.unlock(credential)?;
        Ok(Self::from_vault(vault))
    }

    pub fn signer(&self) -> Result<IdentityKey, IdentityError> {
        self.vault.keyring().active()
    }

    pub fn author_id(&self) -> Result<AuthorId, IdentityError> {
        Ok(AuthorId(self.signer()?.author().0.serialize()))
    }

    pub fn receive_address(&self, chain: Chain) -> Result<String, NativeError> {
        let network = match chain {
            Chain::Bitcoin => Network::BitcoinTestnet4,
            Chain::Litecoin => Network::LitecoinTestnet,
        };
        Ok(urma_wallet::address::receive_address(
            &self.signer()?,
            network,
        )?)
    }

    pub fn summary(&self, chain: Chain, balance: Balance) -> Result<WalletSummary, NativeError> {
        let author = self.author_id()?;
        Ok(WalletSummary {
            author_pubkey: author.to_hex(),
            author_fingerprint: author.fingerprint(),
            receive_address: self.receive_address(chain)?,
            chain,
            network_name: chain.network_name().to_owned(),
            confirmed_balance_sats: balance.confirmed,
            unconfirmed_balance_sats: balance.unconfirmed,
            utxo_count: balance.outputs,
            tip_height: balance.tip_height,
        })
    }
}
