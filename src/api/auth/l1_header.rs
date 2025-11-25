use crate::api::auth::EIP712Struct;

use alloy::{
    dyn_abi::{DynSolType, DynSolValue},
    primitives::{keccak256, Address, B256, U256},
};

use std::str::FromStr;

// Domain constants
const NAME: &str = "ClobAuthDomain";
const VERSION: &str = "1";
const CHAIN_ID: i32 = 137;

// Message constants
const NONCE: i32 = 0;
const MESSAGE: &str = "This message attests that I control the given wallet";

pub struct L1Header {
    signer: String,
}

impl L1Header {
    pub fn new(signer: &str) -> Self {
        L1Header {
            signer: signer.to_string(),
        }
    }

    fn get_signer(&self) -> &str {
        self.signer.as_str()
    }
}

impl EIP712Struct for L1Header {
    fn get_domain_type_hash(&self) -> B256 {
        keccak256("EIP712Domain(string name,string version,uint256 chainId)")
    }

    fn get_domain_values(&self) -> DynSolValue {
        let _domain_type = DynSolType::Tuple(vec![
            DynSolType::String,
            DynSolType::String,
            DynSolType::Uint(256),
        ]);

        DynSolValue::Tuple(vec![
            DynSolValue::String(NAME.to_string()),
            DynSolValue::String(VERSION.to_string()),
            DynSolValue::Uint(U256::from(CHAIN_ID), 256),
        ])
    }

    fn get_message_type_hash(&self) -> B256 {
        keccak256("Order(address address,string timestamp,uint256 nonce,string message)")
    }

    fn get_message_values(&self, salt: &str, _nonce: i32) -> DynSolValue {
        let _message_type = DynSolType::Tuple(vec![
            DynSolType::Address,
            DynSolType::String,
            DynSolType::Uint(256),
            DynSolType::String,
        ]);

        DynSolValue::Tuple(vec![
            DynSolValue::Address(Address::from_str(self.get_signer()).unwrap()),
            DynSolValue::String(salt.to_string()),
            DynSolValue::Uint(U256::from(NONCE), 256),
            DynSolValue::String(MESSAGE.to_string()),
        ])
    }
}
