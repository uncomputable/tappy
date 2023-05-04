use crate::error::Error;
use crate::state::State;
use miniscript::bitcoin::hashes::{sha256, Hash};
use miniscript::bitcoin::secp256k1::rand::rngs::OsRng;
use miniscript::bitcoin::secp256k1::rand::Rng;
use miniscript::bitcoin::secp256k1::{Parity, Secp256k1};
use miniscript::ToPublicKey;
use miniscript::{bitcoin, Preimage32};

pub fn generate_keys(state: &mut State, number: u32) -> Result<(), Error> {
    let secp = Secp256k1::new();

    for _ in 0..number {
        let (mut seckey, mut pubkey) = secp.generate_keypair(&mut OsRng);
        let (_, parity) = pubkey.x_only_public_key();

        if parity == Parity::Odd {
            seckey = seckey.negate();
            pubkey = seckey.public_key(&secp);
        }

        let public_key = pubkey.to_public_key();
        let keypair = seckey.keypair(&secp);
        state.passive_keys.insert(public_key, keypair);
    }

    Ok(())
}

pub fn generate_images(state: &mut State, number: u32) -> Result<(), Error> {
    let mut rng = OsRng;

    for _ in 0..number {
        let preimage: Preimage32 = rng.gen();
        let image = sha256::Hash::hash(&preimage);
        state.passive_images.insert(image, preimage);
    }

    Ok(())
}

pub fn toggle_key(state: &mut State, pubkey: bitcoin::XOnlyPublicKey) -> Result<(), Error> {
    let public_key = pubkey.to_public_key();

    if let Some(keypair) = state.passive_keys.remove(&public_key) {
        state.active_keys.insert(public_key, keypair);
    } else if let Some(keypair) = state.active_keys.remove(&public_key) {
        state.passive_keys.insert(public_key, keypair);
    } else {
        return Err(Error::UnknownKey);
    }

    Ok(())
}

pub fn toggle_image(state: &mut State, image: sha256::Hash) -> Result<(), Error> {
    if let Some(preimage) = state.passive_images.remove(&image) {
        state.active_images.insert(image, preimage);
    } else if let Some(preimage) = state.active_images.remove(&image) {
        state.passive_images.insert(image, preimage);
    } else {
        return Err(Error::UnknownImage);
    }

    Ok(())
}
