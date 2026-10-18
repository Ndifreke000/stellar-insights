//! Multi-signature transaction coordination: hash a transaction, verify each
//! co-signer's signature against that hash, attach the signatures and submit
//! the result to Horizon.
//!
//! The backend never signs. Signers sign in their own wallets and send either
//! a raw base64 signature or the envelope their wallet returned.

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature as Ed25519Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use stellar_xdr::curr::{
    DecoratedSignature, Limits, MuxedAccount, ReadXdr, Signature, SignatureHint,
    TransactionEnvelope, TransactionSignaturePayload, TransactionSignaturePayloadTaggedTransaction,
    WriteXdr,
};

fn decode_envelope(xdr_b64: &str) -> Result<TransactionEnvelope> {
    let bytes = BASE64
        .decode(xdr_b64.trim())
        .context("transaction XDR is not valid base64")?;
    TransactionEnvelope::from_xdr(&bytes, Limits::len(1_000_000))
        .map_err(|e| anyhow!("transaction XDR could not be parsed: {e}"))
}

/// The hash signers sign: SHA-256 of the XDR `TransactionSignaturePayload`
/// for this network.
pub fn transaction_hash(xdr_b64: &str, network_passphrase: &str) -> Result<[u8; 32]> {
    let envelope = decode_envelope(xdr_b64)?;
    let tagged = match envelope {
        TransactionEnvelope::Tx(v1) => TransactionSignaturePayloadTaggedTransaction::Tx(v1.tx),
        TransactionEnvelope::TxFeeBump(fb) => {
            TransactionSignaturePayloadTaggedTransaction::TxFeeBump(fb.tx)
        }
        TransactionEnvelope::TxV0(_) => {
            return Err(anyhow!("legacy v0 transaction envelopes are not supported"))
        }
    };
    let payload = TransactionSignaturePayload {
        network_id: stellar_xdr::curr::Hash(Sha256::digest(network_passphrase.as_bytes()).into()),
        tagged_transaction: tagged,
    };
    let bytes = payload
        .to_xdr(Limits::none())
        .context("encode signature payload")?;
    Ok(Sha256::digest(bytes).into())
}

/// The source account (`G...`) of a v1 transaction.
pub fn source_account(xdr_b64: &str) -> Result<String> {
    match decode_envelope(xdr_b64)? {
        TransactionEnvelope::Tx(v1) => {
            let key = match v1.tx.source_account {
                MuxedAccount::Ed25519(key) => key.0,
                MuxedAccount::MuxedEd25519(muxed) => muxed.ed25519.0,
            };
            Ok(stellar_strkey::ed25519::PublicKey(key)
                .to_string()
                .to_string())
        }
        _ => Err(anyhow!("only v1 transaction envelopes can be coordinated")),
    }
}

fn verifying_key(account: &str) -> Result<VerifyingKey> {
    let key = stellar_strkey::ed25519::PublicKey::from_string(account)
        .map_err(|_| anyhow!("{account} is not a valid Stellar account"))?;
    VerifyingKey::from_bytes(&key.0).map_err(|_| anyhow!("{account} is not a valid Ed25519 key"))
}

fn hint(key: &VerifyingKey) -> [u8; 4] {
    let bytes = key.to_bytes();
    [bytes[28], bytes[29], bytes[30], bytes[31]]
}

/// Accept `signature` as either a base64 Ed25519 signature or a signed
/// transaction envelope, and return the 64-byte signature by `signer` over
/// `hash`. Anything that does not verify is rejected.
pub fn extract_signature(signature: &str, signer: &str, hash: &[u8; 32]) -> Result<[u8; 64]> {
    let key = verifying_key(signer)?;

    if let Ok(raw) = BASE64.decode(signature.trim()) {
        if let Ok(bytes) = <[u8; 64]>::try_from(raw.as_slice()) {
            key.verify(hash, &Ed25519Signature::from_bytes(&bytes))
                .map_err(|_| anyhow!("signature does not verify for {signer}"))?;
            return Ok(bytes);
        }
    }

    let envelope = decode_envelope(signature)
        .context("signature must be a base64 Ed25519 signature or a signed transaction XDR")?;
    let signatures = match &envelope {
        TransactionEnvelope::Tx(v1) => v1.signatures.to_vec(),
        TransactionEnvelope::TxFeeBump(fb) => fb.signatures.to_vec(),
        TransactionEnvelope::TxV0(v0) => v0.signatures.to_vec(),
    };
    let expected_hint = hint(&key);
    signatures
        .iter()
        .filter(|decorated| decorated.hint.0 == expected_hint)
        .find_map(|decorated| {
            let bytes = <[u8; 64]>::try_from(decorated.signature.0.as_slice()).ok()?;
            key.verify(hash, &Ed25519Signature::from_bytes(&bytes))
                .ok()
                .map(|()| bytes)
        })
        .ok_or_else(|| anyhow!("the envelope carries no valid signature by {signer}"))
}

/// Attach verified signatures to the transaction and return the envelope XDR.
pub fn assemble(xdr_b64: &str, signatures: &[(String, [u8; 64])]) -> Result<String> {
    let mut envelope = decode_envelope(xdr_b64)?;
    let mut decorated = Vec::with_capacity(signatures.len());
    for (signer, bytes) in signatures {
        let key = verifying_key(signer)?;
        decorated.push(DecoratedSignature {
            hint: SignatureHint(hint(&key)),
            signature: Signature(
                bytes
                    .to_vec()
                    .try_into()
                    .map_err(|_| anyhow!("unexpected signature length"))?,
            ),
        });
    }
    match &mut envelope {
        TransactionEnvelope::Tx(v1) => {
            v1.signatures = decorated
                .try_into()
                .map_err(|_| anyhow!("a transaction can carry at most 20 signatures"))?;
        }
        _ => return Err(anyhow!("only v1 transaction envelopes can be coordinated")),
    }
    let bytes = envelope.to_xdr(Limits::none()).context("encode envelope")?;
    Ok(BASE64.encode(bytes))
}

/// Outcome of submitting to Horizon.
#[derive(Debug)]
pub struct Submission {
    pub hash: String,
    pub successful: bool,
    /// Horizon's result codes when the transaction was rejected.
    pub error: Option<String>,
}

/// Submit an envelope to Horizon's `/transactions` endpoint.
pub async fn submit(
    http: &reqwest::Client,
    horizon_url: &str,
    envelope_xdr: &str,
) -> Result<Submission> {
    let body = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("tx", envelope_xdr)
        .finish();
    let response = http
        .post(format!(
            "{}/transactions",
            horizon_url.trim_end_matches('/')
        ))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .context("could not reach Horizon")?;
    let status = response.status();
    let json: serde_json::Value = response.json().await.context("Horizon response")?;

    if status.is_success() {
        return Ok(Submission {
            hash: json["hash"].as_str().unwrap_or_default().to_string(),
            successful: json["successful"].as_bool().unwrap_or(true),
            error: None,
        });
    }
    Ok(Submission {
        hash: json["extras"]["hash"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        successful: false,
        error: Some(
            json["extras"]["result_codes"]
                .as_object()
                .map(|codes| serde_json::Value::Object(codes.clone()).to_string())
                .unwrap_or_else(|| json["title"].as_str().unwrap_or("rejected").to_string()),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use stellar_xdr::curr::{
        Memo, Preconditions, SequenceNumber, Transaction, TransactionExt, TransactionV1Envelope,
        Uint256, VecM,
    };

    const PASSPHRASE: &str = "Test SDF Network ; September 2015";

    fn key(seed: u8) -> (SigningKey, String) {
        let signing = SigningKey::from_bytes(&[seed; 32]);
        let account = stellar_strkey::ed25519::PublicKey(signing.verifying_key().to_bytes())
            .to_string()
            .to_string();
        (signing, account)
    }

    fn unsigned_tx(source: &SigningKey) -> String {
        let tx = Transaction {
            source_account: MuxedAccount::Ed25519(Uint256(source.verifying_key().to_bytes())),
            fee: 100,
            seq_num: SequenceNumber(1),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: VecM::default(),
            ext: TransactionExt::V0,
        };
        let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
            tx,
            signatures: VecM::default(),
        });
        BASE64.encode(envelope.to_xdr(Limits::none()).unwrap())
    }

    #[test]
    fn reads_the_source_account() {
        let (source, account) = key(1);
        assert_eq!(source_account(&unsigned_tx(&source)).unwrap(), account);
    }

    #[test]
    fn accepts_a_raw_signature_over_the_hash() {
        let (source, account) = key(1);
        let xdr = unsigned_tx(&source);
        let hash = transaction_hash(&xdr, PASSPHRASE).unwrap();
        let sig = BASE64.encode(source.sign(&hash).to_bytes());

        assert!(extract_signature(&sig, &account, &hash).is_ok());
    }

    #[test]
    fn accepts_a_signed_envelope_from_a_wallet() {
        let (source, account) = key(1);
        let xdr = unsigned_tx(&source);
        let hash = transaction_hash(&xdr, PASSPHRASE).unwrap();
        let signed = assemble(&xdr, &[(account.clone(), source.sign(&hash).to_bytes())]).unwrap();

        assert!(extract_signature(&signed, &account, &hash).is_ok());
    }

    #[test]
    fn rejects_a_signature_by_someone_else() {
        let (source, account) = key(1);
        let (other, _) = key(2);
        let xdr = unsigned_tx(&source);
        let hash = transaction_hash(&xdr, PASSPHRASE).unwrap();
        let forged = BASE64.encode(other.sign(&hash).to_bytes());

        assert!(extract_signature(&forged, &account, &hash).is_err());
    }

    #[test]
    fn rejects_a_signature_for_another_network() {
        let (source, account) = key(1);
        let xdr = unsigned_tx(&source);
        let mainnet =
            transaction_hash(&xdr, "Public Global Stellar Network ; September 2015").unwrap();
        let testnet = transaction_hash(&xdr, PASSPHRASE).unwrap();
        let sig = BASE64.encode(source.sign(&mainnet).to_bytes());

        assert!(extract_signature(&sig, &account, &testnet).is_err());
    }

    #[test]
    fn assembles_every_signature_into_the_envelope() {
        let (a, account_a) = key(1);
        let (b, account_b) = key(2);
        let xdr = unsigned_tx(&a);
        let hash = transaction_hash(&xdr, PASSPHRASE).unwrap();
        let signed = assemble(
            &xdr,
            &[
                (account_a.clone(), a.sign(&hash).to_bytes()),
                (account_b.clone(), b.sign(&hash).to_bytes()),
            ],
        )
        .unwrap();

        assert!(extract_signature(&signed, &account_a, &hash).is_ok());
        assert!(extract_signature(&signed, &account_b, &hash).is_ok());
        // The hash is over the transaction, not its signatures.
        assert_eq!(transaction_hash(&signed, PASSPHRASE).unwrap(), hash);
    }

    #[test]
    fn rejects_garbage_input() {
        let (_, account) = key(1);
        assert!(extract_signature("not base64 at all!", &account, &[0; 32]).is_err());
        assert!(transaction_hash("AAAA", PASSPHRASE).is_err());
    }
}
