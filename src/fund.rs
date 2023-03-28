use crate::error::Error;
use crate::state::State;
use miniscript::bitcoin::{Address, Network};

pub fn get_input_address(state: &State, index: usize) -> Result<Address, Error> {
    let input = state.inputs.get(&index).ok_or(Error::MissingInput)?;
    let address = input.descriptor.address(Network::Regtest)?;

    Ok(address)
}
