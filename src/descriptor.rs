use crate::error::Error;
use crate::util;
use elements_miniscript::elements::secp256k1_zkp;
use elements_miniscript::elements::taproot::{LeafVersion, TaprootBuilder, TaprootSpendInfo};
use elements_miniscript::{bitcoin, elements, MiniscriptKey, ToPublicKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use simplicity::bitwriter::BitWriter;
use simplicity::policy::key::PublicKey32;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SimplicityDescriptor<Pk: MiniscriptKey> {
    policy: simplicity::Policy<Pk>,
    spend_info: TaprootSpendInfo,
    cmr: simplicity::merkle::cmr::Cmr,
    script: elements::Script,
    version: LeafVersion,
}

impl<Pk: PublicKey32 + ToPublicKey> SimplicityDescriptor<Pk> {
    pub fn new(policy: simplicity::Policy<Pk>) -> Result<Self, Error> {
        let internal_key = bitcoin::PublicKey::from_str(util::PUBLIC_KEY_UNSPENDABLE).unwrap();

        let mut context = simplicity::core::Context::default();
        let commit = policy.compile(&mut context)?;
        let cmr = commit.cmr;
        let script = elements::Script::from(Vec::from(cmr.as_ref()));

        let version = LeafVersion::from_u8(util::TAPLICITY_LEAF_VERSION).unwrap();
        let builder = TaprootBuilder::new().add_leaf_with_ver(0, script.clone(), version)?;
        let secp = secp256k1_zkp::Secp256k1::verification_only();
        let spend_info = builder.finalize(&secp, internal_key.to_x_only_pubkey())?;

        Ok(Self {
            policy,
            spend_info,
            cmr,
            script,
            version,
        })
    }

    pub fn spend_info(&self) -> &TaprootSpendInfo {
        &self.spend_info
    }

    pub fn script_pubkey(&self) -> elements::Script {
        let output_key = self.spend_info().output_key();
        let builder = elements::script::Builder::new();
        builder
            .push_opcode(elements::opcodes::all::OP_PUSHNUM_1)
            .push_slice(&output_key.as_inner().serialize())
            .into_script()
    }

    pub fn address(&self, params: &'static elements::AddressParams) -> elements::Address {
        let output_key = self.spend_info().output_key();
        elements::Address::p2tr_tweaked(output_key, None, params)
    }

    // TODO: Support multiple tap leaves
    pub fn cmr(&self) -> simplicity::merkle::cmr::Cmr {
        self.cmr
    }

    // TODO: Support multiple tap leaves
    pub fn leaf(&self) -> (elements::Script, LeafVersion) {
        (self.script.clone(), self.version)
    }

    pub fn get_satisfaction(
        &self,
        satisfier: &simplicity::policy::satisfy::PolicySatisfier<Pk>,
    ) -> Result<(Vec<Vec<u8>>, elements::Script), Error> {
        let mut context = simplicity::core::Context::default();
        let commit = self.policy.compile(&mut context)?;
        let wit_values = self.policy.satisfy(satisfier).ok_or(Error::Miniscript(
            elements_miniscript::Error::CouldNotSatisfy,
        ))?;
        let program = commit.finalize(wit_values.into_iter())?;

        let mut program_and_witness_bytes = Vec::<u8>::new();
        let mut writer = BitWriter::new(&mut program_and_witness_bytes);
        program.encode(&mut writer)?;
        writer.flush_all()?;
        debug_assert_ne!(program_and_witness_bytes.len(), 0);
        let cmr_bytes = Vec::from(program.cmr.as_ref());

        // FIXME: Should env be public?
        let control_block = satisfier.env.control_block();
        let witness = vec![
            program_and_witness_bytes,
            cmr_bytes,
            control_block.serialize(),
        ];
        let script_sig = elements::Script::new();

        Ok((witness, script_sig))
    }
}

impl<Pk: MiniscriptKey> fmt::Display for SimplicityDescriptor<Pk> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.policy, f)
    }
}

impl<Pk> FromStr for SimplicityDescriptor<Pk>
where
    Pk: PublicKey32 + ToPublicKey + FromStr,
    <Pk as MiniscriptKey>::Sha256: FromStr,
    <Pk as FromStr>::Err: fmt::Display,
    <<Pk as MiniscriptKey>::Sha256 as FromStr>::Err: fmt::Display,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let policy = simplicity::Policy::from_str(s)?;
        Self::new(policy)
    }
}

impl<Pk: MiniscriptKey> Serialize for SimplicityDescriptor<Pk> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        simplicity::Policy::serialize(&self.policy, serializer)
    }
}

impl<'de, Pk> Deserialize<'de> for SimplicityDescriptor<Pk>
where
    Pk: PublicKey32 + ToPublicKey + FromStr,
    <Pk as MiniscriptKey>::Sha256: FromStr,
    <Pk as MiniscriptKey>::Hash256: FromStr,
    <Pk as MiniscriptKey>::Ripemd160: FromStr,
    <Pk as MiniscriptKey>::Hash160: FromStr,
    <Pk as FromStr>::Err: fmt::Display,
    <<Pk as MiniscriptKey>::Sha256 as FromStr>::Err: fmt::Display,
    <<Pk as MiniscriptKey>::Hash256 as FromStr>::Err: fmt::Display,
    <<Pk as MiniscriptKey>::Ripemd160 as FromStr>::Err: fmt::Display,
    <<Pk as MiniscriptKey>::Hash160 as FromStr>::Err: fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let policy = simplicity::Policy::deserialize(deserializer)?;
        Self::new(policy).map_err(serde::de::Error::custom)
    }
}
