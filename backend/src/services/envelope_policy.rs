//! Envelope policy validation performed *before* a transaction is signed.
//!
//! The signer only ever sees an opaque 32-byte hash. Before that hash is
//! produced we decode the transaction the RPC simulation returned and check
//! that it is exactly the invocation we intended to submit:
//!
//! - exactly one operation,
//! - an `InvokeHostFunction` / `InvokeContract` call,
//! - targeting the configured `SNAPSHOT_CONTRACT_ID`,
//! - calling `submit_snapshot`,
//! - with the expected arguments (`hash` bytes, then `epoch` u64 — the same
//!   order `ContractService::build_invoke_args` sends).
//!
//! Anything else — a different contract, function, argument list, or extra
//! operations — is rejected and the signature request never reaches the KMS.
//! This is the guard that keeps a compromised or buggy simulation response
//! from turning the submitter key into a signing oracle.

use anyhow::{anyhow, bail, Result};
use stellar_xdr::curr::{
    ContractId, Hash, HostFunction, OperationBody, ScAddress, ScVal, Transaction,
};

/// Function name the snapshot contract exposes for anchoring.
pub const SUBMIT_SNAPSHOT_FUNCTION: &str = "submit_snapshot";

/// Validate that `tx` is exactly the `submit_snapshot(hash, epoch)` call on
/// `contract_id` that this service intended to build.
///
/// Returns `Err` with a human-readable reason when any check fails.
pub fn validate_submit_snapshot_envelope(
    tx: &Transaction,
    contract_id: &str,
    hash: &[u8; 32],
    epoch: u64,
) -> Result<()> {
    // Exactly one operation: the contract invocation. Anything else means
    // the simulation returned a transaction we did not ask for.
    let mut ops = tx.operations.iter();
    let op = ops
        .next()
        .ok_or_else(|| anyhow!("Transaction has no operations"))?;
    if ops.next().is_some() {
        bail!("Transaction has more than one operation; refusing to sign");
    }

    let OperationBody::InvokeHostFunction(invoke) = &op.body else {
        bail!("Transaction operation is not InvokeHostFunction; refusing to sign");
    };

    let HostFunction::InvokeContract(args) = &invoke.host_function else {
        bail!("Host function is not InvokeContract; refusing to sign");
    };

    // The contract address must be the configured snapshot contract.
    let contract = stellar_strkey::Contract::from_string(contract_id)
        .map_err(|e| anyhow!("Invalid SNAPSHOT_CONTRACT_ID '{contract_id}': {e}"))?;
    let expected_address = ScAddress::Contract(ContractId(Hash(contract.0)));
    if args.contract_address != expected_address {
        bail!("Transaction targets an unexpected contract; refusing to sign");
    }

    // The function must be submit_snapshot.
    if args.function_name.0.to_utf8_string_lossy() != SUBMIT_SNAPSHOT_FUNCTION {
        bail!(
            "Transaction invokes unexpected function '{}'; refusing to sign",
            args.function_name.0.to_utf8_string_lossy()
        );
    }

    // Exactly two args: snapshot hash (bytes) then epoch (u64).
    let mut vals = args.args.iter();
    let (Some(hash_arg), Some(epoch_arg), None) = (vals.next(), vals.next(), vals.next())
    else {
        bail!(
            "submit_snapshot expects exactly 2 arguments, got {}",
            args.args.len()
        );
    };

    match hash_arg {
        ScVal::Bytes(bytes) => {
            let got = bytes.0.as_vec();
            if got.as_slice() != &hash[..] {
                bail!("submit_snapshot hash argument does not match; refusing to sign");
            }
        }
        _ => bail!("submit_snapshot first argument is not bytes; refusing to sign"),
    }

    match epoch_arg {
        ScVal::U64(v) if *v == epoch => {}
        ScVal::U64(v) => {
            bail!("submit_snapshot epoch argument is {v}, expected {epoch}; refusing to sign")
        }
        _ => bail!("submit_snapshot second argument is not u64; refusing to sign"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{
        InvokeContractArgs, InvokeHostFunctionOp, Memo, MuxedAccount, Operation,
        Preconditions, ScBytes, ScSymbol, SequenceNumber, TransactionExt, Uint256, VecM,
    };
    use ed25519_dalek::SigningKey;

    // Real testnet contract ids from contracts/.env.testnet.
    const TEST_CONTRACT: &str = "CAPHQZ4BBT43HU5EUSJAOPKWB66HGLTN4AKJUALV3R2RXS4A6IOXWUTL";
    const OTHER_CONTRACT: &str = "CAZO4LD7NSWZFUJCB5ORHS3IBFJC76KHSOCPTHVDBDBISZJ72ACSHPH5";
    const TEST_FUNCTION: &str = SUBMIT_SNAPSHOT_FUNCTION;

    fn contract_strkey() -> &'static str {
        TEST_CONTRACT
    }

    fn make_tx(
        contract_id: &str,
        function: &str,
        args: Vec<ScVal>,
    ) -> Transaction {
        let contract = stellar_strkey::Contract::from_string(contract_id)
            .unwrap_or_else(|e| panic!("bad test contract id {contract_id}: {e}"));
        let op = Operation {
            source_account: None,
            body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
                host_function: HostFunction::InvokeContract(InvokeContractArgs {
                    contract_address: ScAddress::Contract(ContractId(Hash(contract.0))),
                    function_name: ScSymbol::try_from(function.as_bytes().to_vec()).unwrap(),
                    args: args.try_into().unwrap(),
                }),
                auth: VecM::default(),
            }),
        };
        let signing = SigningKey::from_bytes(&[9u8; 32]);
        Transaction {
            source_account: MuxedAccount::Ed25519(Uint256(signing.verifying_key().to_bytes())),
            fee: 100,
            seq_num: SequenceNumber(1),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: vec![op].try_into().unwrap(),
            ext: TransactionExt::V0,
        }
    }

    fn valid_args(hash: &[u8; 32], epoch: u64) -> Vec<ScVal> {
        vec![
            ScVal::Bytes(ScBytes::try_from(hash.to_vec()).unwrap()),
            ScVal::U64(epoch),
        ]
    }

    #[test]
    fn accepts_the_expected_invocation() {
        let hash = [5u8; 32];
        let tx = make_tx(contract_strkey(), TEST_FUNCTION, valid_args(&hash, 7));
        validate_submit_snapshot_envelope(&tx, contract_strkey(), &hash, 7).unwrap();
    }

    #[test]
    fn rejects_wrong_function() {
        let hash = [5u8; 32];
        let tx = make_tx(contract_strkey(), "withdraw", valid_args(&hash, 7));
        assert!(validate_submit_snapshot_envelope(&tx, contract_strkey(), &hash, 7).is_err());
    }

    #[test]
    fn rejects_wrong_hash_argument() {
        let tx = make_tx(contract_strkey(), TEST_FUNCTION, valid_args(&[6u8; 32], 7));
        assert!(validate_submit_snapshot_envelope(&tx, contract_strkey(), &[5u8; 32], 7).is_err());
    }

    #[test]
    fn rejects_wrong_epoch() {
        let hash = [5u8; 32];
        let tx = make_tx(contract_strkey(), TEST_FUNCTION, valid_args(&hash, 8));
        assert!(validate_submit_snapshot_envelope(&tx, contract_strkey(), &hash, 7).is_err());
    }

    #[test]
    fn rejects_wrong_contract() {
        let hash = [5u8; 32];
        assert_ne!(OTHER_CONTRACT, TEST_CONTRACT);

        // The tx targets TEST_CONTRACT but validation is configured for a
        // different contract: must be rejected.
        let tx = make_tx(contract_strkey(), TEST_FUNCTION, valid_args(&hash, 7));
        assert!(validate_submit_snapshot_envelope(&tx, OTHER_CONTRACT, &hash, 7).is_err());
    }

    #[test]
    fn rejects_empty_operations() {
        let signing = SigningKey::from_bytes(&[9u8; 32]);
        let tx = Transaction {
            source_account: MuxedAccount::Ed25519(Uint256(signing.verifying_key().to_bytes())),
            fee: 100,
            seq_num: SequenceNumber(1),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: VecM::default(),
            ext: TransactionExt::V0,
        };
        assert!(validate_submit_snapshot_envelope(&tx, contract_strkey(), &[5u8; 32], 7).is_err());
    }

    #[test]
    fn rejects_extra_operations() {
        let hash = [5u8; 32];
        let mut tx = make_tx(contract_strkey(), TEST_FUNCTION, valid_args(&hash, 7));
        // Duplicate the single op to simulate an unexpected second operation.
        let first = tx.operations.iter().next().unwrap().clone();
        let mut ops = tx.operations.to_vec();
        ops.push(first);
        tx.operations = ops.try_into().unwrap();
        assert!(validate_submit_snapshot_envelope(&tx, contract_strkey(), &hash, 7).is_err());
    }

    #[test]
    fn rejects_missing_args() {
        let hash = [5u8; 32];
        let tx = make_tx(contract_strkey(), TEST_FUNCTION, vec![ScVal::U64(7)]);
        assert!(validate_submit_snapshot_envelope(&tx, contract_strkey(), &hash, 7).is_err());
    }

    #[test]
    fn invalid_configured_contract_id_errors() {
        let hash = [5u8; 32];
        let tx = make_tx(contract_strkey(), TEST_FUNCTION, valid_args(&hash, 7));
        assert!(validate_submit_snapshot_envelope(&tx, "not-a-contract", &hash, 7).is_err());
    }
}
