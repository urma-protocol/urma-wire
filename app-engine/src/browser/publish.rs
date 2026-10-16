use crate::browser::funding::{outputs, Spendable};
use crate::browser::session::{funder, session_epoch, with_signer, Epoch};
use crate::browser::source::{Presence, Source};
use crate::browser::store::{self, Stored};
use crate::config::{ACTIVE_KEY, FEE_HEADROOM, FEE_RATE, NETWORK};
use crate::errors::BrowserError;
use crate::record::{build, review};
use bitcoin::consensus::deserialize;
use bitcoin::Transaction;
use serde::{Deserialize, Serialize};
use urma_chain::observation::Chain as Network;
use urma_core::envelope;
use urma_runtime::config::{publication_return, WALLET_MAX_INPUTS};
use urma_runtime::plan::PublicationPlan;
use urma_runtime::publication;
use urma_runtime::wallet::FundingInput;
use urma_wallet::wallet::{FeeBudget, FeeRate};
use wire_app_core::publication::{Content, Publication};

#[derive(Serialize, Deserialize)]
pub struct Active {
    plan: PublicationPlan,
    approved: Option<String>,
    attempted: Vec<String>,
    observations: Vec<Presence>,
    last_error: Option<String>,
}

impl Active {
    fn validate(&self) -> Result<(), BrowserError> {
        self.plan.validate()?;
        if !matches!(self.plan.chain, Network::LitecoinTestnet) || self.plan.records.len() != 1 {
            return Err(BrowserError::ForeignPlan);
        }
        for approval in self.approved.iter() {
            if *approval != self.plan.id()? {
                return Err(BrowserError::ApprovalMismatch);
            }
        }
        Ok(())
    }

    fn approved(&self) -> bool {
        let Some(_approval) = &self.approved else {
            return false;
        };
        true
    }

    fn approves(&self, id: &str) -> bool {
        let Some(approval) = &self.approved else {
            return false;
        };
        approval == id
    }
}

pub enum Planned {
    Active(Active),
    Idle,
}

pub async fn active() -> Result<Planned, BrowserError> {
    let stored = match store::read::<Active>(ACTIVE_KEY).await {
        Ok(stored) => stored,
        Err(BrowserError::Json(error)) => {
            tracing::warn!(error = %error, "saved publication cannot be read; it is dropped");
            return forget().await;
        }
        Err(other) => return Err(other),
    };
    let Stored::Found(active) = stored else {
        return Ok(Planned::Idle);
    };
    match active.validate() {
        Ok(()) => Ok(Planned::Active(active)),
        Err(error) => {
            tracing::warn!(error = %error, "saved publication fails validation; it is dropped");
            forget().await
        }
    }
}

async fn forget() -> Result<Planned, BrowserError> {
    store::delete(ACTIVE_KEY).await?;
    Ok(Planned::Idle)
}

pub async fn discard(id: &str) -> Result<(), BrowserError> {
    let Planned::Active(active) = active().await? else {
        return Ok(());
    };
    if active.plan.id()? != id {
        return Ok(());
    }
    if active.approved() {
        return Err(BrowserError::Unfinished);
    }
    store::delete(ACTIVE_KEY).await
}

fn transaction(raw: &str) -> Result<Transaction, BrowserError> {
    Ok(deserialize(&hex::decode(raw)?)?)
}

async fn refuse_while_unfinished(source: &Source) -> Result<(), BrowserError> {
    let Planned::Active(old) = active().await? else {
        return Ok(());
    };
    if old.approved() && !observe(source, &old).await?.complete {
        return Err(BrowserError::Unfinished);
    }
    Ok(())
}

pub async fn quote(source: &Source, content: &Content) -> Result<Publication, BrowserError> {
    refuse_while_unfinished(source).await?;
    let record = build(content)?.encode()?;
    let epoch = session_epoch();
    let author = with_signer(Epoch::Exactly(epoch), |signer| Ok(signer.author().0))?;
    let account = funder(Epoch::Exactly(epoch))?;
    let return_script = account.receive_script()?;
    let quoted = publication::quote_record(&record, author, &return_script, FEE_RATE.get())?;
    let maximum = quoted
        .checked_add(FEE_HEADROOM)
        .ok_or(BrowserError::BudgetOverflow)?;
    let minimum = maximum
        .checked_add(2 * publication_return(NETWORK))
        .ok_or(BrowserError::BudgetOverflow)?;
    let inputs = selected(outputs(source, &account).await?, minimum)?;
    let pair = with_signer(Epoch::Exactly(epoch), |signer| {
        Ok(publication::prepare_signed_bytes(
            &record,
            signer,
            &account,
            &inputs,
            FeeBudget {
                chain: NETWORK.genesis()?,
                rate: FeeRate(FEE_RATE),
                maximum_base_units: maximum,
            },
        )?)
    })?;
    let reveal = transaction(&pair.plan.reveal)?;
    let plan = PublicationPlan {
        version: 1,
        chain: NETWORK,
        author: author.to_string(),
        return_script: hex::encode(return_script.as_bytes()),
        root_txid: reveal.compute_txid().to_string(),
        total_fee: pair.fee,
        maximum_fee: maximum,
        records: vec![pair.plan],
    };
    plan.validate()?;
    plan.require_funder(&account)?;
    let active = Active {
        plan,
        approved: None,
        attempted: vec![],
        observations: vec![],
        last_error: None,
    };
    store::write(ACTIVE_KEY, &active).await?;
    observe(source, &active).await
}

fn selected(spendable: Vec<Spendable>, minimum: u64) -> Result<Vec<FundingInput>, BrowserError> {
    let mut inputs = Vec::new();
    let mut total: u64 = 0;
    for output in spendable {
        if total >= minimum {
            break;
        }
        if inputs.len() >= WALLET_MAX_INPUTS {
            return Err(BrowserError::NoFundingOutput);
        }
        total = total
            .checked_add(output.value)
            .ok_or(BrowserError::BalanceOverflow)?;
        inputs.push(output.input);
    }
    match total >= minimum {
        true => Ok(inputs),
        false => Err(BrowserError::NoFundingOutput),
    }
}

fn selected_funding(active: &Active) -> Result<u64, BrowserError> {
    let mut total: u64 = 0;
    for funding in &active.plan.records[0].funding {
        total = total
            .checked_add(funding.prevout()?.1.value.to_sat())
            .ok_or(BrowserError::InvalidPlanFee)?;
    }
    Ok(total)
}

pub async fn observe(source: &Source, active: &Active) -> Result<Publication, BrowserError> {
    active.validate()?;
    let anchor = source.tip().await?;
    let pair = &active.plan.records[0];
    let mut confirmed = 0;
    let mut mempool = 0;
    for raw in [&pair.commit, &pair.reveal] {
        match source.presence(transaction(raw)?.compute_txid()).await? {
            Presence::Confirmed { .. } => confirmed += 1,
            Presence::Mempool => mempool += 1,
            Presence::Missing => (),
        }
    }
    if source.hash(anchor.0).await? != anchor.1 {
        return Err(BrowserError::ObservationChanged);
    }
    reviewed(active, confirmed, mempool)
}

pub fn offline_status(active: &Active) -> Result<Publication, BrowserError> {
    let mut publication = reviewed(active, 0, 0)?;
    publication.message = "Saved exact signed plan. Chain source unavailable; current confirmation status is unknown. Reconnect and Resume to reconcile.".into();
    Ok(publication)
}

fn reviewed(
    active: &Active,
    confirmed: usize,
    mempool: usize,
) -> Result<Publication, BrowserError> {
    active.validate()?;
    let pair = &active.plan.records[0];
    let commit = transaction(&pair.commit)?;
    let reveal = transaction(&pair.reveal)?;
    let signed = envelope::verify_reveal(&reveal, &commit)?;
    let funding = selected_funding(active)?;
    let Some(returned) = funding.checked_sub(active.plan.total_fee) else {
        return Err(BrowserError::InvalidPlanFee);
    };
    let approved = active.approved();
    Ok(Publication {
        content: review(&signed.record)?,
        id: active.plan.id()?,
        txid: active.plan.root_txid.clone(),
        fee: active.plan.total_fee,
        selected_funding: funding,
        returned,
        approved,
        confirmed,
        mempool,
        complete: confirmed == 2,
        message: waiting(confirmed, approved).into(),
        abandonable: false,
    })
}

fn waiting(confirmed: usize, approved: bool) -> &'static str {
    if confirmed == 2 {
        return "Confirmed on Litecoin Testnet";
    }
    match approved {
        false => "Prepared. Review the exact record and fee before approving.",
        true => "Waiting for blockchain confirmation; Esplora provides no mempool preflight. Exact signed bytes and fee remain unchanged.",
    }
}

pub async fn advance(
    source: &Source,
    id: &str,
    approve: bool,
) -> Result<Publication, BrowserError> {
    let Planned::Active(mut active) = active().await? else {
        return Err(BrowserError::NoPublication);
    };
    if active.plan.id()? != id {
        return Err(BrowserError::PlanMismatch);
    }
    if approve {
        active.plan.require_funder(&funder(Epoch::Any)?)?;
        active.approved = Some(id.into());
        store::write(ACTIVE_KEY, &active).await?;
    }
    if !active.approves(id) {
        return Err(BrowserError::ApprovalRequired);
    }
    match reconcile(source, &mut active).await {
        Ok(publication) => Ok(publication),
        Err(cause) => {
            active.last_error = Some(cause.to_string());
            store::write(ACTIVE_KEY, &active).await?;
            Err(cause)
        }
    }
}

async fn reconcile(source: &Source, active: &mut Active) -> Result<Publication, BrowserError> {
    active.validate()?;
    active.observations.clear();
    active.last_error = None;
    let anchor = source.tip().await?;
    let pair = &active.plan.records[0];
    let raws = [pair.commit.clone(), pair.reveal.clone()];
    for raw in raws {
        let tx = transaction(&raw)?;
        let txid = tx.compute_txid();
        let mut presence = source.presence(txid).await?;
        if presence == Presence::Missing {
            if !active.attempted.contains(&txid.to_string()) {
                active.attempted.push(txid.to_string());
            }
            store::write(ACTIVE_KEY, &*active).await?;
            source.broadcast(&tx).await?;
            presence = source.presence(txid).await?;
        }
        let missing = presence == Presence::Missing;
        active.observations.push(presence);
        store::write(ACTIVE_KEY, &*active).await?;
        if missing {
            return Err(BrowserError::SubmissionMissing);
        }
    }
    if source.hash(anchor.0).await? != anchor.1 {
        return Err(BrowserError::ReconciliationChanged);
    }
    observe(source, active).await
}
