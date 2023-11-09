use crate::collections::BTreeMap;
use crate::{vec::Vec, MMRStore, Merge, MerkleProof, Result, MMR};
use core::cell::RefCell;
use core::fmt::Debug;
use core::marker::PhantomData;

#[derive(Clone)]
pub struct MemStore<T>(RefCell<BTreeMap<u64, T>>);

impl<T> Default for MemStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MemStore<T> {
    fn new() -> Self {
        MemStore(RefCell::new(Default::default()))
    }
}

impl<T: Clone> MMRStore<T> for &MemStore<T> {
    fn get_elem(&self, pos: u64) -> Result<Option<T>> {
        Ok(self.0.borrow().get(&pos).cloned())
    }

    fn append(&mut self, pos: u64, elems: Vec<T>) -> Result<()> {
        let mut store = self.0.borrow_mut();
        for (i, elem) in elems.into_iter().enumerate() {
            store.insert(pos + i as u64, elem);
        }
        Ok(())
    }
}

pub struct MemMMR<T, M> {
    store: MemStore<T>,
    mmr_size: u64,
    merge: PhantomData<M>,
}

impl<T: Clone + Debug + PartialEq, M: Merge<Item = T>> Default for MemMMR<T, M> {
    fn default() -> Self {
        Self::new(0, Default::default())
    }
}

impl<T: Clone + Debug + PartialEq, M: Merge<Item = T>> MemMMR<T, M> {
    pub fn new(mmr_size: u64, store: MemStore<T>) -> Self {
        MemMMR {
            mmr_size,
            store,
            merge: PhantomData,
        }
    }

    pub fn store(&self) -> &MemStore<T> {
        &self.store
    }

    pub fn mmr_size(&self) -> u64 {
        self.mmr_size
    }

    pub fn get_root(&self) -> Result<T> {
        let mmr = MMR::<T, M, &MemStore<T>>::new(self.mmr_size, &self.store);
        mmr.get_root()
    }

    pub fn push(&mut self, elem: T) -> Result<u64> {
        let mut mmr = MMR::<T, M, &MemStore<T>>::new(self.mmr_size, &self.store);
        let pos = mmr.push(elem)?;
        self.mmr_size = mmr.mmr_size();
        mmr.commit()?;
        Ok(pos)
    }

    pub fn gen_proof(&self, pos_list: Vec<u64>) -> Result<MerkleProof<T, M>> {
        let mmr = MMR::<T, M, &MemStore<T>>::new(self.mmr_size, &self.store);
        mmr.gen_proof(pos_list)
    }
}
