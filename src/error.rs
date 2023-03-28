use miniscript::bitcoin::hashes::hex;
use std::{fmt, io};
use thiserror::Error;

#[derive(Error)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Miniscript(#[from] miniscript::Error),
    #[error("{0}")]
    Hex(#[from] hex::Error),
    #[error("Input is missing")]
    MissingInput,
    #[error("UTXO is missing for an input")]
    MissingUtxo,
    #[error("Output is missing")]
    MissingOutput,
    #[error("Unknown public key")]
    UnknownKey,
    #[error("Not enough funds to fund remaining output")]
    NotEnoughFunds,
    #[error("Only Taproot descriptors are supported")]
    OnlyTaproot,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
