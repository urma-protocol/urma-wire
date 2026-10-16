use crate::errors::NativeError;
use bitcoin::ScriptBuf;
use urma_chain::observation::Chain as Network;
use urma_identity::{
    error::IdentityError,
    identity::IdentityKey,
    vault::{EncryptedVault, UnlockCredential, UnlockedVault},
};
use urma_runtime::wallet::Coin;
use urma_wallet::account::Account;
use wire_app_core::model::{AuthorId, Chain, WalletSummary};

pub struct WireWallet {
    vault: UnlockedVault,
}

pub enum Known {
    Coins { receive: ScriptBuf, coins: Vec<Coin> },
    Nothing,
}

pub struct Balance {
    pub confirmed: u64,
    pub unconfirmed: u64,
    pub outputs: usize,
    pub tip_height: u64,
}

fn network(chain: Chain) -> Network {
    match chain {
        Chain::Bitcoin => Network::BitcoinTestnet4,
        Chain::Litecoin => Network::LitecoinTestnet,
    }
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

    pub fn funder(&self, chain: Chain) -> Result<Account, NativeError> {
        Ok(Account::from_root(
            network(chain),
            self.vault.keyring().root(),
        )?)
    }

    pub fn author_id(&self) -> Result<AuthorId, IdentityError> {
        Ok(AuthorId(self.signer()?.author().0.serialize()))
    }

    pub fn receive_address(&self, chain: Chain) -> Result<String, NativeError> {
        Ok(self.funder(chain)?.receive_address()?)
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
