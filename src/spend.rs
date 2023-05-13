use crate::error::Error;
use crate::state::State;
use crate::util;
use itertools::Itertools;
use miniscript::bitcoin::hashes::sha256;
use miniscript::bitcoin::psbt::serialize::Serialize;
use miniscript::bitcoin::psbt::Prevouts;
use miniscript::bitcoin::schnorr::TapTweak;
use miniscript::bitcoin::secp256k1::{All, Message, Secp256k1};
use miniscript::bitcoin::util::sighash::SighashCache;
use miniscript::bitcoin::util::taproot::{TapBranchHash, TapLeafHash, TapSighashHash};
use miniscript::bitcoin::{LockTime, PackedLockTime, SchnorrSighashType, Sequence, Witness};
use miniscript::{bitcoin, Descriptor, MiniscriptKey, Preimage32, Satisfier, ToPublicKey};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

pub fn get_raw_transaction(state: &mut State) -> Result<(String, f64), Error> {
    let mut spending_inputs = Vec::new();
    let mut receiving_outputs = Vec::new();
    let mut prevouts = Vec::new();

    // Add unsigned inputs
    for (expected_index, input_index) in state.inputs.keys().sorted().enumerate() {
        if expected_index != *input_index {
            return Err(Error::MissingInput);
        }

        let input = &state.inputs[input_index];
        let utxo = &input.utxo;
        let txin = bitcoin::TxIn {
            previous_output: utxo.outpoint,
            script_sig: bitcoin::Script::new(),
            sequence: input.sequence,
            witness: Witness::default(),
        };
        spending_inputs.push(txin);
        prevouts.push(&utxo.output);
    }

    // Add outputs
    for (expected_index, output_index) in state.outputs.keys().sorted().enumerate() {
        if expected_index != *output_index {
            return Err(Error::MissingOutput);
        }

        let output = &state.outputs[output_index];
        let txout = bitcoin::TxOut {
            value: output.value,
            script_pubkey: output.descriptor.script_pubkey(),
        };
        receiving_outputs.push(txout);
    }

    // Assign remaining input funds to the remaining output (if it exists)
    if let Some((output_index, value)) = util::get_remaining_funds(state)? {
        receiving_outputs[output_index].value = value;
    }

    // Construct unsigned transaction
    let mut spending_tx = bitcoin::Transaction {
        version: 2,
        lock_time: PackedLockTime(state.locktime.to_consensus_u32()),
        input: spending_inputs,
        output: receiving_outputs,
    };

    let secp = Secp256k1::new();
    let cache = Rc::new(RefCell::new(SighashCache::new(&spending_tx)));
    let mut witnesses = Vec::new();

    // Sign inputs
    for input_index in state.inputs.keys().sorted() {
        let input = &state.inputs[input_index];
        // Extract internal key and merkle root for key spends
        let (internal_key, merkle_root) = match &input.utxo.descriptor {
            Descriptor::Tr(tr) => {
                let info = tr.spend_info();
                let internal_key = info.internal_key().to_public_key();
                let merkle_root = info.merkle_root();
                (internal_key, merkle_root)
            }
            _ => return Err(Error::OnlyTaproot),
        };

        let satisfier = DynamicSigner {
            active_keys: &state.active_keys,
            active_images: &state.active_images,
            internal_key,
            merkle_root,
            input_index: *input_index,
            prevouts: Prevouts::All(&prevouts),
            locktime: state.locktime,
            sequence: state.inputs[input_index].sequence,
            sighash_type: SchnorrSighashType::All,
            cache: cache.clone(),
            secp: &secp,
        };
        let (witness, _script_sig) = input.utxo.descriptor.get_satisfaction(satisfier)?;
        witnesses.push(Witness::from_vec(witness));
    }

    // Add witness to inputs
    // Cannot be done in previous loop due to borrowing issue
    for (input_index, witness) in witnesses.into_iter().enumerate() {
        spending_tx.input[input_index].witness = witness;
    }

    // Compute feerate (includes witness)
    let feerate = state.fee as f64 / spending_tx.vsize() as f64;

    // Serialize transaction as hex
    let tx_hex = spending_tx
        .serialize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Ok((tx_hex, feerate))
}

struct DynamicSigner<'a, T: Deref<Target = bitcoin::Transaction>, O: Borrow<bitcoin::TxOut>> {
    active_keys: &'a HashMap<bitcoin::PublicKey, bitcoin::KeyPair>,
    active_images: &'a HashMap<sha256::Hash, Preimage32>,
    internal_key: bitcoin::PublicKey,
    merkle_root: Option<TapBranchHash>,
    input_index: usize,
    prevouts: Prevouts<'a, O>,
    locktime: LockTime,
    sequence: Sequence,
    sighash_type: SchnorrSighashType,
    cache: Rc<RefCell<SighashCache<T>>>,
    secp: &'a Secp256k1<All>,
}

impl<'a, T, O> DynamicSigner<'a, T, O>
where
    T: Deref<Target = bitcoin::Transaction>,
    O: Borrow<bitcoin::TxOut>,
{
    fn get_keypair(&self, pk: bitcoin::PublicKey) -> Option<&bitcoin::KeyPair> {
        match self.active_keys.get(&pk) {
            Some(keypair) => Some(keypair),
            None => {
                println!("Unknown key: {}", util::into_xonly(pk));
                None
            }
        }
    }

    fn get_signature(
        &self,
        sighash: TapSighashHash,
        keypair: &bitcoin::KeyPair,
    ) -> bitcoin::SchnorrSig {
        let msg = Message::from(sighash);
        let sig = self.secp.sign_schnorr(&msg, keypair);

        bitcoin::SchnorrSig {
            sig,
            hash_ty: self.sighash_type,
        }
    }
}

impl<'a, Pk, T, O> Satisfier<Pk> for DynamicSigner<'a, T, O>
where
    Pk: MiniscriptKey<Sha256 = sha256::Hash> + ToPublicKey,
    T: Deref<Target = bitcoin::Transaction>,
    O: Borrow<bitcoin::TxOut>,
{
    fn lookup_tap_key_spend_sig(&self) -> Option<bitcoin::SchnorrSig> {
        let internal_pair = self.get_keypair(self.internal_key)?;
        let output_pair = internal_pair
            .tap_tweak(self.secp, self.merkle_root)
            .to_inner();
        let sighash = match self.cache.borrow_mut().taproot_key_spend_signature_hash(
            self.input_index,
            &self.prevouts,
            self.sighash_type,
        ) {
            Ok(hash) => hash,
            Err(error) => {
                println!("{}", error);
                return None;
            }
        };
        let signature = self.get_signature(sighash, &output_pair);

        Some(signature)
    }

    fn lookup_tap_leaf_script_sig(
        &self,
        pk: &Pk,
        leaf_hash: &TapLeafHash,
    ) -> Option<bitcoin::SchnorrSig> {
        let pk = pk.to_public_key();
        let keypair = self.get_keypair(pk)?;
        let sighash = match self.cache.borrow_mut().taproot_script_spend_signature_hash(
            self.input_index,
            &self.prevouts,
            *leaf_hash,
            self.sighash_type,
        ) {
            Ok(hash) => hash,
            Err(error) => {
                println!("{}", error);
                return None;
            }
        };
        let signature = self.get_signature(sighash, keypair);

        Some(signature)
    }

    fn lookup_sha256(&self, image: &Pk::Sha256) -> Option<Preimage32> {
        self.active_images.get(image.as_ref()).copied()
    }

    fn check_older(&self, sequence: Sequence) -> bool {
        <Sequence as Satisfier<Pk>>::check_older(&self.sequence, sequence)
    }

    fn check_after(&self, locktime: LockTime) -> bool {
        <LockTime as Satisfier<Pk>>::check_after(&self.locktime, locktime)
    }
}
