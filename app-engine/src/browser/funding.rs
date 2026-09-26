use crate::browser::session::{with_signer, Epoch};
use crate::browser::source::{Presence, Source};
use crate::config::{COINBASE_MATURITY, MAX_UTXOS, NETWORK};
use crate::errors::BrowserError;
use bitcoin::{ScriptBuf, Txid};
use serde_json::Value;
use std::collections::HashSet;
use urma_wallet::funding::Funding;
use wire_app_core::model::{AuthorId, Chain, WalletSummary};

pub struct Spendable {
    pub funding: Funding,
    pub value: u64,
}

enum Checked {
    Spendable(Spendable),
    Skipped,
}

pub async fn outputs(
    source: &Source,
    address: &str,
    script: &ScriptBuf,
) -> Result<Vec<Spendable>, BrowserError> {
    let (tip, anchor) = source.tip().await?;
    let listed = source.json(&format!("/address/{address}/utxo")).await?;
    let Some(rows) = listed.as_array() else {
        return Err(BrowserError::UtxoResponse);
    };
    if rows.len() > MAX_UTXOS {
        return Err(BrowserError::UtxoCapacity);
    }
    let mut spendable = Vec::new();
    let mut seen = HashSet::new();
    for row in rows {
        if row["status"]["confirmed"] != true {
            continue;
        }
        match checked(source, row, script, tip, &mut seen).await? {
            Checked::Spendable(output) => spendable.push(output),
            Checked::Skipped => (),
        }
    }
    if source.hash(tip).await? != anchor {
        return Err(BrowserError::FundingChanged);
    }
    spendable.sort_by_key(|output| {
        (
            output.value,
            output.funding.raw_transaction.clone(),
            output.funding.vout,
        )
    });
    Ok(spendable)
}

async fn checked(
    source: &Source,
    row: &Value,
    script: &ScriptBuf,
    tip: u64,
    seen: &mut HashSet<(Txid, u32)>,
) -> Result<Checked, BrowserError> {
    let Some(id) = row["txid"].as_str() else {
        return Err(BrowserError::UtxoField("ID"));
    };
    let id: Txid = id.parse()?;
    let Some(vout) = row["vout"].as_u64() else {
        return Err(BrowserError::UtxoField("output index"));
    };
    let vout = u32::try_from(vout)?;
    let Some(value) = row["value"].as_u64() else {
        return Err(BrowserError::UtxoField("amount"));
    };
    if !seen.insert((id, vout)) {
        return Err(BrowserError::DuplicateFunding);
    }
    let tx = source.transaction(id).await?;
    let Some(previous) = tx.output.get(usize::try_from(vout)?) else {
        return Err(BrowserError::UtxoAbsent);
    };
    if previous.value.to_sat() != value || &previous.script_pubkey != script {
        return Err(BrowserError::FundingMismatch);
    }
    let Presence::Confirmed { height, .. } = source.presence(id).await? else {
        return Ok(Checked::Skipped);
    };
    let Some(behind) = tip.checked_sub(height) else {
        return Err(BrowserError::FundingReorg);
    };
    if tx.is_coinbase() && behind + 1 < COINBASE_MATURITY {
        return Ok(Checked::Skipped);
    }
    let spend = source.json(&format!("/tx/{id}/outspend/{vout}")).await?;
    match spend["spent"].as_bool() {
        Some(true) => return Ok(Checked::Skipped),
        Some(false) => (),
        None => return Err(BrowserError::SpendStatus),
    }
    Ok(Checked::Spendable(Spendable {
        funding: Funding {
            raw_transaction: hex::encode(bitcoin::consensus::serialize(&tx)),
            vout,
        },
        value,
    }))
}

pub async fn wallet(source: &Source) -> Result<WalletSummary, BrowserError> {
    let (author, address, script) = with_signer(Epoch::Any, |signer| {
        Ok((
            AuthorId(signer.author().0.serialize()),
            urma_wallet::address::receive_address(signer, NETWORK)?,
            urma_wallet::signing::script(signer)?,
        ))
    })?;
    let spendable = outputs(source, &address, &script).await?;
    let mut balance: u64 = 0;
    for output in &spendable {
        balance = balance
            .checked_add(output.value)
            .ok_or(BrowserError::BalanceOverflow)?;
    }
    Ok(WalletSummary {
        author_pubkey: author.to_hex(),
        author_fingerprint: author.fingerprint(),
        receive_address: address,
        chain: Chain::Litecoin,
        network_name: Chain::Litecoin.network_name().into(),
        confirmed_balance_sats: balance,
        unconfirmed_balance_sats: 0,
        utxo_count: spendable.len(),
        tip_height: source.tip().await?.0,
    })
}
