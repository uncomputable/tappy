use crate::error::Error;
use crate::state::State;
use miniscript::bitcoin::secp256k1;
use miniscript::{bitcoin, ToPublicKey};

pub fn generate_keys(state: &mut State, number: u32) -> Result<(), Error> {
    let secp = secp256k1::Secp256k1::new();

    for _ in 0..number {
        let (mut seckey, mut pubkey) = secp.generate_keypair(&mut secp256k1::rand::rngs::OsRng);
        let (_, parity) = pubkey.x_only_public_key();

        if parity == secp256k1::Parity::Odd {
            seckey = seckey.negate();
            pubkey = seckey.public_key(&secp);
        }

        let public_key = pubkey.to_public_key();
        let keypair = seckey.keypair(&secp);
        state.passive_keys.insert(public_key, keypair);
    }

    Ok(())
}

pub fn enable_key(state: &mut State, pubkey: bitcoin::XOnlyPublicKey) -> Result<(), Error> {
    let public_key = pubkey.to_public_key();
    let keypair = state
        .passive_keys
        .remove(&public_key)
        .ok_or(Error::UnknownKey)?;
    state.active_keys.insert(public_key, keypair);

    Ok(())
}

pub fn disable_key(state: &mut State, pubkey: bitcoin::XOnlyPublicKey) -> Result<(), Error> {
    let public_key = pubkey.to_public_key();
    let keypair = state
        .active_keys
        .remove(&public_key)
        .ok_or(Error::UnknownKey)?;
    state.passive_keys.insert(public_key, keypair);

    Ok(())
}

pub fn delete_key(
    state: &mut State,
    pubkey: &bitcoin::XOnlyPublicKey,
) -> Result<bitcoin::KeyPair, Error> {
    let public_key = pubkey.to_public_key();

    if let Some(keypair) = state.active_keys.remove(&public_key) {
        Ok(keypair)
    } else if let Some(keypair) = state.passive_keys.remove(&public_key) {
        Ok(keypair)
    } else {
        Err(Error::UnknownKey)
    }
}
