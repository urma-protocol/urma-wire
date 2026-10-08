use bitcoin::{
    absolute,
    consensus::serialize,
    hashes::Hash,
    opcodes::all::{OP_CHECKSIG, OP_ENDIF, OP_IF},
    script::PushBytesBuf,
    secp256k1::{Keypair, Message, Secp256k1, SecretKey},
    sighash::{Prevouts, SighashCache, TapSighashType},
    taproot::{LeafVersion, TapLeafHash, TaprootBuilder},
    transaction::Version,
    Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use std::num::NonZeroU64;
use urma_chain::observation::Chain;
use urma_core::{
    envelope,
    format::{RecordKind, Urma},
    multipart::VerifiedRecord,
};
use urma_runtime::{plan::PublicationPlan, publication::PublicPlan};
use urma_wallet::{
    funding::Funding,
    wallet::{FeeBudget, FeeRate, SpendRequest},
};

fn transaction(previous: OutPoint, output: TxOut) -> Transaction {
    Transaction {
        version: Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: previous,
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            ..TxIn::default()
        }],
        output: vec![output],
    }
}

pub fn signed_plan() -> PublicationPlan {
    let secp = Secp256k1::new();
    let signer = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[7; 32]).unwrap());
    let author = signer.x_only_public_key().0;
    let chain = Chain::LitecoinTestnet;
    let mut record = RecordKind::Avatar.prefix().to_vec();
    record.extend_from_slice(&[0x12; 512]);
    // Construct the historical script explicitly: current publication APIs must refuse it.
    let script = ScriptBuf::builder()
        .push_x_only_key(&author)
        .push_opcode(OP_CHECKSIG)
        .push_int(0)
        .push_opcode(OP_IF)
        .push_slice(Urma::MAGIC)
        .push_slice(PushBytesBuf::try_from(record.clone()).unwrap())
        .push_opcode(OP_ENDIF)
        .into_script();
    let info = TaprootBuilder::new()
        .add_leaf(0, script.clone())
        .unwrap()
        .finalize(&secp, author)
        .unwrap();
    let funding_output = TxOut {
        value: Amount::from_sat(100_000),
        script_pubkey: urma_wallet::signing::script(&signer).unwrap(),
    };
    let funding = transaction(OutPoint::null(), funding_output.clone());
    let retained = urma_runtime::config::publication_return(chain);
    let mut reveal = transaction(
        OutPoint::null(),
        TxOut {
            value: Amount::from_sat(retained),
            script_pubkey: funding_output.script_pubkey.clone(),
        },
    );
    reveal.input[0].witness = envelope::witness(&[0; 64], &script, &info).unwrap();
    let reveal_fee = u64::try_from(reveal.vsize()).unwrap();
    let publication = TxOut {
        value: Amount::from_sat(retained + reveal_fee),
        script_pubkey: ScriptBuf::new_p2tr_tweaked(info.output_key()),
    };
    let mut commit = transaction(
        OutPoint {
            txid: funding.compute_txid(),
            vout: 0,
        },
        publication.clone(),
    );
    commit.output.push(funding_output.clone());
    commit.input[0].witness = Witness::from_slice(&[vec![0; 73], vec![0; 33]]);
    let commit_fee = u64::try_from(commit.vsize()).unwrap();
    commit.input[0].witness = Witness::new();
    commit.output[1].value = Amount::from_sat(100_000 - retained - reveal_fee - commit_fee);
    let budget = FeeBudget {
        chain: chain.genesis().unwrap(),
        rate: FeeRate(NonZeroU64::MIN),
        maximum_base_units: 10_000,
    };
    let commit = urma_wallet::signing::sign(
        &signer,
        &SpendRequest {
            transaction: &commit,
            prevouts: &[funding_output],
            budget,
        },
    )
    .unwrap();
    reveal.input[0].previous_output = OutPoint {
        txid: commit.compute_txid(),
        vout: 0,
    };
    let digest = SighashCache::new(&reveal)
        .taproot_script_spend_signature_hash(
            0,
            &Prevouts::All(&[publication]),
            TapLeafHash::from_script(&script, LeafVersion::TapScript),
            TapSighashType::Default,
        )
        .unwrap();
    let signature =
        secp.sign_schnorr_no_aux_rand(&Message::from_digest(digest.to_byte_array()), &signer);
    reveal.input[0].witness = envelope::witness(signature.as_ref(), &script, &info).unwrap();
    let proof = VerifiedRecord::verify(reveal.compute_txid(), &reveal, &commit).unwrap();
    assert_eq!(proof.record_bytes(), record);
    PublicationPlan {
        version: 1,
        chain,
        author: author.to_string(),
        root_txid: reveal.compute_txid().to_string(),
        total_fee: commit_fee + reveal_fee,
        maximum_fee: 10_000,
        records: vec![PublicPlan {
            version: 0,
            chain,
            funding: Funding {
                raw_transaction: hex::encode(serialize(&funding)),
                vout: 0,
            },
            commit: hex::encode(serialize(&commit)),
            reveal: hex::encode(serialize(&reveal)),
            fee_rate: 1,
        }],
    }
}
