use crate::browser::session::{funder, with_signer, Epoch};
use crate::browser::source::{Presence, Source};
use crate::config::{COINBASE_MATURITY, MAX_UTXOS};
use crate::errors::BrowserError;
use bitcoin::Txid;
use serde_json::Value;
use std::cmp::Reverse;
use std::collections::HashSet;
use urma_runtime::config::{WALLET_GAP, WALLET_RANGE, WALLET_VERIFY_OUTPUTS};
use urma_runtime::wallet::FundingInput;
use urma_wallet::account::{Account, Branch, KeyPath};
use urma_wallet::funding::Funding;
use wire_app_core::model::{AuthorId, Chain, WalletSummary};

pub struct Spendable {
    pub input: FundingInput,
    pub value: u64,
}

struct Listed {
    txid: Txid,
    vout: u32,
    value: u64,
    path: KeyPath,
}

enum Checked {
    Spendable(Spendable),
    Skipped,
}

struct Scan<'a> {
    source: &'a Source,
    account: &'a Account,
    seen: HashSet<(Txid, u32)>,
    listed: Vec<Listed>,
}

pub async fn outputs(source: &Source, account: &Account) -> Result<Vec<Spendable>, BrowserError> {
    let (tip, anchor) = source.tip().await?;
    let mut scan = Scan {
        source,
        account,
        seen: HashSet::new(),
        listed: Vec::new(),
    };
    for branch in [Branch::External, Branch::Internal] {
        let mut empty = 0;
        let mut index = 0;
        while empty < WALLET_GAP && index < WALLET_RANGE {
            let used = scan.address(KeyPath::new(branch, index)).await?;
            empty = match used {
                true => 0,
                false => empty + 1,
            };
            index += 1;
        }
    }
    let mut listed = scan.listed;
    listed.sort_by_key(|coin| (Reverse(coin.value), coin.txid, coin.vout));
    let mut spendable = Vec::new();
    for coin in listed.iter().take(WALLET_VERIFY_OUTPUTS) {
        match checked(source, account, coin, tip).await? {
            Checked::Spendable(output) => spendable.push(output),
            Checked::Skipped => (),
        }
    }
    if source.hash(tip).await? != anchor {
        return Err(BrowserError::FundingChanged);
    }
    Ok(spendable)
}

impl Scan<'_> {
    async fn address(&mut self, path: KeyPath) -> Result<bool, BrowserError> {
        let address = self.account.address(path)?;
        let answer = self
            .source
            .json(&format!("/address/{address}/utxo"))
            .await?;
        let Some(rows) = answer.as_array() else {
            return Err(BrowserError::UtxoResponse);
        };
        if rows.len() > MAX_UTXOS {
            return Err(BrowserError::UtxoCapacity);
        }
        for row in rows {
            if row["status"]["confirmed"] != true {
                continue;
            }
            let coin = listed(row, path)?;
            if !self.seen.insert((coin.txid, coin.vout)) {
                return Err(BrowserError::DuplicateFunding);
            }
            self.listed.push(coin);
        }
        Ok(!rows.is_empty())
    }
}

fn listed(row: &Value, path: KeyPath) -> Result<Listed, BrowserError> {
    let Some(id) = row["txid"].as_str() else {
        return Err(BrowserError::UtxoField("ID"));
    };
    let Some(vout) = row["vout"].as_u64() else {
        return Err(BrowserError::UtxoField("output index"));
    };
    let Some(value) = row["value"].as_u64() else {
        return Err(BrowserError::UtxoField("amount"));
    };
    Ok(Listed {
        txid: id.parse()?,
        vout: u32::try_from(vout)?,
        value,
        path,
    })
}

async fn checked(
    source: &Source,
    account: &Account,
    coin: &Listed,
    tip: u64,
) -> Result<Checked, BrowserError> {
    let tx = source.transaction(coin.txid).await?;
    let Some(previous) = tx.output.get(usize::try_from(coin.vout)?) else {
        return Err(BrowserError::UtxoAbsent);
    };
    if previous.value.to_sat() != coin.value
        || previous.script_pubkey != account.script(coin.path)?
    {
        return Err(BrowserError::FundingMismatch);
    }
    let Presence::Confirmed { height, .. } = source.presence(coin.txid).await? else {
        return Ok(Checked::Skipped);
    };
    let Some(behind) = tip.checked_sub(height) else {
        return Err(BrowserError::FundingReorg);
    };
    if tx.is_coinbase() && behind + 1 < COINBASE_MATURITY {
        return Ok(Checked::Skipped);
    }
    let spend = source
        .json(&format!("/tx/{}/outspend/{}", coin.txid, coin.vout))
        .await?;
    match spend["spent"].as_bool() {
        Some(true) => return Ok(Checked::Skipped),
        Some(false) => (),
        None => return Err(BrowserError::SpendStatus),
    }
    Ok(Checked::Spendable(Spendable {
        input: FundingInput {
            funding: Funding {
                raw_transaction: hex::encode(bitcoin::consensus::serialize(&tx)),
                vout: coin.vout,
            },
            path: coin.path,
        },
        value: coin.value,
    }))
}

pub async fn wallet(source: &Source) -> Result<WalletSummary, BrowserError> {
    let author = with_signer(Epoch::Any, |signer| {
        Ok(AuthorId(signer.author().0.serialize()))
    })?;
    let account = funder(Epoch::Any)?;
    let spendable = outputs(source, &account).await?;
    let mut balance: u64 = 0;
    for output in &spendable {
        balance = balance
            .checked_add(output.value)
            .ok_or(BrowserError::BalanceOverflow)?;
    }
    Ok(WalletSummary {
        author_pubkey: author.to_hex(),
        author_fingerprint: author.fingerprint(),
        receive_address: account.receive_address()?,
        chain: Chain::Litecoin,
        network_name: Chain::Litecoin.network_name().into(),
        confirmed_balance_sats: balance,
        unconfirmed_balance_sats: 0,
        utxo_count: spendable.len(),
        tip_height: source.tip().await?.0,
    })
}
