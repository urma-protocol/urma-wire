use super::{
    browser,
    browser_source::{error, Presence, Source, CHAIN},
    browser_store as store,
};
use bitcoin::{consensus::deserialize, ScriptBuf, Transaction, Txid};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use urma_core::envelope;
use urma_runtime::{plan::PublicationPlan, publication};
use urma_wallet::{
    funding::Funding,
    wallet::{FeeBudget, FeeRate},
};
use wire_app_shared::{live_model::Publication, Chain, WalletSummary};

const ACTIVE: &str = "ltc-testnet-active-plan";
#[derive(Serialize, Deserialize)]
pub struct Active {
    plan: PublicationPlan,
    approved: Option<String>,
    attempted: Vec<String>,
    observations: Vec<Presence>,
    last_error: Option<String>,
}
impl Active {
    fn validate(&self) -> Result<(), String> {
        self.plan.validate().map_err(error)?;
        if !matches!(
            self.plan.chain,
            urma_chain::observation::Chain::LitecoinTestnet
        ) || self.plan.records.len() != 1
        {
            return Err("Active plan must contain one Litecoin Testnet record".into());
        }
        if let Some(approval) = &self.approved {
            if *approval != self.plan.id().map_err(error)? {
                return Err("Stored approval does not match the exact signed plan".into());
            }
        }
        Ok(())
    }
}
pub async fn active() -> Result<Option<Active>, String> {
    let active: Option<Active> = store::read(ACTIVE).await?;
    if let Some(active) = &active {
        active.validate()?;
    }
    Ok(active)
}
fn transaction(raw: &str) -> Result<Transaction, String> {
    deserialize(&hex::decode(raw).map_err(error)?).map_err(error)
}

pub async fn outputs(
    source: &Source,
    address: &str,
    script: &ScriptBuf,
) -> Result<Vec<(Funding, u64)>, String> {
    let (tip, anchor) = source.tip().await?;
    let rows = source.json(&format!("/address/{address}/utxo")).await?;
    let rows = rows.as_array().ok_or("Invalid UTXO response")?;
    if rows.len() > 1000 {
        return Err("Source UTXO capacity exceeded".into());
    }
    let mut outputs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for row in rows {
        if row["status"]["confirmed"] != true {
            continue;
        }
        let id: Txid = row["txid"]
            .as_str()
            .ok_or("UTXO missing ID")?
            .parse()
            .map_err(error)?;
        let vout: u32 = row["vout"]
            .as_u64()
            .ok_or("UTXO missing output index")?
            .try_into()
            .map_err(error)?;
        let value = row["value"].as_u64().ok_or("UTXO missing amount")?;
        if !seen.insert((id, vout)) {
            return Err("Source returned duplicate funding outputs".into());
        }
        let tx = source.transaction(id).await?;
        let previous = tx.output.get(vout as usize).ok_or("UTXO output absent")?;
        if previous.value.to_sat() != value || &previous.script_pubkey != script {
            return Err("Funding amount or script disagrees with identity transaction".into());
        }
        let Presence::Confirmed { height, .. } = source.presence(id).await? else {
            continue;
        };
        let confirmations = tip
            .checked_sub(height)
            .ok_or("Chain changed during funding verification")?
            + 1;
        if tx.is_coinbase() && confirmations < 100 {
            continue;
        }
        let spend = source.json(&format!("/tx/{id}/outspend/{vout}")).await?;
        match spend["spent"].as_bool() {
            Some(true) => continue,
            Some(false) => {}
            None => return Err("Invalid funding spend status".into()),
        }
        outputs.push((
            Funding {
                raw_transaction: hex::encode(bitcoin::consensus::serialize(&tx)),
                vout,
            },
            value,
        ));
    }
    if source.hash(tip).await? != anchor {
        return Err("Chain changed during funding verification; retry".into());
    }
    outputs.sort_by_key(|(funding, value)| (*value, funding.raw_transaction.clone(), funding.vout));
    Ok(outputs)
}

pub async fn wallet(source: &Source) -> Result<WalletSummary, String> {
    let (author, address, script) = browser::with_signer(None, |signer| {
        Ok((
            signer.author().0.to_string(),
            urma_wallet::address::receive_address(signer, CHAIN).map_err(error)?,
            urma_wallet::signing::script(signer).map_err(error)?,
        ))
    })?;
    let outputs = outputs(source, &address, &script).await?;
    let balance = outputs
        .iter()
        .try_fold(0u64, |sum, (_, value)| sum.checked_add(*value))
        .ok_or("Balance overflow")?;
    Ok(WalletSummary {
        author_fingerprint: author[..8].into(),
        author_pubkey: author,
        receive_address: address,
        chain: Chain::Litecoin,
        network_name: Chain::Litecoin.network_name().into(),
        confirmed_balance_sats: balance,
        unconfirmed_balance_sats: 0,
        utxo_count: outputs.len(),
        tip_height: source.tip().await?.0,
    })
}

pub async fn prepare(source: &Source, args: &Value) -> Result<Publication, String> {
    if let Some(old) = active().await? {
        if old.approved.is_some() && !observe(source, &old).await?.complete {
            return Err(
                "Finish the approved publication before preparing another; use Resume".into(),
            );
        }
    }
    let text = args["text"].as_str().ok_or("Missing post text")?.to_owned();
    let target = args["target"].as_str().map(str::to_owned);
    let record = wire_app_shared::publication_record::build(
        args["kind"].as_str().unwrap_or("post"),
        text,
        target,
        args["wire"].as_str().unwrap_or("").into(),
        serde_json::from_value(
            args.get("hashtags")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
        )
        .map_err(error)?,
        args.get("avatar")
            .filter(|value| !value.is_null())
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(error)?,
    )?
    .encode()
    .map_err(error)?;
    let epoch = browser::session_epoch();
    let (author, address, script) = browser::with_signer(Some(epoch), |signer| {
        Ok((
            signer.author().0,
            urma_wallet::address::receive_address(signer, CHAIN).map_err(error)?,
            urma_wallet::signing::script(signer).map_err(error)?,
        ))
    })?;
    let fee = publication::quote_record(&record, author, &script, 1).map_err(error)?;
    let minimum = fee
        .checked_add(2 * urma_runtime::config::publication_return(CHAIN))
        .ok_or("Funding budget overflow")?;
    let funding = outputs(source, &address, &script)
        .await?
        .into_iter()
        .find(|(_, value)| *value >= minimum)
        .ok_or("No single confirmed spendable output covers the exact fee and return outputs")?
        .0;
    // Reborrow the session only for synchronous signing; no key spans a network await.
    let pair = browser::with_signer(Some(epoch), |signer| {
        publication::prepare_signed_bytes(
            &record,
            signer,
            funding,
            CHAIN,
            FeeBudget {
                chain: CHAIN.genesis().map_err(error)?,
                rate: FeeRate(std::num::NonZeroU64::new(1).unwrap()),
                maximum_base_units: fee,
            },
        )
        .map_err(error)
    })?;
    let reveal = transaction(&pair.plan.reveal)?;
    let plan = PublicationPlan {
        version: 1,
        chain: CHAIN,
        author: author.to_string(),
        root_txid: reveal.compute_txid().to_string(),
        total_fee: pair.fee,
        maximum_fee: fee,
        records: vec![pair.plan],
    };
    plan.validate().map_err(error)?;
    let active = Active {
        plan,
        approved: None,
        attempted: vec![],
        observations: vec![],
        last_error: None,
    };
    store::write(ACTIVE, &active).await?;
    observe(source, &active).await
}

pub async fn observe(source: &Source, active: &Active) -> Result<Publication, String> {
    active.validate()?;
    let anchor = source.tip().await?;
    let pair = &active.plan.records[0];
    let mut confirmed = 0;
    let mut mempool = 0;
    for raw in [&pair.commit, &pair.reveal] {
        match source.presence(transaction(raw)?.compute_txid()).await? {
            Presence::Confirmed { .. } => confirmed += 1,
            Presence::Mempool => mempool += 1,
            Presence::Missing => {}
        }
    }
    if source.hash(anchor.0).await? != anchor.1 {
        return Err("Chain changed during publication observation; resume".into());
    }
    projection(active, confirmed, mempool)
}

pub fn offline_status(active: &Active) -> Result<Publication, String> {
    let mut publication = projection(active, 0, 0)?;
    publication.message = "Saved exact signed plan. Chain source unavailable; current confirmation status is unknown. Reconnect and Resume to reconcile.".into();
    Ok(publication)
}

fn projection(active: &Active, confirmed: usize, mempool: usize) -> Result<Publication, String> {
    active.validate()?;
    let pair = &active.plan.records[0];
    let commit = transaction(&pair.commit)?;
    let reveal = transaction(&pair.reveal)?;
    let parsed = envelope::verify_reveal(&reveal, &commit).map_err(error)?;
    let wire_app_shared::publication_record::Review {
        kind,
        text,
        target,
        wire,
        hashtags,
        avatar,
    } = wire_app_shared::publication_record::review(&parsed.record)?;
    let funding = pair.funding.prevout().map_err(error)?.1.value.to_sat();
    Ok(Publication {kind, avatar, id: active.plan.id().map_err(error)?, txid: active.plan.root_txid.clone(), text, wire, hashtags, target, fee: active.plan.total_fee, selected_funding: funding, returned: funding.checked_sub(active.plan.total_fee).ok_or("Invalid plan fee")?, approved: active.approved.is_some(), confirmed, mempool, complete: confirmed == 2,
        message: if confirmed == 2 {"Confirmed on Litecoin Testnet"} else if active.approved.is_none() {"Prepared. Review the exact record and fee before approving."} else {"Waiting for blockchain confirmation; Esplora provides no mempool preflight. Exact signed bytes and fee remain unchanged."}.into()})
}

pub async fn advance(source: &Source, id: &str, approve: bool) -> Result<Publication, String> {
    let mut active = active().await?.ok_or("No prepared publication")?;
    if active.plan.id().map_err(error)? != id {
        return Err("Approval does not match the current exact plan".into());
    }
    if approve {
        active.approved = Some(id.into());
        store::write(ACTIVE, &active).await?;
    }
    if active.approved.as_deref() != Some(id) {
        return Err("Exact plan approval required before broadcast".into());
    }
    let result = advance_inner(source, &mut active).await;
    if let Err(cause) = &result {
        active.last_error = Some(cause.clone());
        store::write(ACTIVE, &active).await?;
    }
    result
}

async fn advance_inner(source: &Source, active: &mut Active) -> Result<Publication, String> {
    active.validate()?;
    active.observations.clear();
    active.last_error = None;
    let anchor = source.tip().await?;
    let pair = &active.plan.records[0];
    for raw in [pair.commit.clone(), pair.reveal.clone()] {
        let tx = transaction(&raw)?;
        let txid = tx.compute_txid();
        let mut presence = source.presence(txid).await?;
        if presence == Presence::Missing {
            // Journal intent before submission so a lost response/reload resumes these same bytes.
            if !active.attempted.contains(&txid.to_string()) {
                active.attempted.push(txid.to_string());
            }
            store::write(ACTIVE, active).await?;
            source.broadcast(&tx).await?;
            presence = source.presence(txid).await?;
        }
        let missing = matches!(presence, Presence::Missing);
        active.observations.push(presence);
        store::write(ACTIVE, active).await?;
        if missing {
            return Err(
                "Transaction missing after submission; resume the exact approved plan".into(),
            );
        }
    }
    if source.hash(anchor.0).await? != anchor.1 {
        return Err("Chain changed during publication reconciliation; resume exact plan".into());
    }
    observe(source, active).await
}
