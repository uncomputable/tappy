use crate::error::Error;
use crate::state::State;
use miniscript::bitcoin::locktime::Height;
use miniscript::bitcoin::LockTime;

pub fn update_locktime(state: &mut State, height: Height) -> Result<(), Error> {
    state.locktime = LockTime::Blocks(height);
    Ok(())
}

pub fn update_fee(state: &mut State, value: u64) -> Result<(), Error> {
    state.fee = value;
    Ok(())
}
