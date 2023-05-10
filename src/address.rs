use crate::descriptor::SimplicityDescriptor;
use crate::error::Error;
use crate::state::{State, Utxo};
use crate::util;
use elements_miniscript::elements::hashes::hex::FromHex;
use elements_miniscript::elements::{confidential, AssetId, TxOutWitness};
use elements_miniscript::{bitcoin, elements};

pub fn set_address(
    state: &mut State,
    descriptor: SimplicityDescriptor<bitcoin::XOnlyPublicKey>,
) -> Result<elements::Address, Error> {
    let address = descriptor.address(&elements::AddressParams::ELEMENTS);
    state.inbound_address = Some(descriptor);

    Ok(address)
}

pub fn into_utxo(
    state: &mut State,
    txid: elements::Txid,
    output_index: u32,
    value: u64,
) -> Result<(), Error> {
    let descriptor = state.inbound_address.take().ok_or(Error::MissingAddress)?;
    let utxo = Utxo {
        output: elements::TxOut {
            asset: confidential::Asset::Explicit(
                AssetId::from_hex(util::BITCOIN_ASSET_ID).unwrap(),
            ),
            value: confidential::Value::Explicit(value),
            nonce: confidential::Nonce::Null,
            script_pubkey: descriptor.script_pubkey(),
            witness: TxOutWitness::default(),
        },
        descriptor,
        outpoint: elements::OutPoint {
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
