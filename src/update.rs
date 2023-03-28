use crate::error::Error;
use crate::state::{Input, Output, State, Utxo};
use miniscript::bitcoin::secp256k1::rand::rngs::OsRng;
use miniscript::bitcoin::secp256k1::{Parity, Secp256k1};
use miniscript::bitcoin::util::address::WitnessVersion;
use miniscript::bitcoin::{OutPoint, TxOut, Txid, XOnlyPublicKey};
use miniscript::{Descriptor, ToPublicKey};

pub fn generate_unknown_keys(state: &mut State, number: u32) -> Result<(), Error> {
    let secp = Secp256k1::new();

    for _ in 0..number {
        let (mut seckey, mut pubkey) = secp.generate_keypair(&mut OsRng);
        let (_, parity) = pubkey.x_only_public_key();

        if parity == Parity::Odd {
            seckey = seckey.negate();
            pubkey = seckey.public_key(&secp);
        }

        let public_key = pubkey.to_public_key();
        let keypair = seckey.keypair(&secp);
        state.passive_keys.insert(public_key, keypair);
    }

    Ok(())
}

pub fn trigger_key(state: &mut State, pubkey: XOnlyPublicKey) -> Result<(), Error> {
    let public_key = pubkey.to_public_key();

    if let Some(keypair) = state.passive_keys.remove(&public_key) {
        state.active_keys.insert(public_key, keypair);
    } else if let Some(keypair) = state.active_keys.remove(&public_key) {
        state.passive_keys.insert(public_key, keypair);
    } else {
        return Err(Error::UnknownKey);
    }

    Ok(())
}

fn verify_taproot(descriptor: &Descriptor<XOnlyPublicKey>) -> Result<(), Error> {
    if let Some(WitnessVersion::V1) = descriptor.desc_type().segwit_version() {
        Ok(())
    } else {
        Err(Error::OnlyTaproot)
    }
}

pub fn add_input(
    state: &mut State,
    index: usize,
    descriptor: Descriptor<XOnlyPublicKey>,
) -> Result<Option<Input>, Error> {
    verify_taproot(&descriptor)?;

    let input = Input {
        descriptor,
        utxo: None,
    };

    let old = state.inputs.insert(index, input);
    Ok(old)
}

pub fn add_utxo(
    state: &mut State,
    input_index: usize,
    txid: Txid,
    output_index: u32,
    value: u64,
) -> Result<Option<Utxo>, Error> {
    let input = state
        .inputs
        .get_mut(&input_index)
        .ok_or(Error::MissingInput)?;
    let utxo = Utxo {
        outpoint: OutPoint {
            txid,
            vout: output_index,
        },
        output: TxOut {
            value,
            script_pubkey: input.descriptor.script_pubkey(),
        },
    };

    let old = input.utxo.replace(utxo);
    Ok(old)
}

pub fn add_output(
    state: &mut State,
    index: usize,
    descriptor: Descriptor<XOnlyPublicKey>,
    value: u64,
) -> Result<Option<Output>, Error> {
    verify_taproot(&descriptor)?;

    let output = Output { value, descriptor };
    let old = state.outputs.insert(index, output);

    Ok(old)
}

pub fn update_fee(state: &mut State, value: u64) -> Result<(), Error> {
    state.fee = value;
    Ok(())
}

pub fn move_output(
    state: &mut State,
    output_index: usize,
    input_index: usize,
) -> Result<Option<Input>, Error> {
    let output = state
        .outputs
        .remove(&output_index)
        .ok_or(Error::MissingOutput)?;
    let new_input = Input {
        descriptor: output.descriptor,
        utxo: None,
    };

    let old = state.inputs.insert(input_index, new_input);
    Ok(old)
}
