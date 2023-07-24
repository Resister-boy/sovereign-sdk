#![deny(missing_docs)]

//! Blob storage module allows to save DA blobs in the state

use sov_modules_api::Module;
use sov_modules_macros::ModuleInfo;
use sov_rollup_interface::da::BlobTransactionTrait;
use sov_state::{StateMap, WorkingSet};

/// Blob storage contains only address and vector of blobs
#[derive(ModuleInfo, Clone)]
pub struct BlobStorage<C: sov_modules_api::Context> {
    /// The address of blob storage module
    /// Note: this is address is generated by the module framework and the corresponding private key is unknown.
    #[address]
    pub(crate) address: C::Address,

    /// Actual storage of blobs
    /// DA block number => vector of blobs
    /// Caller controls the order of blobs in the vector
    #[state]
    pub(crate) blobs: StateMap<u64, Vec<Vec<u8>>>,
}

/// Non standard methods for blob storage
impl<C: sov_modules_api::Context> BlobStorage<C> {
    /// Store blobs for given block number, overwrite if already exists
    pub fn store_blobs<B: BlobTransactionTrait>(
        &self,
        block_number: u64,
        blobs: &[B],
        working_set: &mut WorkingSet<C::Storage>,
    ) -> anyhow::Result<()> {
        let mut raw_blobs: Vec<Vec<u8>> = Vec::with_capacity(blobs.len());
        for blob in blobs {
            raw_blobs.push(bincode::serialize(blob)?);
        }
        self.blobs.set(&block_number, &raw_blobs, working_set);
        Ok(())
    }

    /// Take all blobs for given block number, return empty vector if not exists
    /// Returned blobs are removed from the storage
    pub fn take_blobs_for_block_number<B: BlobTransactionTrait>(
        &self,
        block_number: u64,
        working_set: &mut WorkingSet<C::Storage>,
    ) -> Vec<B> {
        self.blobs
            .remove(&block_number, working_set)
            .unwrap_or_default()
            .iter()
            .map(|b| bincode::deserialize(b).expect("malformed blob was stored previously"))
            .collect()
    }
}

/// Empty module implementation
impl<C: sov_modules_api::Context> Module for BlobStorage<C> {
    type Context = C;
    type Config = ();
}
