//! Ledger scan coverage (#2765).
//!
//! Coverage is tracked separately from the events themselves: a ledger that
//! was scanned and produced no contract event is still *covered*. The
//! indexer records every successfully scanned batch as an inclusive
//! `[start, end]` range in `ledger_scan_ranges`, merging adjacent and
//! overlapping ranges, and gap detection is a range-difference over that
//! table. Both operations are O(number of ranges), never O(ledgers).

/// A contiguous inclusive ledger range.
pub type LedgerRange = (u64, u64);

/// Sort and merge overlapping or adjacent ranges. Ranges with `start > end`
/// are dropped.
#[must_use]
pub fn merge_ranges(mut ranges: Vec<LedgerRange>) -> Vec<LedgerRange> {
    ranges.retain(|(s, e)| s <= e);
    ranges.sort_unstable();
    let mut merged: Vec<LedgerRange> = Vec::with_capacity(ranges.len());
    for (s, e) in ranges {
        match merged.last_mut() {
            Some((_, last_end)) if s <= last_end.saturating_add(1) => {
                *last_end = (*last_end).max(e);
            }
            _ => merged.push((s, e)),
        }
    }
    merged
}

/// Return the sub-ranges of `[from, to]` not covered by `covered`.
///
/// `covered` may be unsorted and overlapping; it is merged first. The work
/// done is proportional to the number of ranges, not the width of
/// `[from, to]`.
#[must_use]
pub fn compute_gaps(covered: Vec<LedgerRange>, from: u64, to: u64) -> Vec<LedgerRange> {
    if from > to {
        return Vec::new();
    }
    let mut gaps = Vec::new();
    let mut next = from; // first ledger not yet known to be covered
    for (s, e) in merge_ranges(covered) {
        if e < next {
            continue;
        }
        if s > to {
            break;
        }
        if s > next {
            gaps.push((next, s - 1));
        }
        if e >= to {
            return gaps;
        }
        next = e + 1;
    }
    gaps.push((next, to));
    gaps
}

/// Total number of ledgers inside `gaps`.
#[must_use]
pub fn gap_ledgers_total(gaps: &[LedgerRange]) -> u64 {
    gaps.iter().map(|(s, e)| e - s + 1).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    fn brute_force_gaps(covered: &[LedgerRange], from: u64, to: u64) -> Vec<LedgerRange> {
        let set: BTreeSet<u64> = covered
            .iter()
            .flat_map(|&(s, e)| if s <= e { s..=e } else { 1..=0 })
            .collect();
        let mut gaps = Vec::new();
        let mut start: Option<u64> = None;
        for l in from..=to {
            if set.contains(&l) {
                if let Some(s) = start.take() {
                    gaps.push((s, l - 1));
                }
            } else if start.is_none() {
                start = Some(l);
            }
        }
        if let Some(s) = start {
            gaps.push((s, to));
        }
        gaps
    }

    #[test]
    fn merges_adjacent_and_overlapping() {
        assert_eq!(
            merge_ranges(vec![(10, 20), (1, 5), (6, 9), (15, 30), (40, 41)]),
            vec![(1, 30), (40, 41)]
        );
    }

    #[test]
    fn fully_covered_range_has_no_gaps() {
        assert!(compute_gaps(vec![(1, 1000)], 1, 1000).is_empty());
        assert!(compute_gaps(vec![(1, 500), (501, 1000)], 100, 900).is_empty());
    }

    #[test]
    fn empty_coverage_is_one_gap() {
        assert_eq!(compute_gaps(vec![], 5, 10), vec![(5, 10)]);
    }

    #[test]
    fn edges_and_holes() {
        assert_eq!(
            compute_gaps(vec![(3, 4), (8, 9)], 1, 10),
            vec![(1, 2), (5, 7), (10, 10)]
        );
        assert_eq!(compute_gaps(vec![(0, u64::MAX)], 0, u64::MAX), vec![]);
        assert!(compute_gaps(vec![], 10, 5).is_empty());
    }

    #[test]
    fn ten_million_ledger_range_is_fast() {
        // 10M ledgers covered by 100k ranges with a 1-ledger hole between
        // each: work is proportional to ranges, not ledgers.
        let ranges: Vec<LedgerRange> = (0..100_000u64).map(|i| (i * 100, i * 100 + 98)).collect();
        let start = std::time::Instant::now();
        let gaps = compute_gaps(ranges, 0, 10_000_000 - 1);
        let elapsed = start.elapsed();
        assert_eq!(gaps.len(), 100_000);
        assert_eq!(gap_ledgers_total(&gaps), 100_000);
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "took {elapsed:?}"
        );
    }

    proptest! {
        #[test]
        fn gaps_equal_brute_force_complement(
            raw in proptest::collection::vec((0u64..500, 0u64..40), 0..30),
            from in 0u64..300,
            width in 0u64..300,
        ) {
            let covered: Vec<LedgerRange> = raw.into_iter().map(|(s, w)| (s, s + w)).collect();
            let to = from + width;
            prop_assert_eq!(
                compute_gaps(covered.clone(), from, to),
                brute_force_gaps(&covered, from, to)
            );
        }
    }
}
