use crate::error::Error;
use crate::state::State;
use miniscript::bitcoin;

pub fn get_input_address(state: &State, index: usize) -> Result<bitcoin::Address, Error> {
    let input = state.inputs.get(&index).ok_or(Error::MissingInput)?;
    let address = input.descriptor.address(bitcoin::Network::Regtest)?;

    Ok(address)
}
