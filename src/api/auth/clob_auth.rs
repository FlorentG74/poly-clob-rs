use alloy::{
    dyn_abi::DynSolValue,
    hex,
    primitives::{keccak256, Address, B256, U256},
    signers::{local::PrivateKeySigner, Signer as AlloySigner},
};
use futures::executor::block_on;
use std::str::FromStr;

use super::EIP712Struct;

fn generate_values_hash(value: &DynSolValue) -> Vec<u8> {
    let mut encoded_values: Vec<u8> = Vec::new();

    let tup = value.as_tuple().unwrap();
    for val in tup {
        log::debug!("Value: {:?}", val);

        let typ = val.as_type().unwrap();

        log::debug!("Type: {}", typ.to_string());
        match typ.to_string().as_str() {
            "string" => {
                let str = val.as_str().unwrap();
                let encoded_str = keccak256(str);
                log::debug!("Result: {encoded_str}");
                encoded_values.extend_from_slice(encoded_str.as_slice());
            }
            "uint8" => {
                let uint8 = val.as_uint().unwrap();
                let x: [u8; 32] = uint8.0.to_be_bytes();
                let encoded_uint8: [u8; 32] = U256::from_be_slice(&x).to_be_bytes();
                log::debug!("Result: {:?}", &encoded_uint8);
                encoded_values.extend_from_slice(&encoded_uint8);
            }
            "uint256" => {
                let uint256 = val.as_uint().unwrap();
                let x: [u8; 32] = uint256.0.to_be_bytes();
                log::debug!("Result: {:?}", x);
                encoded_values.extend_from_slice(&x);
            }
            "address" => {
                let address: Address = val.as_address().unwrap();
                let address_slice = address.as_slice();

                let encoded_address: [u8; 32] = U256::from_be_slice(address_slice).to_be_bytes();

                log::debug!("Result: {:?}", encoded_address);
                encoded_values.extend_from_slice(&encoded_address);
            }
            _ => panic!("Unknown Type"),
        }
    }

    encoded_values
}

fn get_encoded_domain(eip712_struct: &dyn EIP712Struct) -> B256 {
    let domain_type_hash = eip712_struct.get_domain_type_hash();

    let encoded_domain_values = generate_values_hash(&eip712_struct.get_domain_values());

    let encoded_domain_full_bytes = [&domain_type_hash[..], &encoded_domain_values[..]].concat();

    keccak256(encoded_domain_full_bytes)
}

pub fn build_l1_signature(eip712_struct: &dyn EIP712Struct, salt: &str, signer_pk: &str) -> String {
    let encoded_domain = get_encoded_domain(eip712_struct);

    let message_value = eip712_struct.get_message_values(salt);
    let eip712_message_type_hash = eip712_struct.get_message_type_hash();
    let encoded_message_values = generate_values_hash(&message_value);

    let encoded_message_full_bytes =
        [&eip712_message_type_hash[..], &encoded_message_values[..]].concat();
    let encoded_message_full = keccak256(encoded_message_full_bytes);

    let signable_bytes = [
        &[0x19, 0x01],
        &encoded_domain[..],
        &encoded_message_full[..],
    ]
    .concat();
    let eip712_hash = keccak256(&signable_bytes);

    log::debug!("Signable bytes: {:?}", &signable_bytes);
    log::debug!("Message hash: {:?}", &eip712_hash);

    let wallet = PrivateKeySigner::from_str(signer_pk).unwrap();
    log::debug!("\nSigner address: {}", wallet.address());

    let signature = block_on(wallet.sign_hash(&eip712_hash)).unwrap();
    log::debug!("Signature: 0x{}", hex::encode(signature.as_bytes()));

    let mut result = "0x".to_string();
    result.push_str(hex::encode(signature.as_bytes()).as_str());

    result
}
