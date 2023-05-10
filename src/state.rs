use crate::descriptor::SimplicityDescriptor;
use crate::error::Error;
use elements_miniscript::bitcoin::hashes::sha256;
use elements_miniscript::elements;
use elements_miniscript::elements::{secp256k1_zkp, LockTime, Sequence};
use elements_miniscript::{bitcoin, Preimage32};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub passive_keys: HashMap<bitcoin::PublicKey, bitcoin::KeyPair>,
    pub active_keys: HashMap<bitcoin::PublicKey, bitcoin::KeyPair>,
    pub passive_images: HashMap<sha256::Hash, Preimage32>,
    pub active_images: HashMap<sha256::Hash, Preimage32>,
    pub inbound_address: Option<SimplicityDescriptor<bitcoin::XOnlyPublicKey>>,
    pub utxos: Vec<Utxo>,
    pub inputs: HashMap<usize, Input>,
    pub outputs: HashMap<usize, Output>,
    pub locktime: LockTime,
    pub fee: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Input {
    pub utxo: Utxo,
    pub sequence: Sequence,
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.utxo)?;

        if self.sequence != Sequence::MAX {
            let relative_timelock = self.sequence.0;
            write!(f, " +{} blocks", relative_timelock)?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Utxo {
    pub descriptor: SimplicityDescriptor<bitcoin::XOnlyPublicKey>,
    pub outpoint: elements::OutPoint,
    pub output: elements::TxOut,
}

impl fmt::Display for Utxo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}:{} {} sat",
            self.descriptor, self.outpoint.txid, self.outpoint.vout, self.output.value
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Output {
    pub value: u64,
    pub descriptor: SimplicityDescriptor<bitcoin::XOnlyPublicKey>,
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} sat", self.descriptor, self.value)
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            passive_keys: HashMap::new(),
            active_keys: HashMap::new(),
            passive_images: HashMap::new(),
            active_images: HashMap::new(),
            inbound_address: None,
            utxos: Vec::new(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            locktime: LockTime::ZERO,
            fee: 0,
        }
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let state = serde_json::from_reader(reader)?;
        Ok(state)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P, init: bool) -> Result<(), Error> {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create_new(init)
            .open(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }

    pub(crate) fn locktime_enabled(&self) -> bool {
        for input in self.inputs.values() {
            if input.sequence.enables_absolute_lock_time() {
                return true;
            }
        }

        false
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Keys (xonly: WIF) [disabled for spending]:")?;
        fmt_keys(&self.passive_keys, f)?;
        writeln!(f, "Keys (xonly: WIF) [enabled]:")?;
        fmt_keys(&self.active_keys, f)?;
        writeln!(f, "Images (preimage: image) [disabled for spending]:")?;
        fmt_images(&self.passive_images, f)?;
        writeln!(f, "Images (preimage: image) [enabled]:")?;
        fmt_images(&self.active_images, f)?;
        writeln!(f, "Inputs:")?;
        for index in self.inputs.keys().sorted() {
            writeln!(f, "  {}: {}", index, self.inputs[index])?;
        }
        writeln!(f, "Outputs:")?;
        for index in self.outputs.keys().sorted() {
            writeln!(f, "  {}: {}", index, self.outputs[index])?;
        }
        writeln!(
            f,
            "Locktime: ={} blocks [{}]",
            self.locktime,
            if self.locktime_enabled() {
                "enabled"
            } else {
                "disabled"
            }
        )?;
        write!(f, "Fee: {} sat", self.fee)?;

        Ok(())
    }
}

fn get_private_key(
    secret_key: secp256k1_zkp::SecretKey,
    params: &elements::AddressParams,
) -> bitcoin::PrivateKey {
    let network = match params {
        &elements::AddressParams::LIQUID => bitcoin::Network::Bitcoin,
        _ => bitcoin::Network::Regtest,
    };
    bitcoin::PrivateKey::new(secret_key, network)
}

fn fmt_keys(
    keys: &HashMap<bitcoin::PublicKey, bitcoin::KeyPair>,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    for keypair in keys.values() {
        let (xonly, _) = keypair.x_only_public_key();
        let prv = get_private_key(keypair.secret_key(), &elements::AddressParams::ELEMENTS);
        writeln!(f, "  {}: {}", xonly, prv.to_wif())?;
    }

    Ok(())
}

fn fmt_images(
    images: &HashMap<sha256::Hash, Preimage32>,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    for (image, preimage) in images {
        write!(f, "  {}: ", image)?;
        for byte in preimage {
            write!(f, "{:02x}", byte)?;
        }
        writeln!(f)?;
    }

    Ok(())
}
