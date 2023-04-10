use crate::error::Error;
use crate::state::State;
use itertools::Itertools;
use miniscript::bitcoin::psbt::serialize::Serialize;
use miniscript::bitcoin::psbt::Prevouts;
use miniscript::bitcoin::schnorr::TapTweak;
use miniscript::bitcoin::secp256k1::{All, Message, Secp256k1};
use miniscript::bitcoin::util::sighash::SighashCache;
use miniscript::bitcoin::util::taproot::{TapBranchHash, TapLeafHash, TapSighashHash};
use miniscript::bitcoin::{
    KeyPair, PackedLockTime, PublicKey, SchnorrSig, SchnorrSighashType, Script, Sequence,
    Transaction, TxIn, TxOut, Witness,
};
use miniscript::{Descriptor, MiniscriptKey, Satisfier, ToPublicKey};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

pub fn get_raw_transaction(state: &State) -> Result<(String, f64), Error> {
    let mut spending_inputs = Vec::new();
    let mut receiving_outputs = Vec::new();
    let mut prevouts = Vec::new();
    let mut input_funds = 0;
    let mut output_funds = 0;

    // Add unsigned inputs
    for (expected_index, input_index) in state.inputs.keys().sorted().enumerate() {
        if expected_index != *input_index {
            return Err(Error::MissingInput);
        }

        let input = &state.inputs[input_index];
        let utxo = input.utxo.as_ref().ok_or(Error::MissingUtxo)?;
        let txin = TxIn {
            previous_output: utxo.outpoint,
            script_sig: Script::new(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        };
        spending_inputs.push(txin);
        prevouts.push(&utxo.output);
        input_funds += utxo.output.value;
    }

    // Add outputs
    for (expected_index, output_index) in state.outputs.keys().sorted().enumerate() {
        if expected_index != *output_index {
            return Err(Error::MissingOutput);
        }

        let output = &state.outputs[output_index];
        let txout = TxOut {
            value: output.value,
            script_pubkey: output.descriptor.script_pubkey(),
        };
        receiving_outputs.push(txout);
        output_funds += output.value;
    }

    output_funds += state.fee;

    // Assign remaining input funds to the remaining output (if it exists)
    for output_index in state.outputs.keys().sorted() {
        let output = &state.outputs[output_index];
        if output.value == 0 {
            let remaining_funds = input_funds
                .checked_sub(output_funds)
                .ok_or(Error::NotEnoughFunds)?;
            receiving_outputs[*output_index].value = remaining_funds;
            break;
        }
    }

    // Construct unsigned transaction
    let mut spending_tx = Transaction {
        version: 2,
        lock_time: PackedLockTime(0),
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
        let (internal_key, merkle_root) = match &input.descriptor {
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
            internal_key,
            merkle_root,
            input_index: *input_index,
            prevouts: Prevouts::All(&prevouts),
            sighash_type: SchnorrSighashType::All,
            cache: cache.clone(),
            secp: &secp,
        };
        let (witness, _script_sig) = input.descriptor.get_satisfaction(satisfier)?;
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

struct DynamicSigner<'a, T: Deref<Target = Transaction>, O: Borrow<TxOut>> {
    active_keys: &'a HashMap<PublicKey, KeyPair>,
    internal_key: PublicKey,
    merkle_root: Option<TapBranchHash>,
    input_index: usize,
    prevouts: Prevouts<'a, O>,
    sighash_type: SchnorrSighashType,
    cache: Rc<RefCell<SighashCache<T>>>,
    secp: &'a Secp256k1<All>,
}

impl<'a, T, O> DynamicSigner<'a, T, O>
where
    T: Deref<Target = Transaction>,
    O: Borrow<TxOut>,
{
    fn get_keypair(&self, pk: PublicKey) -> Option<&KeyPair> {
        match self.active_keys.get(&pk) {
            Some(keypair) => Some(keypair),
            None => {
                let (xonly, _) = pk.inner.x_only_public_key();
                println!("Unknown key: {}", xonly);
                None
            }
        }
    }

    fn get_signature(&self, sighash: TapSighashHash, keypair: &KeyPair) -> SchnorrSig {
        let msg = Message::from(sighash);
        let sig = self.secp.sign_schnorr(&msg, keypair);

        SchnorrSig {
            sig,
            hash_ty: self.sighash_type,
        }
    }
}

impl<'a, Pk, T, O> Satisfier<Pk> for DynamicSigner<'a, T, O>
where
    Pk: MiniscriptKey + ToPublicKey,
    T: Deref<Target = Transaction>,
    O: Borrow<TxOut>,
{
    fn lookup_tap_key_spend_sig(&self) -> Option<SchnorrSig> {
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

    fn lookup_tap_leaf_script_sig(&self, pk: &Pk, leaf_hash: &TapLeafHash) -> Option<SchnorrSig> {
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
}
