use crate::error::Error;
use crate::state::{Input, State, Utxo};
use crate::util;
use elements_miniscript::elements;
use elements_miniscript::elements::hashes::hex::FromHex;
use elements_miniscript::elements::locktime::Height;
use elements_miniscript::elements::{confidential, AssetId, LockTime, Sequence, TxOutWitness};
use itertools::Itertools;

pub fn update_locktime(state: &mut State, height: u32) -> Result<(), Error> {
    let height = Height::from_consensus(height).map_err(|_| Error::InvalidHeight)?;
    state.locktime = LockTime::Blocks(height);
    Ok(())
}

pub fn update_fee(state: &mut State, value: u64) -> Result<(), Error> {
    state.fee = value;
    Ok(())
}

pub fn finalize_transaction(state: &mut State, txid: elements::Txid) -> Result<(), Error> {
    for (_, input) in state.inputs.drain() {
        if let Some(index) = state.utxos.iter().position(|x| x == &input.utxo) {
            let _utxo = state.utxos.remove(index);
        }
    }

    let mut is_first_input = true;

    for (output_index, output) in state.outputs.drain().sorted_by(|(a, _), (b, _)| a.cmp(b)) {
        let utxo = Utxo {
            output: elements::TxOut {
                asset: confidential::Asset::Explicit(
                    AssetId::from_hex(util::BITCOIN_ASSET_ID).unwrap(),
                ),
                value: confidential::Value::Explicit(output.value),
                nonce: confidential::Nonce::Null,
                script_pubkey: output.descriptor.script_pubkey(),
                witness: TxOutWitness::default(),
            },
            descriptor: output.descriptor,
            outpoint: elements::OutPoint {
                txid,
                vout: output_index as u32,
            },
        };

        if is_first_input {
            let first_input = Input {
                utxo: utxo.clone(),
                sequence: Sequence::MAX,
            };
            println!("New txin: {}", first_input);
            state.inputs.insert(0, first_input);
            is_first_input = false;
        }

        if !state.utxos.contains(&utxo) {
            println!("New UTXO: {}", utxo);
            state.utxos.push(utxo);
        }
    }

    Ok(())
}
