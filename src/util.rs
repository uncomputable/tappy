use elements_miniscript::bitcoin;

pub const BITCOIN_ASSET_ID: &str =
    "b2e15d0d7a0c94e4e2ce0fe6e8691b9e451377f6e46e8045a86f7c4b5d4f0f23";
pub const ELEMENTS_REGTEST_GENESIS_BLOCK_HASH: &str =
    "209577bda6bf4b5804bd46f8621580dd6d4e8bfa2d190e1c50e932492baca07d";
pub const PUBLIC_KEY_UNSPENDABLE: &str =
    "0250929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
pub const TAPLICITY_LEAF_VERSION: u8 = 0xbe;

pub fn into_xonly(key: bitcoin::PublicKey) -> bitcoin::XOnlyPublicKey {
    let (xonly, _parity) = key.inner.x_only_public_key();
    xonly
}
