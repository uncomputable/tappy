use crate::error::Error;
use crate::state::State;
use miniscript::bitcoin::hashes::{sha256, Hash};
use miniscript::bitcoin::secp256k1;
use miniscript::bitcoin::secp256k1::rand::Rng;
use miniscript::Preimage32;

pub fn generate_images(state: &mut State, number: u32) -> Result<(), Error> {
    let mut rng = secp256k1::rand::rngs::OsRng;

    for _ in 0..number {
        let preimage: Preimage32 = rng.gen();
        let image = sha256::Hash::hash(&preimage);
        state.passive_images.insert(image, preimage);
    }

    Ok(())
}

pub fn enable_image(state: &mut State, image: sha256::Hash) -> Result<(), Error> {
    let preimage = state
        .passive_images
        .remove(&image)
        .ok_or(Error::UnknownImage)?;
    state.active_images.insert(image, preimage);

    Ok(())
}

pub fn disable_image(state: &mut State, image: sha256::Hash) -> Result<(), Error> {
    let preimage = state
        .active_images
        .remove(&image)
        .ok_or(Error::UnknownImage)?;
    state.passive_images.insert(image, preimage);

    Ok(())
}

pub fn delete_image(state: &mut State, image: &sha256::Hash) -> Result<Preimage32, Error> {
    if let Some(preimage) = state.active_images.remove(image) {
        Ok(preimage)
    } else if let Some(preimage) = state.passive_images.remove(image) {
        Ok(preimage)
    } else {
        Err(Error::UnknownImage)
    }
}
