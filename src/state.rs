use crate::error::Error;
use itertools::Itertools;
use miniscript::bitcoin::{
    KeyPair, Network, OutPoint, PrivateKey, PublicKey, TxOut, XOnlyPublicKey,
};
use miniscript::Descriptor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub passive_keys: HashMap<PublicKey, KeyPair>,
    pub active_keys: HashMap<PublicKey, KeyPair>,
    pub inputs: HashMap<usize, Input>,
    pub outputs: HashMap<usize, Output>,
    pub fee: u64,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Input {
    pub descriptor: Descriptor<XOnlyPublicKey>,
    pub utxo: Option<Utxo>,
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.descriptor.fmt(f)?;

        if let Some(utxo) = &self.utxo {
            write!(f, " <- {}", utxo)?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Utxo {
    pub outpoint: OutPoint,
    pub output: TxOut,
}

impl fmt::Display for Utxo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {} sat",
            self.outpoint.vout, self.outpoint.txid, self.output.value
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Output {
    pub value: u64,
    pub descriptor: Descriptor<XOnlyPublicKey>,
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
            inputs: HashMap::new(),
            outputs: HashMap::new(),
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
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Passive keys:\n")?;
        for (_, keypair) in self.passive_keys.iter() {
            let (xonly, _) = keypair.x_only_public_key();
            let prv = PrivateKey::new(keypair.secret_key(), Network::Regtest);
            writeln!(f, "  {}: {}", xonly, prv.to_wif())?;
        }
        f.write_str("Active keys:\n")?;
        for (_, keypair) in self.active_keys.iter() {
            let (xonly, _) = keypair.x_only_public_key();
            let prv = PrivateKey::new(keypair.secret_key(), Network::Regtest);
            writeln!(f, "  {}: {}", xonly, prv.to_wif())?;
        }
        f.write_str("Inputs:\n")?;
        for index in self.inputs.keys().sorted() {
            writeln!(f, "  {}: {}", index, self.inputs[index])?;
        }
        f.write_str("Outputs:\n")?;
        for index in self.outputs.keys().sorted() {
            writeln!(f, "  {}: {}", index, self.outputs[index])?;
        }
        write!(f, "Fee: {} sat", self.fee)?;

        Ok(())
    }
}
