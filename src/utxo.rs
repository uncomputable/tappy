use crate::error::Error;
use crate::state::{State, Utxo};

pub fn list_utxos(state: &State) {
    println!("UTXOs:");
    for (index, utxo) in state.utxos.iter().enumerate() {
        println!("{}: {}", index, utxo);
    }
}

pub fn delete_utxo(state: &mut State, utxo_index: usize) -> Result<Utxo, Error> {
    if state.utxos.len() <= utxo_index {
        return Err(Error::MissingUtxo);
    }

    let old = state.utxos.remove(utxo_index);
    Ok(old)
}
