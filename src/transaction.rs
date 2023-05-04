use crate::error::Error;
use crate::state::{Input, State, Utxo};
use itertools::Itertools;
use miniscript::bitcoin;
use miniscript::bitcoin::locktime::Height;
use miniscript::bitcoin::{LockTime, Sequence};

pub fn update_locktime(state: &mut State, height: Height) -> Result<(), Error> {
    state.locktime = LockTime::Blocks(height);
    Ok(())
}

pub fn update_fee(state: &mut State, value: u64) -> Result<(), Error> {
    state.fee = value;
    Ok(())
}

pub fn finalize_transaction(state: &mut State, txid: bitcoin::Txid) -> Result<(), Error> {
    for (_, input) in state.inputs.drain() {
        if let Some(index) = state.utxos.iter().position(|x| x == &input.utxo) {
            let _utxo = state.utxos.remove(index);
        }
    }

    let mut is_first_input = true;

    for (output_index, output) in state.outputs.drain().sorted_by(|(a, _), (b, _)| a.cmp(b)) {
        let utxo = Utxo {
            output: bitcoin::TxOut {
                value: output.value,
                script_pubkey: output.descriptor.script_pubkey(),
            },
            descriptor: output.descriptor,
            outpoint: bitcoin::OutPoint {
                txid,
                vout: output_index as u32,
            },
        };

        if is_first_input {
            let first_input = Input {
                utxo: utxo.clone(),
                sequence: Sequence::MAX,
            };
            state.inputs.insert(0, first_input);
            is_first_input = false;
        }

        if !state.utxos.contains(&utxo) {
            state.utxos.push(utxo);
        }
    }

    Ok(())
}
