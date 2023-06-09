use crate::error::Error;
use crate::state::{Output, State};
use crate::util;
use miniscript::{bitcoin, Descriptor};

pub fn add_output(
    state: &mut State,
    output_index: usize,
    descriptor: Descriptor<bitcoin::XOnlyPublicKey>,
    value: u64,
) -> Result<Option<Output>, Error> {
    util::verify_taproot(&descriptor)?;

    if state.outputs.values().any(|o| o.value == 0) {
        return Err(Error::OneZeroOutput);
    }

    let output = Output { value, descriptor };
    println!("New output #{}: {}", output_index, output);
    let old = state.outputs.insert(output_index, output);

    Ok(old)
}

pub fn delete_output(state: &mut State, output_index: usize) -> Result<Output, Error> {
    state
        .outputs
        .remove(&output_index)
        .ok_or(Error::MissingOutput)
}
