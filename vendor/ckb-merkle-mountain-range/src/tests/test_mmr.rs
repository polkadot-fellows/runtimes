use super::{MergeNumberHash, NumberHash};
use crate::{
    helper::pos_height_in_tree, leaf_index_to_mmr_size, util::MemStore, Error, MMRStore, MMR,
};
use faster_hex::hex_string;
use proptest::prelude::*;
use rand::{seq::SliceRandom, thread_rng};

fn test_mmr(count: u32, proof_elem: Vec<u32>) {
    let store = MemStore::default();
    let mut mmr = MMR::<_, MergeNumberHash, _>::new(0, &store);
    let positions: Vec<u64> = (0u32..count)
        .map(|i| mmr.push(NumberHash::from(i)).unwrap())
        .collect();
    let root = mmr.get_root().expect("get root");
    let proof = mmr
        .gen_proof(
            proof_elem
                .iter()
                .map(|elem| positions[*elem as usize])
                .collect(),
        )
        .expect("gen proof");
    mmr.commit().expect("commit changes");
    let result = proof
        .verify(
            root,
            proof_elem
                .iter()
                .map(|elem| (positions[*elem as usize], NumberHash::from(*elem)))
                .collect(),
        )
        .unwrap();
    assert!(result);
}

fn test_gen_new_root_from_proof(count: u32) {
    let store = MemStore::default();
    let mut mmr = MMR::<_, MergeNumberHash, _>::new(0, &store);
    let positions: Vec<u64> = (0u32..count)
        .map(|i| mmr.push(NumberHash::from(i)).unwrap())
        .collect();
    let elem = count - 1;
    let pos = positions[elem as usize];
    let proof = mmr.gen_proof(vec![pos]).expect("gen proof");
    let new_elem = count;
    let new_pos = mmr.push(NumberHash::from(new_elem)).unwrap();
    let root = mmr.get_root().expect("get root");
    mmr.commit().expect("commit changes");
    let calculated_root = proof
        .calculate_root_with_new_leaf(
            vec![(pos, NumberHash::from(elem))],
            new_pos,
            NumberHash::from(new_elem),
            leaf_index_to_mmr_size(new_elem.into()),
        )
        .unwrap();
    assert_eq!(calculated_root, root);
}

#[test]
fn test_mmr_root() {
    let store = MemStore::default();
    let mut mmr = MMR::<_, MergeNumberHash, _>::new(0, &store);
    (0u32..11).for_each(|i| {
        mmr.push(NumberHash::from(i)).unwrap();
    });
    let root = mmr.get_root().expect("get root");
    let hex_root = hex_string(&root.0);
    assert_eq!(
        "f6794677f37a57df6a5ec36ce61036e43a36c1a009d05c81c9aa685dde1fd6e3",
        hex_root
    );
}

#[test]
fn test_empty_mmr_root() {
    let store = MemStore::<NumberHash>::default();
    let mmr = MMR::<_, MergeNumberHash, _>::new(0, &store);
    assert_eq!(Err(Error::GetRootOnEmpty), mmr.get_root());
}

#[test]
fn test_mmr_3_peaks() {
    test_mmr(11, vec![5]);
}

#[test]
fn test_mmr_2_peaks() {
    test_mmr(10, vec![5]);
}

#[test]
fn test_mmr_1_peak() {
    test_mmr(8, vec![5]);
}

#[test]
fn test_mmr_first_elem_proof() {
    test_mmr(11, vec![0]);
}

#[test]
fn test_mmr_last_elem_proof() {
    test_mmr(11, vec![10]);
}

#[test]
fn test_mmr_1_elem() {
    test_mmr(1, vec![0]);
}

#[test]
fn test_mmr_2_elems() {
    test_mmr(2, vec![0]);
    test_mmr(2, vec![1]);
}

#[test]
fn test_mmr_2_leaves_merkle_proof() {
    test_mmr(11, vec![3, 7]);
    test_mmr(11, vec![3, 4]);
}

#[test]
fn test_mmr_2_sibling_leaves_merkle_proof() {
    test_mmr(11, vec![4, 5]);
    test_mmr(11, vec![5, 6]);
    test_mmr(11, vec![6, 7]);
}

#[test]
fn test_mmr_3_leaves_merkle_proof() {
    test_mmr(11, vec![4, 5, 6]);
    test_mmr(11, vec![3, 5, 7]);
    test_mmr(11, vec![3, 4, 5]);
    test_mmr(100, vec![3, 5, 13]);
}

#[test]
fn test_gen_root_from_proof() {
    test_gen_new_root_from_proof(11);
}

#[test]
fn test_gen_proof_with_duplicate_leaves() {
    test_mmr(10, vec![5, 5]);
}

fn test_invalid_proof_verification(
    leaf_count: u32,
    positions_to_verify: Vec<u64>,
    // positions of entries that should be tampered
    tampered_positions: Vec<usize>,
    // optionally handroll proof from these positions
    handrolled_proof_positions: Option<Vec<u64>>,
) {
    use crate::{util::MemMMR, Merge, MerkleProof};
    use std::fmt::{Debug, Formatter};

    // Simple item struct to allow debugging the contents of MMR nodes/peaks
    #[derive(Clone, PartialEq)]
    enum MyItem {
        Number(u32),
        Merged(Box<MyItem>, Box<MyItem>),
    }

    impl Debug for MyItem {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                MyItem::Number(x) => f.write_fmt(format_args!("{}", x)),
                MyItem::Merged(a, b) => f.write_fmt(format_args!("Merged({:#?}, {:#?})", a, b)),
            }
        }
    }

    #[derive(Debug)]
    struct MyMerge;

    impl Merge for MyMerge {
        type Item = MyItem;
        fn merge(lhs: &Self::Item, rhs: &Self::Item) -> Result<Self::Item, crate::Error> {
            Ok(MyItem::Merged(Box::new(lhs.clone()), Box::new(rhs.clone())))
        }
    }

    let mut mmr: MemMMR<MyItem, MyMerge> = MemMMR::default();
    let mut positions: Vec<u64> = Vec::new();
    for i in 0u32..leaf_count {
        let pos = mmr.push(MyItem::Number(i)).unwrap();
        positions.push(pos);
    }
    let root = mmr.get_root().unwrap();

    let entries_to_verify: Vec<(u64, MyItem)> = positions_to_verify
        .iter()
        .map(|pos| (*pos, mmr.store().get_elem(*pos).unwrap().unwrap()))
        .collect();

    let mut tampered_entries_to_verify = entries_to_verify.clone();
    tampered_positions.iter().for_each(|proof_pos| {
        tampered_entries_to_verify[*proof_pos] = (
            tampered_entries_to_verify[*proof_pos].0,
            MyItem::Number(31337),
        )
    });

    let handrolled_proof: Option<MerkleProof<MyItem, MyMerge>> =
        handrolled_proof_positions.map(|handrolled_proof_positions| {
            MerkleProof::new(
                mmr.mmr_size(),
                handrolled_proof_positions
                    .iter()
                    .map(|pos| mmr.store().get_elem(*pos).unwrap().unwrap())
                    .collect(),
            )
        });

    // verification should fail whenever trying to prove membership of a non-member
    if let Some(handrolled_proof) = handrolled_proof {
        let handrolled_proof_result =
            handrolled_proof.verify(root.clone(), tampered_entries_to_verify.clone());
        assert!(handrolled_proof_result.is_err() || !handrolled_proof_result.unwrap());
    }

    match mmr.gen_proof(positions_to_verify.clone()) {
        Ok(proof) => {
            assert!(proof.verify(root.clone(), entries_to_verify).unwrap());
            assert!(!proof.verify(root, tampered_entries_to_verify).unwrap());
        }
        Err(Error::NodeProofsNotSupported) => {
            // if couldn't generate proof, then it contained a non-leaf
            assert!(positions_to_verify
                .iter()
                .any(|pos| pos_height_in_tree(*pos) > 0));
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[test]
fn test_generic_proofs() {
    test_invalid_proof_verification(7, vec![5], vec![0], Some(vec![2, 9, 10]));
    test_invalid_proof_verification(7, vec![1, 2], vec![0], Some(vec![5, 9, 10]));
    test_invalid_proof_verification(7, vec![1, 5], vec![0], Some(vec![0, 9, 10]));
    test_invalid_proof_verification(7, vec![1, 6], vec![0], Some(vec![0, 5, 9, 10]));
    test_invalid_proof_verification(7, vec![5, 6], vec![0], Some(vec![2, 9, 10]));
    test_invalid_proof_verification(7, vec![1, 5, 6], vec![0], Some(vec![0, 9, 10]));
    test_invalid_proof_verification(7, vec![1, 5, 7], vec![0], Some(vec![0, 8, 10]));
    test_invalid_proof_verification(7, vec![5, 6, 7], vec![0], Some(vec![2, 8, 10]));
    test_invalid_proof_verification(7, vec![5, 6, 7, 8, 9, 10], vec![0], Some(vec![2]));
    test_invalid_proof_verification(7, vec![1, 5, 7, 8, 9, 10], vec![0], Some(vec![0]));
    test_invalid_proof_verification(7, vec![0, 1, 5, 7, 8, 9, 10], vec![0], Some(vec![]));
    test_invalid_proof_verification(7, vec![0, 1, 5, 6, 7, 8, 9, 10], vec![0], Some(vec![]));
    test_invalid_proof_verification(7, vec![0, 1, 2, 5, 6, 7, 8, 9, 10], vec![0], Some(vec![]));
    test_invalid_proof_verification(7, vec![0, 1, 2, 3, 7, 8, 9, 10], vec![0], Some(vec![4]));
    test_invalid_proof_verification(7, vec![0, 2, 3, 7, 8, 9, 10], vec![0], Some(vec![1, 4]));
    test_invalid_proof_verification(7, vec![0, 3, 7, 8, 9, 10], vec![0], Some(vec![1, 4]));
    test_invalid_proof_verification(7, vec![0, 2, 3, 7, 8, 9, 10], vec![0], Some(vec![1, 4]));
}

prop_compose! {
    fn count_elem(count: u32)
                (elem in 0..count)
                -> (u32, u32) {
                    (count, elem)
    }
}

proptest! {
    #[test]
    fn test_random_mmr(count in 10u32..500u32) {
        let mut leaves: Vec<u32> = (0..count).collect();
        let mut rng = thread_rng();
        leaves.shuffle(&mut rng);
        let leaves_count = rng.gen_range(1..count - 1);
        leaves.truncate(leaves_count as usize);
        test_mmr(count, leaves);
    }

    #[test]
    fn test_random_gen_root_with_new_leaf(count in 1u32..500u32) {
        test_gen_new_root_from_proof(count);
    }
}
