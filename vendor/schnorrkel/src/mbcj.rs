// -*- mode: rust; -*-
//
// This file is part of schnorrkel.
// Copyright (c) 2019 Web 3 Foundation
// See LICENSE for licensing information.
//
// Authors:
// - jeffrey Burdges <jeff@web3.foundation>

//! Implementation for Ristretto Schnorr signatures of mBCJ
//! multi-signatures from page 
//! "On the Provable Security of Two-Round Multi-Signatures" by 
//! Manu Drijvers, Kasra Edalatnejad, Bryan Ford, and Gregory Neven
//! https://eprint.iacr.org/2018/417
//! ([slides](https://rwc.iacr.org/2019/slides/neven.pdf)).
//! These are 
//!
//https://github.com/lovesh/signature-schemes/issues/2

use core::borrow::{Borrow};  // BorrowMut
use std::collections::BTreeMap;

use merlin::Transcript;

use curve25519_dalek::constants;
use curve25519_dalek::ristretto::{CompressedRistretto,RistrettoPoint};
use curve25519_dalek::scalar::Scalar;

use super::*;
use crate::context::SigningTranscript;
use crate::errors::MultiSignatureStage;



#[allow(non_snake_case)]
pub struct {
    tau1: CompressedRistretto,
    tau2: CompressedRistretto,
    s: Scalar,
    gamma1: Scalar,
    gamma2: Scalar,
}


/// Multi-signature container generic over its session types
#[allow(non_snake_case)]
pub struct RmBCJ<T: SigningTranscript,S> {
    t: T,
    Rs: BTreeMap<PublicKey,CoR>,
    stage: S
}




