use crate::error::Error;
use crate::state::{State, Utxo};
use crate::util;
use miniscript::{bitcoin, Descriptor};

pub fn set_address(
    state: &mut State,
    descriptor: Descriptor<bitcoin::XOnlyPublicKey>,
) -> Result<bitcoin::Address, Error> {
    util::verify_taproot(&descriptor)?;

    let address = descriptor.address(bitcoin::Network::Regtest).unwrap();
    state.inbound_address = Some(descriptor);

    Ok(address)
}

pub fn into_utxo(
    state: &mut State,
    txid: bitcoin::Txid,
    output_index: u32,
    value: u64,
) -> Result<(), Error> {
    let descriptor = state.inbound_address.take().ok_or(Error::MissingAddress)?;
    let utxo = Utxo {
        output: bitcoin::TxOut {
            value,
            script_pubkey: descriptor.script_pubkey(),
        },
        descriptor,
        outpoint: bitcoin::OutPoint {
            txid,
            vout: output_index,
        },
    };

    if !state.utxos.contains(&utxo) {
        println!("New UTXO #{}: {}", state.utxos.len(), utxo);
        state.utxos.push(utxo);
    }

    Ok(())
}
