use crate::error::Error;
use crate::state::{Input, State};
use itertools::Itertools;
use miniscript::bitcoin::Sequence;

pub fn add_from_utxo(
    state: &mut State,
    input_index: usize,
    utxo_index: usize,
) -> Result<Option<Input>, Error> {
    let utxo = state.utxos.get(utxo_index).ok_or(Error::MissingUtxo)?;
    let input = Input {
        utxo: utxo.clone(),
        sequence: Sequence::MAX,
    };
    if state.inputs.values().contains(&input) {
        return Err(Error::DoubleSpend);
    }

    println!("New input #{}: {}", input_index, input);
    let old = state.inputs.insert(input_index, input);

    Ok(old)
}

pub fn delete_input(state: &mut State, input_index: usize) -> Result<Input, Error> {
    state.inputs.remove(&input_index).ok_or(Error::MissingInput)
}

pub fn update_sequence_height(
    state: &mut State,
    input_index: usize,
    relative_height: u16,
) -> Result<(), Error> {
    let input = state
        .inputs
        .get_mut(&input_index)
        .ok_or(Error::MissingInput)?;
    input.sequence = Sequence::from_height(relative_height);

    Ok(())
}

pub fn set_sequence_max(state: &mut State, input_index: usize) -> Result<(), Error> {
    let input = state
        .inputs
        .get_mut(&input_index)
        .ok_or(Error::MissingInput)?;
    input.sequence = Sequence::MAX;

    Ok(())
}
