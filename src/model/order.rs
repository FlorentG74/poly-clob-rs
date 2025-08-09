use crate::controller::{clob_auth, EIP712Struct};
use string_builder::Builder;

use alloy::{
    dyn_abi::{DynSolType, DynSolValue},
    primitives::{address, keccak256, Address, B256, U256},
};

use std::str::FromStr;

const NAME: &str = "Polymarket CTF Exchange";
const VERSION: &str = "1";
const CHAIN_ID: i32 = 137;
const VERIFYING_CONTRACT: Address = address!("C5d563A36AE78145C45a50134d48A1215220f80a");

pub struct Order {
    pub maker: String,
    pub signer: String,
    pub taker: String,
    pub token_id: String,
    pub maker_amount: i32,
    pub taker_amount: i32,
    pub expiration: i64,
    pub nonce: i32,
    pub fee_rate_bps: i32,
    pub side: i32,
    pub signature_type: i32,
    pub order_type: String,
    signature: String,
}

impl Order {
    pub fn new(
        maker: &str,
        signer: &str,
        taker: &str,
        token_id: &str,
        maker_amount: i32,
        taker_amount: i32,
        expiration: i64,
        fee_rate_bps: i32,
        side: i32,
        order_type: &str,
    ) -> Self {
        Order {
            maker: maker.to_string(),
            signer: signer.to_string(),
            taker: taker.to_string(),
            token_id: token_id.to_string(),
            maker_amount,
            taker_amount,
            expiration,
            fee_rate_bps,
            nonce: 0,
            side,
            signature_type: 0,
            order_type: order_type.to_string(),
            signature: "".to_string(),
        }
    }

    pub fn build_order_query_body(&mut self, salt: &str, api_key: &str, pk: &str) -> String {
        let mut builder = Builder::default();

        self.signature = clob_auth::build_l1_signature(self, salt, pk).to_string();

        log::debug!("Signature added to msg: {}", self.signature);

        builder.append("{\"order\": {");
        builder.append("\"salt\": ");
        builder.append(salt);
        builder.append(",\"maker\": \"");
        builder.append(self.maker.clone());
        builder.append("\",\"signer\": \"");
        builder.append(self.signer.clone());
        builder.append("\",\"taker\": \"");
        builder.append(self.taker.clone());
        builder.append("\",\"tokenId\": \"");
        builder.append(self.token_id.clone());
        builder.append("\",\"makerAmount\": \"");
        builder.append(self.maker_amount.to_string());
        builder.append("\",\"takerAmount\": \"");
        builder.append(self.taker_amount.to_string());
        builder.append("\",\"expiration\": \"");
        builder.append(self.expiration.to_string());
        builder.append("\",\"nonce\": \"");
        builder.append(self.nonce.to_string());
        builder.append("\",\"feeRateBps\": \"");
        builder.append(self.fee_rate_bps.to_string());
        builder.append("\",\"side\": \"");
        builder.append("BUY"); // 0 vs 1
        builder.append("\",\"signatureType\": ");
        builder.append(self.signature_type.to_string());
        builder.append(",\"signature\": \"");
        builder.append(self.signature.clone());
        builder.append("\"},");
        builder.append("");
        builder.append("\"owner\": \"");
        builder.append(api_key);
        builder.append("\",\"orderType\": \"");
        builder.append(self.order_type.to_string());
        builder.append("\"}");

        builder.string().expect("Error in String conversion")
    }
}

impl EIP712Struct for Order {
    fn get_domain_type_hash(&self) -> B256 {
        keccak256(
            "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        )
    }

    fn get_domain_values(&self) -> DynSolValue {
        // EIP-712 domain
        let _domain_type = DynSolType::Tuple(vec![
            DynSolType::String,    // name
            DynSolType::String,    // version
            DynSolType::Uint(256), // chainId
            DynSolType::Address,   // verifyingContract
        ]);

        DynSolValue::Tuple(vec![
            DynSolValue::String(NAME.to_string()),
            DynSolValue::String(VERSION.to_string()),
            DynSolValue::Uint(U256::from(CHAIN_ID), 256),
            DynSolValue::Address(VERIFYING_CONTRACT),
        ])
    }

    fn get_message_type_hash(&self) -> B256 {
        keccak256("Order(uint256 salt,address maker,address signer,address taker,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint256 expiration,uint256 nonce,uint256 feeRateBps,uint8 side,uint8 signatureType)")
    }

    fn get_message_values(&self, salt: &str) -> DynSolValue {
        // Message type (Order structure)
        let _message_type = DynSolType::Tuple(vec![
            DynSolType::Uint(256),
            DynSolType::Address,
            DynSolType::Address,
            DynSolType::Address,
            DynSolType::Uint(256),
            DynSolType::Uint(256),
            DynSolType::Uint(256),
            DynSolType::Uint(256),
            DynSolType::Uint(256),
            DynSolType::Uint(256),
            DynSolType::Uint(8),
            DynSolType::Uint(8),
        ]);

        // Populate values from object
        DynSolValue::Tuple(vec![
            DynSolValue::Uint(U256::from_str(salt).unwrap(), 256),
            DynSolValue::Address(Address::from_str(self.maker.as_str()).unwrap()),
            DynSolValue::Address(Address::from_str(self.signer.as_str()).unwrap()),
            DynSolValue::Address(Address::from_str(self.taker.as_str()).unwrap()),
            DynSolValue::Uint(U256::from_str(self.token_id.as_str()).unwrap(), 256),
            DynSolValue::Uint(U256::from(self.maker_amount), 256),
            DynSolValue::Uint(U256::from(self.taker_amount), 256),
            DynSolValue::Uint(U256::from(self.expiration), 256),
            DynSolValue::Uint(U256::from(self.nonce), 256),
            DynSolValue::Uint(U256::from(self.fee_rate_bps), 256),
            DynSolValue::Uint(U256::from(self.side), 8),
            DynSolValue::Uint(U256::from(self.signature_type), 8),
        ])
    }
}
