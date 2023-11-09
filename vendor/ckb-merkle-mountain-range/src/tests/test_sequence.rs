use std::fmt;

use proptest::proptest;
use rand::{prelude::*, thread_rng};

use crate::{util::MemStore, Merge, Result, MMR};

#[derive(Eq, PartialEq, Clone, Default)]
struct NumberRange {
    start: u32,
    end: u32,
}

struct MergeNumberRange;

impl fmt::Debug for NumberRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NumberRange({}, {})", self.start, self.end)
    }
}

impl fmt::Debug for MergeNumberRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MergeNumberRange")
    }
}

impl From<u32> for NumberRange {
    fn from(num: u32) -> Self {
        Self {
            start: num,
            end: num,
        }
    }
}

impl NumberRange {
    fn is_normalized(&self) -> bool {
        self.start <= self.end
    }
}

impl Merge for MergeNumberRange {
    type Item = NumberRange;
    fn merge(lhs: &Self::Item, rhs: &Self::Item) -> Result<Self::Item> {
        Ok(Self::Item {
            start: lhs.start,
            end: rhs.end,
        })
    }
    fn merge_peaks(lhs: &Self::Item, rhs: &Self::Item) -> Result<Self::Item> {
        Self::merge(rhs, lhs)
    }
}

fn test_sequence_sub_func(count: u32, proof_elem: Vec<u32>) {
    let store = MemStore::default();
    let mut mmr = MMR::<_, MergeNumberRange, _>::new(0, &store);
    let positions = (0..count)
        .map(|i| mmr.push(NumberRange::from(i)).expect("push"))
        .collect::<Vec<_>>();
    let root = mmr.get_root().expect("get_root");
    assert!(root.is_normalized());
    let proof = mmr
        .gen_proof(
            proof_elem
                .iter()
                .map(|elem| positions[*elem as usize])
                .collect(),
        )
        .expect("gen_proof");
    for item in proof.proof_items() {
        assert!(item.is_normalized())
    }
    mmr.commit().expect("commit");
    let result = proof
        .verify(
            root,
            proof_elem
                .iter()
                .map(|elem| (positions[*elem as usize], NumberRange::from(*elem)))
                .collect(),
        )
        .expect("verify");
    assert!(result);
}

proptest! {
    #[test]
    fn test_sequence(count in 10u32..500u32) {
        let mut leaves: Vec<u32> = (0..count).collect();
        let mut rng = thread_rng();
        leaves.shuffle(&mut rng);
        let leaves_count = rng.gen_range(1..count - 1);
        leaves.truncate(leaves_count as usize);
        test_sequence_sub_func(count, leaves);
    }
}
