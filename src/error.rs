use elements_miniscript::bitcoin::hashes::hex;
use elements_miniscript::elements;
use std::{fmt, io};
use thiserror::Error;

#[derive(Error)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Miniscript(#[from] elements_miniscript::Error),
    #[error("{0}")]
    Hex(#[from] hex::Error),
    #[error("Inbound address is missing")]
    MissingAddress,
    #[error("No UTXO at index")]
    MissingUtxo,
    #[error("Input is missing")]
    MissingInput,
    #[error("Output is missing")]
    MissingOutput,
    #[error("Invalid block height")]
    InvalidHeight,
    #[error("{0}")]
    Simplicity(#[from] simplicity::Error),
    #[error("Sanity check failed: Simplicity program rejected witness")]
    SimplicitySanityCheck,
    #[error("{0}")]
    Taproot(#[from] elements::taproot::TaprootError),
    #[error("{0}")]
    TaprootBuilder(#[from] elements::taproot::TaprootBuilderError),
    #[error("Unknown public key")]
    UnknownKey,
    #[error("Unknown hash image")]
    UnknownImage,
    #[error("Not enough funds to fund remaining output")]
    NotEnoughFunds,
    #[error("At most one output can have zero value")]
    OneZeroOutput,
    #[error("Same UTXO can be used at most once as input")]
    DoubleSpend,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<simplicity::types::Error> for Error {
    fn from(error: simplicity::types::Error) -> Self {
        Self::Simplicity(simplicity::Error::Type(error))
    }
}
