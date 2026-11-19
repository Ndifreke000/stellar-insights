//! Canonical path-payment failure taxonomy (#2766).
//!
//! Before this module there were two independent tallies
//! (`PathPaymentFailureCode`/`CorridorFailureBreakdown` in `ingestion/mod.rs`
//! and `PathPaymentFailure`/`CorridorFailureCounts` in `ingestion/ledger.rs`),
//! each with its own result-code mapping and both matching substrings of the
//! base64 `result_xdr`. That substring matching never actually worked on real
//! XDR (result codes are encoded as `int32` discriminants, not ASCII), and a
//! code added to one mapping was silently dropped by the other.
//!
//! This module is now the only place that knows about path-payment result
//! codes. It decodes `TransactionResult` XDR with `stellar-xdr` and maps every
//! failure arm of both `PathPaymentStrictReceiveResult` and
//! `PathPaymentStrictSendResult` (including fee-bump inner results) to a
//! single [`PathPaymentFailureCode`].

use std::collections::BTreeMap;

use serde::Serialize;
use stellar_xdr::curr::{
    InnerTransactionResultResult, Limits, OperationResult, OperationResultTr,
    PathPaymentStrictReceiveResult, PathPaymentStrictSendResult, ReadXdr, TransactionResult,
    TransactionResultResult,
};

/// Which path-payment operation produced a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathPaymentKind {
    StrictReceive,
    StrictSend,
}

/// Every failure code a path payment can return, for both strict-send and
/// strict-receive. `OverSendmax` only occurs on strict-receive and
/// `UnderDestmin` only on strict-send; all others are shared.
///
/// The string form (`as_str`) matches the Horizon `result_codes.operations`
/// spelling and is what gets persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathPaymentFailureCode {
    Malformed,
    Underfunded,
    SrcNoTrust,
    SrcNotAuthorized,
    NoDestination,
    NoTrust,
    NotAuthorized,
    LineFull,
    NoIssuer,
    TooFewOffers,
    OfferCrossSelf,
    /// strict-receive only: path would cost more than `send_max`.
    OverSendmax,
    /// strict-send only: path would deliver less than `dest_min`.
    UnderDestmin,
}

impl PathPaymentFailureCode {
    pub const ALL: [Self; 13] = [
        Self::Malformed,
        Self::Underfunded,
        Self::SrcNoTrust,
        Self::SrcNotAuthorized,
        Self::NoDestination,
        Self::NoTrust,
        Self::NotAuthorized,
        Self::LineFull,
        Self::NoIssuer,
        Self::TooFewOffers,
        Self::OfferCrossSelf,
        Self::OverSendmax,
        Self::UnderDestmin,
    ];

    /// Horizon spelling of the result code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Malformed => "op_malformed",
            Self::Underfunded => "op_underfunded",
            Self::SrcNoTrust => "op_src_no_trust",
            Self::SrcNotAuthorized => "op_src_not_authorized",
            Self::NoDestination => "op_no_destination",
            Self::NoTrust => "op_no_trust",
            Self::NotAuthorized => "op_not_authorized",
            Self::LineFull => "op_line_full",
            Self::NoIssuer => "op_no_issuer",
            Self::TooFewOffers => "op_too_few_offers",
            Self::OfferCrossSelf => "op_cross_self",
            Self::OverSendmax => "op_over_source_max",
            Self::UnderDestmin => "op_under_dest_min",
        }
    }

    /// Parse a Horizon `result_codes.operations` string. Used only for
    /// Horizon JSON responses; XDR goes through [`decode_result_xdr`].
    #[must_use]
    pub fn from_horizon_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == code)
    }

    /// Liquidity failures are the ones that reflect corridor depth (as
    /// opposed to user/account misconfiguration).
    #[must_use]
    pub const fn is_liquidity_failure(self) -> bool {
        matches!(
            self,
            Self::TooFewOffers | Self::OverSendmax | Self::UnderDestmin | Self::OfferCrossSelf
        )
    }

    #[must_use]
    pub fn from_strict_receive(r: &PathPaymentStrictReceiveResult) -> Option<Self> {
        use PathPaymentStrictReceiveResult as R;
        Some(match r {
            R::Success(_) => return None,
            R::Malformed => Self::Malformed,
            R::Underfunded => Self::Underfunded,
            R::SrcNoTrust => Self::SrcNoTrust,
            R::SrcNotAuthorized => Self::SrcNotAuthorized,
            R::NoDestination => Self::NoDestination,
            R::NoTrust => Self::NoTrust,
            R::NotAuthorized => Self::NotAuthorized,
            R::LineFull => Self::LineFull,
            R::NoIssuer(_) => Self::NoIssuer,
            R::TooFewOffers => Self::TooFewOffers,
            R::OfferCrossSelf => Self::OfferCrossSelf,
            R::OverSendmax => Self::OverSendmax,
        })
    }

    #[must_use]
    pub fn from_strict_send(r: &PathPaymentStrictSendResult) -> Option<Self> {
        use PathPaymentStrictSendResult as R;
        Some(match r {
            R::Success(_) => return None,
            R::Malformed => Self::Malformed,
            R::Underfunded => Self::Underfunded,
            R::SrcNoTrust => Self::SrcNoTrust,
            R::SrcNotAuthorized => Self::SrcNotAuthorized,
            R::NoDestination => Self::NoDestination,
            R::NoTrust => Self::NoTrust,
            R::NotAuthorized => Self::NotAuthorized,
            R::LineFull => Self::LineFull,
            R::NoIssuer(_) => Self::NoIssuer,
            R::TooFewOffers => Self::TooFewOffers,
            R::OfferCrossSelf => Self::OfferCrossSelf,
            R::UnderDestmin => Self::UnderDestmin,
        })
    }
}

/// Outcome of one path-payment operation inside a transaction result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathPaymentOutcome {
    pub kind: PathPaymentKind,
    /// `None` on success.
    pub failure: Option<PathPaymentFailureCode>,
}

#[derive(Debug, thiserror::Error)]
pub enum FailureDecodeError {
    #[error("invalid TransactionResult XDR: {0}")]
    Xdr(#[from] stellar_xdr::curr::Error),
}

fn op_results(result: &TransactionResult) -> &[OperationResult] {
    match &result.result {
        TransactionResultResult::TxSuccess(ops) | TransactionResultResult::TxFailed(ops) => ops,
        TransactionResultResult::TxFeeBumpInnerSuccess(pair)
        | TransactionResultResult::TxFeeBumpInnerFailed(pair) => match &pair.result.result {
            InnerTransactionResultResult::TxSuccess(ops)
            | InnerTransactionResultResult::TxFailed(ops) => ops,
            _ => &[],
        },
        _ => &[],
    }
}

/// Extract the outcome of every path-payment operation in a decoded result.
#[must_use]
pub fn path_payment_outcomes(result: &TransactionResult) -> Vec<PathPaymentOutcome> {
    op_results(result)
        .iter()
        .filter_map(|op| match op {
            OperationResult::OpInner(OperationResultTr::PathPaymentStrictReceive(r)) => {
                Some(PathPaymentOutcome {
                    kind: PathPaymentKind::StrictReceive,
                    failure: PathPaymentFailureCode::from_strict_receive(r),
                })
            }
            OperationResult::OpInner(OperationResultTr::PathPaymentStrictSend(r)) => {
                Some(PathPaymentOutcome {
                    kind: PathPaymentKind::StrictSend,
                    failure: PathPaymentFailureCode::from_strict_send(r),
                })
            }
            _ => None,
        })
        .collect()
}

/// Decode a base64 `TransactionResult` and return its path-payment outcomes.
pub fn decode_result_xdr(result_xdr: &str) -> Result<Vec<PathPaymentOutcome>, FailureDecodeError> {
    let result = TransactionResult::from_xdr_base64(result_xdr, Limits::none())?;
    Ok(path_payment_outcomes(&result))
}

/// Per-corridor breakdown of path-payment outcomes. This is the single
/// tally type used by every ingestion path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CorridorFailureBreakdown {
    pub source_asset: String,
    pub destination_asset: String,
    pub successful: u64,
    /// Failure counts keyed by the canonical code.
    pub failures: BTreeMap<PathPaymentFailureCode, u64>,
}

impl CorridorFailureBreakdown {
    #[must_use]
    pub fn new(source_asset: impl Into<String>, destination_asset: impl Into<String>) -> Self {
        Self {
            source_asset: source_asset.into(),
            destination_asset: destination_asset.into(),
            ..Default::default()
        }
    }

    pub fn record_failure(&mut self, code: PathPaymentFailureCode) {
        *self.failures.entry(code).or_insert(0) += 1;
    }

    pub fn record(&mut self, outcome: PathPaymentOutcome) {
        match outcome.failure {
            Some(code) => self.record_failure(code),
            None => self.successful += 1,
        }
    }

    #[must_use]
    pub fn count(&self, code: PathPaymentFailureCode) -> u64 {
        self.failures.get(&code).copied().unwrap_or(0)
    }

    #[must_use]
    pub fn total_failures(&self) -> u64 {
        self.failures.values().sum()
    }

    /// Success rate as a percentage in `[0, 100]`; `0.0` with no activity.
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        corridor_success_rate(self.successful, self)
    }
}

/// Success rate for a corridor that accounts for every failed path payment.
#[must_use]
pub fn corridor_success_rate(successful: u64, failures: &CorridorFailureBreakdown) -> f64 {
    let total = successful + failures.total_failures();
    if total == 0 {
        return 0.0;
    }
    (successful as f64 / total as f64) * 100.0
}

/// Decode `result_xdr` and fold every path-payment outcome into `counts`.
/// Returns the number of failures recorded.
pub fn tally_path_payment_failures(
    result_xdr: &str,
    counts: &mut CorridorFailureBreakdown,
) -> Result<u64, FailureDecodeError> {
    let mut found = 0;
    for outcome in decode_result_xdr(result_xdr)? {
        if outcome.failure.is_some() {
            found += 1;
        }
        counts.record(outcome);
    }
    Ok(found)
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! Result XDR fixtures, built with the same `stellar-xdr` types core
    //! emits so the base64 is byte-identical to what Horizon/RPC return.
    use super::*;
    use stellar_xdr::curr::{
        AccountId, AlphaNum4, Asset, AssetCode4, Hash, InnerTransactionResult,
        InnerTransactionResultExt, InnerTransactionResultPair,
        PathPaymentStrictReceiveResultSuccess, PathPaymentStrictSendResultSuccess, PublicKey,
        SimplePaymentResult, TransactionResultExt, Uint256, VecM, WriteXdr,
    };

    fn usdc() -> Asset {
        Asset::CreditAlphanum4(AlphaNum4 {
            asset_code: AssetCode4(*b"USDC"),
            issuer: AccountId(PublicKey::PublicKeyTypeEd25519(Uint256([7; 32]))),
        })
    }

    pub fn strict_receive(code: Option<PathPaymentFailureCode>) -> OperationResult {
        use PathPaymentFailureCode as C;
        use PathPaymentStrictReceiveResult as R;
        let r = match code {
            None => R::Success(PathPaymentStrictReceiveResultSuccess {
                offers: VecM::default(),
                last: SimplePaymentResult {
                    destination: AccountId(PublicKey::PublicKeyTypeEd25519(Uint256([1; 32]))),
                    asset: usdc(),
                    amount: 100,
                },
            }),
            Some(C::Malformed) => R::Malformed,
            Some(C::Underfunded) => R::Underfunded,
            Some(C::SrcNoTrust) => R::SrcNoTrust,
            Some(C::SrcNotAuthorized) => R::SrcNotAuthorized,
            Some(C::NoDestination) => R::NoDestination,
            Some(C::NoTrust) => R::NoTrust,
            Some(C::NotAuthorized) => R::NotAuthorized,
            Some(C::LineFull) => R::LineFull,
            Some(C::NoIssuer) => R::NoIssuer(usdc()),
            Some(C::TooFewOffers) => R::TooFewOffers,
            Some(C::OfferCrossSelf) => R::OfferCrossSelf,
            Some(C::OverSendmax) => R::OverSendmax,
            Some(C::UnderDestmin) => panic!("strict-receive has no under_dest_min"),
        };
        OperationResult::OpInner(OperationResultTr::PathPaymentStrictReceive(r))
    }

    pub fn strict_send(code: Option<PathPaymentFailureCode>) -> OperationResult {
        use PathPaymentFailureCode as C;
        use PathPaymentStrictSendResult as R;
        let r = match code {
            None => R::Success(PathPaymentStrictSendResultSuccess {
                offers: VecM::default(),
                last: SimplePaymentResult {
                    destination: AccountId(PublicKey::PublicKeyTypeEd25519(Uint256([1; 32]))),
                    asset: usdc(),
                    amount: 100,
                },
            }),
            Some(C::Malformed) => R::Malformed,
            Some(C::Underfunded) => R::Underfunded,
            Some(C::SrcNoTrust) => R::SrcNoTrust,
            Some(C::SrcNotAuthorized) => R::SrcNotAuthorized,
            Some(C::NoDestination) => R::NoDestination,
            Some(C::NoTrust) => R::NoTrust,
            Some(C::NotAuthorized) => R::NotAuthorized,
            Some(C::LineFull) => R::LineFull,
            Some(C::NoIssuer) => R::NoIssuer(usdc()),
            Some(C::TooFewOffers) => R::TooFewOffers,
            Some(C::OfferCrossSelf) => R::OfferCrossSelf,
            Some(C::UnderDestmin) => R::UnderDestmin,
            Some(C::OverSendmax) => panic!("strict-send has no over_source_max"),
        };
        OperationResult::OpInner(OperationResultTr::PathPaymentStrictSend(r))
    }

    pub fn tx_result_xdr(ops: Vec<OperationResult>, failed: bool) -> String {
        let ops: VecM<OperationResult> = ops.try_into().expect("ops fit");
        let result = TransactionResult {
            fee_charged: 100,
            result: if failed {
                TransactionResultResult::TxFailed(ops)
            } else {
                TransactionResultResult::TxSuccess(ops)
            },
            ext: TransactionResultExt::V0,
        };
        result.to_xdr_base64(Limits::none()).expect("encode")
    }

    pub fn fee_bump_result_xdr(ops: Vec<OperationResult>) -> String {
        let ops: VecM<OperationResult> = ops.try_into().expect("ops fit");
        let result = TransactionResult {
            fee_charged: 200,
            result: TransactionResultResult::TxFeeBumpInnerFailed(InnerTransactionResultPair {
                transaction_hash: Hash([9; 32]),
                result: InnerTransactionResult {
                    fee_charged: 100,
                    result: InnerTransactionResultResult::TxFailed(ops),
                    ext: InnerTransactionResultExt::V0,
                },
            }),
            ext: TransactionResultExt::V0,
        };
        result.to_xdr_base64(Limits::none()).expect("encode")
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;
    use PathPaymentFailureCode as C;

    const RECEIVE_CODES: [C; 12] = [
        C::Malformed,
        C::Underfunded,
        C::SrcNoTrust,
        C::SrcNotAuthorized,
        C::NoDestination,
        C::NoTrust,
        C::NotAuthorized,
        C::LineFull,
        C::NoIssuer,
        C::TooFewOffers,
        C::OfferCrossSelf,
        C::OverSendmax,
    ];
    const SEND_CODES: [C; 12] = [
        C::Malformed,
        C::Underfunded,
        C::SrcNoTrust,
        C::SrcNotAuthorized,
        C::NoDestination,
        C::NoTrust,
        C::NotAuthorized,
        C::LineFull,
        C::NoIssuer,
        C::TooFewOffers,
        C::OfferCrossSelf,
        C::UnderDestmin,
    ];

    #[test]
    fn golden_strict_receive_every_failure_code() {
        for code in RECEIVE_CODES {
            let xdr = tx_result_xdr(vec![strict_receive(Some(code))], true);
            let outcomes = decode_result_xdr(&xdr).unwrap();
            assert_eq!(
                outcomes,
                vec![PathPaymentOutcome {
                    kind: PathPaymentKind::StrictReceive,
                    failure: Some(code)
                }],
                "{code:?}"
            );
        }
    }

    #[test]
    fn golden_strict_send_every_failure_code() {
        for code in SEND_CODES {
            let xdr = tx_result_xdr(vec![strict_send(Some(code))], true);
            let outcomes = decode_result_xdr(&xdr).unwrap();
            assert_eq!(
                outcomes,
                vec![PathPaymentOutcome {
                    kind: PathPaymentKind::StrictSend,
                    failure: Some(code)
                }],
                "{code:?}"
            );
        }
    }

    #[test]
    fn regression_under_dest_min_on_strict_send_is_counted() {
        // The old substring matcher never matched real XDR, so this
        // failure was silently dropped on both paths.
        let xdr = tx_result_xdr(vec![strict_send(Some(C::UnderDestmin))], true);
        let mut counts = CorridorFailureBreakdown::new("USDC", "NGN");
        assert_eq!(tally_path_payment_failures(&xdr, &mut counts).unwrap(), 1);
        assert_eq!(counts.count(C::UnderDestmin), 1);
    }

    #[test]
    fn success_and_fee_bump_inner_results_are_decoded() {
        let ok = tx_result_xdr(vec![strict_receive(None), strict_send(None)], false);
        let mut counts = CorridorFailureBreakdown::default();
        assert_eq!(tally_path_payment_failures(&ok, &mut counts).unwrap(), 0);
        assert_eq!(counts.successful, 2);

        let fb = fee_bump_result_xdr(vec![strict_receive(Some(C::TooFewOffers))]);
        assert_eq!(tally_path_payment_failures(&fb, &mut counts).unwrap(), 1);
        assert_eq!(counts.count(C::TooFewOffers), 1);
    }

    #[test]
    fn invalid_xdr_is_an_error_not_zero() {
        let mut counts = CorridorFailureBreakdown::default();
        assert!(tally_path_payment_failures("not-xdr", &mut counts).is_err());
        assert!(tally_path_payment_failures("AAAA...op_too_few_offers...", &mut counts).is_err());
    }

    #[test]
    fn horizon_codes_round_trip() {
        for code in C::ALL {
            assert_eq!(C::from_horizon_code(code.as_str()), Some(code));
        }
        assert_eq!(C::from_horizon_code("op_success"), None);
    }

    #[test]
    fn success_rate_includes_all_failures() {
        let mut b = CorridorFailureBreakdown::default();
        b.record_failure(C::TooFewOffers);
        b.record_failure(C::TooFewOffers);
        b.record_failure(C::NoTrust);
        assert_eq!(b.total_failures(), 3);
        assert!((corridor_success_rate(6, &b) - 66.666_666).abs() < 1e-3);
        assert_eq!(
            corridor_success_rate(0, &CorridorFailureBreakdown::default()),
            0.0
        );
    }
}
