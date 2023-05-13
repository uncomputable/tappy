use crate::error::Error;
use crate::state::State;
use miniscript::descriptor::DescriptorType;
use miniscript::{bitcoin, Descriptor};

pub fn verify_taproot(descriptor: &Descriptor<bitcoin::XOnlyPublicKey>) -> Result<(), Error> {
    if let DescriptorType::Tr = descriptor.desc_type() {
        Ok(())
    } else {
        Err(Error::OnlyTaproot)
    }
}

pub fn into_xonly(key: bitcoin::PublicKey) -> bitcoin::XOnlyPublicKey {
    let (xonly, _parity) = key.inner.x_only_public_key();
    xonly
}

pub fn get_remaining_funds(state: &State) -> Result<Option<(usize, u64)>, Error> {
    let input_funds = state
        .inputs
        .values()
        .fold(0, |x, i| x + i.utxo.output.value);
    let output_funds = state.outputs.values().fold(0, |x, o| x + o.value) + state.fee;

    if let Some((output_index, _)) = state.outputs.iter().find(|(_, o)| o.value == 0) {
        let remaining_funds = input_funds
            .checked_sub(output_funds)
            .ok_or(Error::NotEnoughFunds)?;
        return Ok(Some((*output_index, remaining_funds)));
    }

    Ok(None)
}
