//! Merkle Mountain Range
//!
//! references:
//! https://github.com/mimblewimble/grin/blob/master/doc/mmr.md#structure
//! https://github.com/mimblewimble/grin/blob/0ff6763ee64e5a14e70ddd4642b99789a1648a32/core/src/core/pmmr.rs#L606

use crate::borrow::Cow;
use crate::collections::VecDeque;
use crate::helper::{get_peaks, parent_offset, pos_height_in_tree, sibling_offset};
use crate::mmr_store::{MMRBatch, MMRStore};
use crate::vec;
use crate::vec::Vec;
use crate::{Error, Merge, Result};
use core::fmt::Debug;
use core::marker::PhantomData;

#[allow(clippy::upper_case_acronyms)]
pub struct MMR<T, M, S> {
    mmr_size: u64,
    batch: MMRBatch<T, S>,
    merge: PhantomData<M>,
}

impl<T: Clone + PartialEq + Debug, M: Merge<Item = T>, S: MMRStore<T>> MMR<T, M, S> {
    pub fn new(mmr_size: u64, store: S) -> Self {
        MMR {
            mmr_size,
            batch: MMRBatch::new(store),
            merge: PhantomData,
        }
    }

    // find internal MMR elem, the pos must exists, otherwise a error will return
    fn find_elem<'b>(&self, pos: u64, hashes: &'b [T]) -> Result<Cow<'b, T>> {
        let pos_offset = pos.checked_sub(self.mmr_size);
        if let Some(elem) = pos_offset.and_then(|i| hashes.get(i as usize)) {
            return Ok(Cow::Borrowed(elem));
        }
        let elem = self.batch.get_elem(pos)?.ok_or(Error::InconsistentStore)?;
        Ok(Cow::Owned(elem))
    }

    pub fn mmr_size(&self) -> u64 {
        self.mmr_size
    }

    pub fn is_empty(&self) -> bool {
        self.mmr_size == 0
    }

    // push a element and return position
    pub fn push(&mut self, elem: T) -> Result<u64> {
        let mut elems: Vec<T> = Vec::new();
        // position of new elem
        let elem_pos = self.mmr_size;
        elems.push(elem);
        let mut height = 0u32;
        let mut pos = elem_pos;
        // continue to merge tree node if next pos heigher than current
        while pos_height_in_tree(pos + 1) > height {
            pos += 1;
            let left_pos = pos - parent_offset(height);
            let right_pos = left_pos + sibling_offset(height);
            let left_elem = self.find_elem(left_pos, &elems)?;
            let right_elem = self.find_elem(right_pos, &elems)?;
            let parent_elem = M::merge(&left_elem, &right_elem)?;
            elems.push(parent_elem);
            height += 1
        }
        // store hashes
        self.batch.append(elem_pos, elems);
        // update mmr_size
        self.mmr_size = pos + 1;
        Ok(elem_pos)
    }

    /// get_root
    pub fn get_root(&self) -> Result<T> {
        if self.mmr_size == 0 {
            return Err(Error::GetRootOnEmpty);
        } else if self.mmr_size == 1 {
            return self.batch.get_elem(0)?.ok_or(Error::InconsistentStore);
        }
        let peaks: Vec<T> = get_peaks(self.mmr_size)
            .into_iter()
            .map(|peak_pos| {
                self.batch
                    .get_elem(peak_pos)
                    .and_then(|elem| elem.ok_or(Error::InconsistentStore))
            })
            .collect::<Result<Vec<T>>>()?;
        self.bag_rhs_peaks(peaks)?.ok_or(Error::InconsistentStore)
    }

    fn bag_rhs_peaks(&self, mut rhs_peaks: Vec<T>) -> Result<Option<T>> {
        while rhs_peaks.len() > 1 {
            let right_peak = rhs_peaks.pop().expect("pop");
            let left_peak = rhs_peaks.pop().expect("pop");
            rhs_peaks.push(M::merge_peaks(&right_peak, &left_peak)?);
        }
        Ok(rhs_peaks.pop())
    }

    /// generate merkle proof for a peak
    /// the pos_list must be sorted, otherwise the behaviour is undefined
    ///
    /// 1. find a lower tree in peak that can generate a complete merkle proof for position
    /// 2. find that tree by compare positions
    /// 3. generate proof for each positions
    fn gen_proof_for_peak(
        &self,
        proof: &mut Vec<T>,
        pos_list: Vec<u64>,
        peak_pos: u64,
    ) -> Result<()> {
        // do nothing if position itself is the peak
        if pos_list.len() == 1 && pos_list == [peak_pos] {
            return Ok(());
        }
        // take peak root from store if no positions need to be proof
        if pos_list.is_empty() {
            proof.push(
                self.batch
                    .get_elem(peak_pos)?
                    .ok_or(Error::InconsistentStore)?,
            );
            return Ok(());
        }

        let mut queue: VecDeque<_> = pos_list.into_iter().map(|pos| (pos, 0u32)).collect();

        // Generate sub-tree merkle proof for positions
        while let Some((pos, height)) = queue.pop_front() {
            debug_assert!(pos <= peak_pos);
            if pos == peak_pos {
                if queue.is_empty() {
                    break;
                } else {
                    return Err(Error::NodeProofsNotSupported);
                }
            }

            // calculate sibling
            let (sib_pos, parent_pos) = {
                let next_height = pos_height_in_tree(pos + 1);
                let sibling_offset = sibling_offset(height);
                if next_height > height {
                    // implies pos is right sibling
                    (pos - sibling_offset, pos + 1)
                } else {
                    // pos is left sibling
                    (pos + sibling_offset, pos + parent_offset(height))
                }
            };

            if Some(&sib_pos) == queue.front().map(|(pos, _)| pos) {
                // drop sibling
                queue.pop_front();
            } else {
                proof.push(
                    self.batch
                        .get_elem(sib_pos)?
                        .ok_or(Error::InconsistentStore)?,
                );
            }
            if parent_pos < peak_pos {
                // save pos to tree buf
                queue.push_back((parent_pos, height + 1));
            }
        }
        Ok(())
    }

    /// Generate merkle proof for positions
    /// 1. sort positions
    /// 2. push merkle proof to proof by peak from left to right
    /// 3. push bagged right hand side root
    pub fn gen_proof(&self, mut pos_list: Vec<u64>) -> Result<MerkleProof<T, M>> {
        if pos_list.is_empty() {
            return Err(Error::GenProofForInvalidLeaves);
        }
        if self.mmr_size == 1 && pos_list == [0] {
            return Ok(MerkleProof::new(self.mmr_size, Vec::new()));
        }
        if pos_list.iter().any(|pos| pos_height_in_tree(*pos) > 0) {
            return Err(Error::NodeProofsNotSupported);
        }
        // ensure positions are sorted and unique
        pos_list.sort_unstable();
        pos_list.dedup();
        let peaks = get_peaks(self.mmr_size);
        let mut proof: Vec<T> = Vec::new();
        // generate merkle proof for each peaks
        let mut bagging_track = 0;
        for peak_pos in peaks {
            let pos_list: Vec<_> = take_while_vec(&mut pos_list, |&pos| pos <= peak_pos);
            if pos_list.is_empty() {
                bagging_track += 1;
            } else {
                bagging_track = 0;
            }
            self.gen_proof_for_peak(&mut proof, pos_list, peak_pos)?;
        }

        // ensure no remain positions
        if !pos_list.is_empty() {
            return Err(Error::GenProofForInvalidLeaves);
        }

        if bagging_track > 1 {
            let rhs_peaks = proof.split_off(proof.len() - bagging_track);
            proof.push(self.bag_rhs_peaks(rhs_peaks)?.expect("bagging rhs peaks"));
        }

        Ok(MerkleProof::new(self.mmr_size, proof))
    }

    pub fn commit(self) -> Result<()> {
        self.batch.commit()
    }
}

#[derive(Debug)]
pub struct MerkleProof<T, M> {
    mmr_size: u64,
    proof: Vec<T>,
    merge: PhantomData<M>,
}

impl<T: PartialEq + Debug + Clone, M: Merge<Item = T>> MerkleProof<T, M> {
    pub fn new(mmr_size: u64, proof: Vec<T>) -> Self {
        MerkleProof {
            mmr_size,
            proof,
            merge: PhantomData,
        }
    }

    pub fn mmr_size(&self) -> u64 {
        self.mmr_size
    }

    pub fn proof_items(&self) -> &[T] {
        &self.proof
    }

    pub fn calculate_root(&self, leaves: Vec<(u64, T)>) -> Result<T> {
        calculate_root::<_, M, _>(leaves, self.mmr_size, self.proof.iter())
    }

    /// from merkle proof of leaf n to calculate merkle root of n + 1 leaves.
    /// by observe the MMR construction graph we know it is possible.
    /// https://github.com/jjyr/merkle-mountain-range#construct
    /// this is kinda tricky, but it works, and useful
    pub fn calculate_root_with_new_leaf(
        &self,
        mut leaves: Vec<(u64, T)>,
        new_pos: u64,
        new_elem: T,
        new_mmr_size: u64,
    ) -> Result<T> {
        let pos_height = pos_height_in_tree(new_pos);
        let next_height = pos_height_in_tree(new_pos + 1);
        if next_height > pos_height {
            let mut peaks_hashes =
                calculate_peaks_hashes::<_, M, _>(leaves, self.mmr_size, self.proof.iter())?;
            let peaks_pos = get_peaks(new_mmr_size);
            // reverse touched peaks
            let mut i = 0;
            while peaks_pos[i] < new_pos {
                i += 1
            }
            peaks_hashes[i..].reverse();
            calculate_root::<_, M, _>(vec![(new_pos, new_elem)], new_mmr_size, peaks_hashes.iter())
        } else {
            leaves.push((new_pos, new_elem));
            calculate_root::<_, M, _>(leaves, new_mmr_size, self.proof.iter())
        }
    }

    pub fn verify(&self, root: T, leaves: Vec<(u64, T)>) -> Result<bool> {
        self.calculate_root(leaves)
            .map(|calculated_root| calculated_root == root)
    }
}

fn calculate_peak_root<
    'a,
    T: 'a + PartialEq + Debug + Clone,
    M: Merge<Item = T>,
    I: Iterator<Item = &'a T>,
>(
    leaves: Vec<(u64, T)>,
    peak_pos: u64,
    proof_iter: &mut I,
) -> Result<T> {
    debug_assert!(!leaves.is_empty(), "can't be empty");
    // (position, hash, height)

    let mut queue: VecDeque<_> = leaves
        .into_iter()
        .map(|(pos, item)| (pos, item, 0u32))
        .collect();

    // calculate tree root from each items
    while let Some((pos, item, height)) = queue.pop_front() {
        if pos == peak_pos {
            if queue.is_empty() {
                // return root once queue is consumed
                return Ok(item);
            } else {
                return Err(Error::CorruptedProof);
            }
        }
        // calculate sibling
        let next_height = pos_height_in_tree(pos + 1);
        let (sib_pos, parent_pos) = {
            let sibling_offset = sibling_offset(height);
            if next_height > height {
                // implies pos is right sibling
                (pos - sibling_offset, pos + 1)
            } else {
                // pos is left sibling
                (pos + sibling_offset, pos + parent_offset(height))
            }
        };
        let sibling_item = if Some(&sib_pos) == queue.front().map(|(pos, _, _)| pos) {
            queue.pop_front().map(|(_, item, _)| item).unwrap()
        } else {
            proof_iter.next().ok_or(Error::CorruptedProof)?.clone()
        };

        let parent_item = if next_height > height {
            M::merge(&sibling_item, &item)
        } else {
            M::merge(&item, &sibling_item)
        }?;

        if parent_pos <= peak_pos {
            queue.push_back((parent_pos, parent_item, height + 1))
        } else {
            return Err(Error::CorruptedProof);
        }
    }
    Err(Error::CorruptedProof)
}

fn calculate_peaks_hashes<
    'a,
    T: 'a + PartialEq + Debug + Clone,
    M: Merge<Item = T>,
    I: Iterator<Item = &'a T>,
>(
    mut leaves: Vec<(u64, T)>,
    mmr_size: u64,
    mut proof_iter: I,
) -> Result<Vec<T>> {
    if leaves.iter().any(|(pos, _)| pos_height_in_tree(*pos) > 0) {
        return Err(Error::NodeProofsNotSupported);
    }

    // special handle the only 1 leaf MMR
    if mmr_size == 1 && leaves.len() == 1 && leaves[0].0 == 0 {
        return Ok(leaves.into_iter().map(|(_pos, item)| item).collect());
    }
    // ensure leaves are sorted and unique
    leaves.sort_by_key(|(pos, _)| *pos);
    leaves.dedup_by(|a, b| a.0 == b.0);
    let peaks = get_peaks(mmr_size);

    let mut peaks_hashes: Vec<T> = Vec::with_capacity(peaks.len() + 1);
    for peak_pos in peaks {
        let mut leaves: Vec<_> = take_while_vec(&mut leaves, |(pos, _)| *pos <= peak_pos);
        let peak_root = if leaves.len() == 1 && leaves[0].0 == peak_pos {
            // leaf is the peak
            leaves.remove(0).1
        } else if leaves.is_empty() {
            // if empty, means the next proof is a peak root or rhs bagged root
            if let Some(peak_root) = proof_iter.next() {
                peak_root.clone()
            } else {
                // means that either all right peaks are bagged, or proof is corrupted
                // so we break loop and check no items left
                break;
            }
        } else {
            calculate_peak_root::<_, M, _>(leaves, peak_pos, &mut proof_iter)?
        };
        peaks_hashes.push(peak_root.clone());
    }

    // ensure nothing left in leaves
    if !leaves.is_empty() {
        return Err(Error::CorruptedProof);
    }

    // check rhs peaks
    if let Some(rhs_peaks_hashes) = proof_iter.next() {
        peaks_hashes.push(rhs_peaks_hashes.clone());
    }
    // ensure nothing left in proof_iter
    if proof_iter.next().is_some() {
        return Err(Error::CorruptedProof);
    }
    Ok(peaks_hashes)
}

fn bagging_peaks_hashes<'a, T: 'a + PartialEq + Debug + Clone, M: Merge<Item = T>>(
    mut peaks_hashes: Vec<T>,
) -> Result<T> {
    // bagging peaks
    // bagging from right to left via hash(right, left).
    while peaks_hashes.len() > 1 {
        let right_peak = peaks_hashes.pop().expect("pop");
        let left_peak = peaks_hashes.pop().expect("pop");
        peaks_hashes.push(M::merge_peaks(&right_peak, &left_peak)?);
    }
    peaks_hashes.pop().ok_or(Error::CorruptedProof)
}

/// merkle proof
/// 1. sort items by position
/// 2. calculate root of each peak
/// 3. bagging peaks
fn calculate_root<
    'a,
    T: 'a + PartialEq + Debug + Clone,
    M: Merge<Item = T>,
    I: Iterator<Item = &'a T>,
>(
    leaves: Vec<(u64, T)>,
    mmr_size: u64,
    proof_iter: I,
) -> Result<T> {
    let peaks_hashes = calculate_peaks_hashes::<_, M, _>(leaves, mmr_size, proof_iter)?;
    bagging_peaks_hashes::<_, M>(peaks_hashes)
}

fn take_while_vec<T, P: Fn(&T) -> bool>(v: &mut Vec<T>, p: P) -> Vec<T> {
    for i in 0..v.len() {
        if !p(&v[i]) {
            return v.drain(..i).collect();
        }
    }
    v.drain(..).collect()
}
