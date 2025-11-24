use crate::api::auth::{build_l1_signature, EIP712Struct};
use crate::models::{Side, OrderType};
use string_builder::Builder;

use alloy::{
    dyn_abi::{DynSolType, DynSolValue},
    primitives::{address, keccak256, Address, B256, U256},
};

use std::str::FromStr;

const NAME: &str = "Polymarket CTF Exchange";
const VERSION: &str = "1";
const CHAIN_ID: i32 = 137;

// TODO make contract dynamic depending on market
// Non-Neg Risk markets
const VERIFYING_CONTRACT: Address = address!("4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E");

// Neg Risk markets
//const VERIFYING_CONTRACT: Address = address!("C5d563A36AE78145C45a50134d48A1215220f80a");

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
    pub side: Side,
    pub signature_type: i32,
    pub order_type: OrderType,
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
        side: Side,
        order_type: OrderType,
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
            signature_type: 1, // Polymarket linked wallet
            order_type,
            signature: "".to_string(),
        }
    }

    pub fn build_order_query_body(&mut self, salt: &str, api_key: &str, pk: &str) -> String {
        let mut builder = Builder::default();

        self.signature = build_l1_signature(self, salt, pk).to_string();

        let buy_sell = self.side.to_string();

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
        builder.append(buy_sell); // 0 vs 1
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
            DynSolValue::Uint(U256::from(self.side.to_int()), 8),
            DynSolValue::Uint(U256::from(self.signature_type), 8),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test values from poly_order_test
    const TEST_TOKEN_ID: &str = "9791340778034406846471990250402404386251253109836550662455900621767083631393";
    const TEST_MAKER: &str = "0x1234567890123456789012345678901234567890"; // Mock address
    const TEST_TAKER: &str = "0x0000000000000000000000000000000000000000"; // Zero address

    #[test]
    fn test_order_new() {
        let order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::FOK,
        );

        assert_eq!(order.maker, TEST_MAKER);
        assert_eq!(order.signer, TEST_MAKER);
        assert_eq!(order.taker, TEST_TAKER);
        assert_eq!(order.token_id, TEST_TOKEN_ID);
        assert_eq!(order.maker_amount, 100);
        assert_eq!(order.taker_amount, 50);
        assert_eq!(order.expiration, 9999999999);
        assert_eq!(order.fee_rate_bps, 10);
        assert_eq!(order.nonce, 0);
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.signature_type, 1);
        assert_eq!(order.order_type, OrderType::FOK);
        assert_eq!(order.signature, "");
    }

    #[test]
    fn test_order_with_different_order_types() {
        let order_types = vec![OrderType::FOK, OrderType::FAK, OrderType::GTC, OrderType::GTD];

        for order_type in order_types {
            let order = Order::new(
                TEST_MAKER,
                TEST_MAKER,
                TEST_TAKER,
                TEST_TOKEN_ID,
                100,
                50,
                9999999999,
                10,
                Side::Buy,
                order_type,
            );

            assert_eq!(order.order_type, order_type);
        }
    }

    #[test]
    fn test_order_with_buy_side() {
        let order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::GTC,
        );

        assert_eq!(order.side, Side::Buy);
    }

    #[test]
    fn test_order_with_sell_side() {
        let order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Sell,
            OrderType::GTC,
        );

        assert_eq!(order.side, Side::Sell);
    }

    #[test]
    fn test_build_order_query_body_structure() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::FOK,
        );

        let expected_body = r#"{"order": {"salt": 123456789,"maker": "0x1234567890123456789012345678901234567890","signer": "0x1234567890123456789012345678901234567890","taker": "0x0000000000000000000000000000000000000000","tokenId": "9791340778034406846471990250402404386251253109836550662455900621767083631393","makerAmount": "100","takerAmount": "50","expiration": "9999999999","nonce": "0","feeRateBps": "10","side": "BUY","signatureType": 1,"signature": "0x513f7e9ebe22fc80d12446263dc6c89404932a7668aa7c4d54d2d1074d63ef1c31259122bf8c6490dac0a3c1fb7dfcf3285d6fe1d8179cc0a8384b33288787371b"},"owner": "test_api_key","orderType": "FOK"}"#;

        let salt = "123456789";
        let api_key = "test_api_key";
        let pk = "0x1234567890123456789012345678901234567890123456789012345678901234";

        let body = order.build_order_query_body(salt, api_key, pk);

        // Verify the body is as expected
        assert!(expected_body.eq(&body));
    }

    #[test]
    fn test_build_order_query_body_order_type_fok() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::FOK,
        );

        let body = order.build_order_query_body("123", "key", "0x1234567890123456789012345678901234567890123456789012345678901234");
        assert!(body.contains("\"orderType\": \"FOK\""));
    }

    #[test]
    fn test_build_order_query_body_order_type_fak() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::FAK,
        );

        let body = order.build_order_query_body("123", "key", "0x1234567890123456789012345678901234567890123456789012345678901234");
        assert!(body.contains("\"orderType\": \"FAK\""));
    }

    #[test]
    fn test_build_order_query_body_order_type_gtc() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::GTC,
        );

        let body = order.build_order_query_body("123", "key", "0x1234567890123456789012345678901234567890123456789012345678901234");
        assert!(body.contains("\"orderType\": \"GTC\""));
    }

    #[test]
    fn test_build_order_query_body_order_type_gtd() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::GTD,
        );

        let body = order.build_order_query_body("123", "key", "0x1234567890123456789012345678901234567890123456789012345678901234");
        assert!(body.contains("\"orderType\": \"GTD\""));
    }

    #[test]
    fn test_build_order_query_body_buy_side() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::GTC,
        );

        let body = order.build_order_query_body("123", "key", "0x1234567890123456789012345678901234567890123456789012345678901234");
        assert!(body.contains("\"side\": \"BUY\""));
    }

    #[test]
    fn test_build_order_query_body_sell_side() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Sell,
            OrderType::GTC,
        );

        let body = order.build_order_query_body("123", "key", "0x1234567890123456789012345678901234567890123456789012345678901234");
        assert!(body.contains("\"side\": \"SELL\""));
    }

    #[test]
    fn test_build_order_query_body_contains_signature() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::FOK,
        );

        let body = order.build_order_query_body("123", "key", "0x1234567890123456789012345678901234567890123456789012345678901234");

        // Signature should be populated after building the body
        assert!(order.signature.len() > 0);
        assert!(body.contains(&format!("\"signature\": \"{}\"", order.signature)));
    }

    #[test]
    fn test_build_order_query_body_includes_api_key() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::FOK,
        );

        let api_key = "my_test_api_key_12345";
        let body = order.build_order_query_body("123", api_key, "0x1234567890123456789012345678901234567890123456789012345678901234");

        assert!(body.contains(&format!("\"owner\": \"{}\"", api_key)));
    }

    #[test]
    fn test_build_order_query_body_includes_salt() {
        let mut order = Order::new(
            TEST_MAKER,
            TEST_MAKER,
            TEST_TAKER,
            TEST_TOKEN_ID,
            100,
            50,
            9999999999,
            10,
            Side::Buy,
            OrderType::FOK,
        );

        let salt = "987654321";
        let body = order.build_order_query_body(salt, "key", "0x1234567890123456789012345678901234567890123456789012345678901234");

        assert!(body.contains(&format!("\"salt\": {}", salt)));
    }
}
