use crate::error::Error;
use crate::state::State;
use crate::util;
use elements_miniscript::bitcoin::hashes::sha256;
use elements_miniscript::elements::hashes::hex::FromHex;
use elements_miniscript::elements::hashes::Hash;
use elements_miniscript::elements::pset::serialize::Serialize;
use elements_miniscript::elements::sighash::{Prevouts, SigHashCache};
use elements_miniscript::elements::taproot::{
    TapBranchHash, TapLeafHash, TapSighashHash, TapTweakHash,
};
use elements_miniscript::elements::{
    confidential, secp256k1_zkp, AssetId, AssetIssuance, BlockHash, LockTime, PackedLockTime,
    SchnorrSigHashType, Sequence, TxInWitness, TxOutWitness,
};
use elements_miniscript::{
    bitcoin, elements, Descriptor, MiniscriptKey, Preimage32, Satisfier, ToPublicKey,
};
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::str::FromStr;

// TODO: Remove in new version of elements-miniscript
const MAX_INPUTS: usize = 1;

pub fn get_raw_transaction(state: &mut State) -> Result<(String, f64), Error> {
    let mut spending_inputs = Vec::new();
    let mut receiving_outputs = Vec::new();
    // Eww
    let mut prevouts = [0; MAX_INPUTS].map(|_| elements::TxOut::default());
    let mut input_funds: u64 = 0;
    let mut output_funds = 0;

    // Add unsigned inputs
    for (expected_index, input_index) in state.inputs.keys().sorted().enumerate() {
        if expected_index != *input_index {
            return Err(Error::MissingInput);
        }

        let input = &state.inputs[input_index];
        let utxo = &input.utxo;
        let txin = elements::TxIn {
            previous_output: utxo.outpoint,
            is_pegin: false,
            script_sig: elements::Script::new(),
            sequence: input.sequence,
            asset_issuance: AssetIssuance::default(),
            witness: TxInWitness::default(),
        };
        spending_inputs.push(txin);
        prevouts[*input_index] = utxo.output.clone();

        if let confidential::Value::Explicit(value) = utxo.output.value {
            input_funds += value;
        } else {
            unreachable!("State should only contain explicit values")
        }
    }

    let bitcoin_asset_id = AssetId::from_hex(util::BITCOIN_ASSET_ID).unwrap();

    // Add outputs
    for (expected_index, output_index) in state.outputs.keys().sorted().enumerate() {
        if expected_index != *output_index {
            return Err(Error::MissingOutput);
        }

        let output = &state.outputs[output_index];
        let txout = elements::TxOut {
            asset: confidential::Asset::Explicit(bitcoin_asset_id),
            value: confidential::Value::Explicit(output.value),
            nonce: confidential::Nonce::Null,
            script_pubkey: output.descriptor.script_pubkey(),
            witness: TxOutWitness::default(),
        };
        receiving_outputs.push(txout);
        output_funds += output.value;
    }

    // Add fee output
    receiving_outputs.push(elements::TxOut::new_fee(state.fee, bitcoin_asset_id));
    output_funds += state.fee;

    let mut remaining_index_value = None;

    // Assign remaining input funds to the remaining output (if it exists)
    for output_index in state.outputs.keys().sorted() {
        let output = &state.outputs[output_index];
        if output.value == 0 {
            if remaining_index_value.is_some() {
                return Err(Error::OneZeroOutput);
            }

            let remaining_funds = input_funds
                .checked_sub(output_funds)
                .ok_or(Error::NotEnoughFunds)?;
            receiving_outputs[*output_index].value = confidential::Value::Explicit(remaining_funds);
            remaining_index_value = Some((*output_index, remaining_funds));
        }
    }

    if let Some((output_index, value)) = remaining_index_value {
        state
            .outputs
            .entry(output_index)
            .and_modify(|e| e.value = value);
    }

    // Construct unsigned transaction
    let mut spending_tx = elements::Transaction {
        version: 2,
        lock_time: PackedLockTime(state.locktime.to_consensus_u32()),
        input: spending_inputs,
        output: receiving_outputs,
    };

    let secp = secp256k1_zkp::Secp256k1::new();
    let cache = Rc::new(RefCell::new(SigHashCache::new(&spending_tx)));
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
            sighash_type: SchnorrSigHashType::All,
            cache: cache.clone(),
            secp: &secp,
        };
        let (script_witness, _script_sig) = input.utxo.descriptor.get_satisfaction(satisfier)?;
        witnesses.push(TxInWitness {
            amount_rangeproof: None,
            inflation_keys_rangeproof: None,
            script_witness,
            pegin_witness: vec![],
        });
    }

    // Add witness to inputs
    // Cannot be done in previous loop due to borrowing issue
    for (input_index, witness) in witnesses.into_iter().enumerate() {
        spending_tx.input[input_index].witness = witness;
    }

    // Compute feerate (includes witness)
    // TODO: Replace with vsize in new version of elements-miniscript
    let feerate = state.fee as f64 / spending_tx.weight() as f64 / 4.0;

    // Serialize transaction as hex
    let tx_hex = spending_tx
        .serialize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Ok((tx_hex, feerate))
}

#[derive()]
struct DynamicSigner<'a, T: Deref<Target = elements::Transaction>> {
    active_keys: &'a HashMap<bitcoin::PublicKey, bitcoin::KeyPair>,
    active_images: &'a HashMap<sha256::Hash, Preimage32>,
    internal_key: bitcoin::PublicKey,
    merkle_root: Option<TapBranchHash>,
    input_index: usize,
    prevouts: Prevouts<'a>,
    locktime: LockTime,
    sequence: Sequence,
    sighash_type: SchnorrSigHashType,
    cache: Rc<RefCell<SigHashCache<T>>>,
    secp: &'a secp256k1_zkp::Secp256k1<secp256k1_zkp::All>,
}

impl<'a, T> DynamicSigner<'a, T>
where
    T: Deref<Target = elements::Transaction>,
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
    ) -> elements::SchnorrSig {
        // TODO: Replace once TapSigHashHash implementsThirtyTwoByteHash
        let msg = secp256k1_zkp::Message::from_slice(sighash.as_ref()).unwrap();
        let sig = self.secp.sign_schnorr(&msg, keypair);

        elements::SchnorrSig {
            sig,
            hash_ty: self.sighash_type,
        }
    }
}

impl<'a, Pk, T> Satisfier<Pk> for DynamicSigner<'a, T>
where
    Pk: MiniscriptKey<Sha256 = sha256::Hash> + ToPublicKey,
    T: Deref<Target = elements::Transaction>,
{
    fn lookup_tap_key_spend_sig(&self) -> Option<elements::SchnorrSig> {
        let internal_pair = self.get_keypair(self.internal_key)?;

        // TODO: Replace in new elements-miniscript version
        let (internal_xpub, _) = internal_pair.x_only_public_key();
        let tweak = TapTweakHash::from_key_and_tweak(internal_xpub, self.merkle_root);
        let tweak = secp256k1_zkp::Scalar::from_be_bytes(tweak.into_inner())
            .expect("hash value greater than curve order");
        let output_pair = &internal_pair.add_xonly_tweak(self.secp, &tweak).unwrap();

        let sighash = match self.cache.borrow_mut().taproot_key_spend_signature_hash(
            self.input_index,
            &self.prevouts,
            self.sighash_type,
            BlockHash::from_str(util::ELEMENTS_REGTEST_GENESIS_BLOCK_HASH).unwrap(),
        ) {
            Ok(hash) => hash,
            Err(error) => {
                println!("{}", error);
                return None;
            }
        };
        let signature = self.get_signature(sighash, output_pair);

        Some(signature)
    }

    fn lookup_tap_leaf_script_sig(
        &self,
        pk: &Pk,
        leaf_hash: &TapLeafHash,
    ) -> Option<elements::SchnorrSig> {
        let pk = pk.to_public_key();
        let keypair = self.get_keypair(pk)?;
        let sighash = match self.cache.borrow_mut().taproot_script_spend_signature_hash(
            self.input_index,
            &self.prevouts,
            *leaf_hash,
            self.sighash_type,
            BlockHash::from_str(util::ELEMENTS_REGTEST_GENESIS_BLOCK_HASH).unwrap(),
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
