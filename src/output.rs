use crate::descriptor::SimplicityDescriptor;
use crate::error::Error;
use crate::state::{Output, State};
use elements_miniscript::bitcoin;

pub fn add_output(
    state: &mut State,
    output_index: usize,
    descriptor: SimplicityDescriptor<bitcoin::XOnlyPublicKey>,
    value: u64,
) -> Result<Option<Output>, Error> {
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
