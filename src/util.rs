use crate::error::Error;
use miniscript::descriptor::DescriptorType;
use miniscript::{bitcoin, Descriptor};

pub fn verify_taproot(descriptor: &Descriptor<bitcoin::XOnlyPublicKey>) -> Result<(), Error> {
    if let DescriptorType::Tr = descriptor.desc_type() {
        Ok(())
    } else {
        Err(Error::OnlyTaproot)
    }
}
