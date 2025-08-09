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

    // Encode domain values
    let tup = value.as_tuple().unwrap();
    for val in tup {
        log::debug!("Value: {:?}", val);

        // Conversion code from https://github.com/Polymarket/poly-py-eip712-structs/blob/main/poly_eip712_structs/types.py
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
                // Pad to 32 bits
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

                // Pad to 32 bits
                let encoded_address: [u8; 32] = U256::from_be_slice(address_slice).to_be_bytes();

                //let decoded_address = hex::decode().unwrap();
                log::debug!("Result: {:?}", encoded_address);
                encoded_values.extend_from_slice(&encoded_address);
            }
            _ => panic!("Unknown Type"),
        }
    }

    encoded_values
}

fn get_encoded_domain(eip712_struct: &dyn EIP712Struct) -> B256 {
    // Encode the domain and message types
    // Prepend Domain Type Hash e.g. keccak("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)") to domain
    let domain_type_hash = eip712_struct.get_domain_type_hash();

    // Encode domain separator
    let encoded_domain_values = generate_values_hash(&eip712_struct.get_domain_values());

    let encoded_domain_full_bytes = [&domain_type_hash[..], &encoded_domain_values[..]].concat();

    keccak256(encoded_domain_full_bytes)
}

pub fn build_l1_signature(eip712_struct: &dyn EIP712Struct, salt: &str, signer_pk: &str) -> String {
    // Populate values from object
    let encoded_domain = get_encoded_domain(eip712_struct);

    // Encode Message
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

    // Create a signer
    let wallet = PrivateKeySigner::from_str(signer_pk).unwrap();
    log::debug!("\nSigner address: {}", wallet.address());

    // Sign the EIP-712 hash
    let signature = block_on(wallet.sign_hash(&eip712_hash)).unwrap();
    log::debug!("Signature: 0x{}", hex::encode(signature.as_bytes()));

    let mut result = "0x".to_string();
    result.push_str(hex::encode(signature.as_bytes()).as_str());

    result
}

#[cfg(test)]
mod clob_auth_tests {
    use crate::controller::{build_l1_signature, L1Header};
    use crate::model::Order;

    #[test]
    pub fn test_l1_header_signature() {
        let salt = "479249096354";
        let signer = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";

        let l1_header = L1Header::new(signer);

        let signer_pk = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

        let expected_signature = "0x07828a03719a11fd567eea8dc12b27c064e3f9451a328d8a34de2486945ecb6f70b015ba07d29ead773c85000ae4b952ced617525d951518899da15435ccfaef1c";
        let signature = build_l1_signature(&l1_header, salt, signer_pk);

        assert_eq!(signature, expected_signature);
    }

    #[test]
    pub fn test_l1_message_signature() {
        let salt = "479249096354";
        let maker = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
        let signer = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
        let taker = "0x0000000000000000000000000000000000000000";
        let token_id = "1234";

        let maker_amount = 100000000;
        let taker_amount = 50000000;
        let expiration: i64 = 0;
        let fee_rate_bps = 100;
        let side = 0;

        let order_type = "GTC";

        let order = Order::new(
            maker,
            signer,
            taker,
            token_id,
            maker_amount,
            taker_amount,
            expiration,
            fee_rate_bps,
            side,
            order_type,
        );

        let signer_pk = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

        let expected_signature = "0x79e74485db6836e1c55d5fadf951021011cae6a49b4bf298781dd8942099774e26e91ac7f61d310f78ec0960e06a38ce898b078dd844cf3038da7a7061fafedb1c";
        let signature = build_l1_signature(&order, salt, signer_pk);

        assert_eq!(signature, expected_signature);
    }
}
