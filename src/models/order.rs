use crate::api::auth::{build_l1_signature, EIP712Struct};
use crate::models::{OrderType, Side};
use serde::Serialize;

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderQueryBody<'a> {
    order: SignedOrderRequest<'a>,
    owner: &'a str,
    #[serde(rename = "orderType")]
    order_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedOrderRequest<'a> {
    #[serde(serialize_with = "serialize_as_number")]
    salt: &'a str,
    maker: &'a str,
    signer: &'a str,
    taker: &'a str,
    #[serde(rename = "tokenId")]
    token_id: &'a str,
    #[serde(serialize_with = "serialize_i32_as_string")]
    maker_amount: i32,
    #[serde(serialize_with = "serialize_i32_as_string")]
    taker_amount: i32,
    #[serde(serialize_with = "serialize_i64_as_string")]
    expiration: i64,
    #[serde(serialize_with = "serialize_i32_as_string")]
    nonce: i32,
    #[serde(rename = "feeRateBps", serialize_with = "serialize_i32_as_string")]
    fee_rate_bps: i32,
    side: String,
    signature_type: i32,
    signature: String,
}

fn serialize_as_number<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::Error;
    value
        .parse::<u64>()
        .map_err(S::Error::custom)?
        .serialize(serializer)
}

fn serialize_i32_as_string<S>(value: &i32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn serialize_i64_as_string<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

pub struct Order {
    pub maker: String,
    pub signer: String,
    pub taker: String,
    pub token_id: String,
    pub maker_amount: i32,
    pub taker_amount: i32,
    pub expiration: i64,
    pub fee_rate_bps: i32,
    pub side: Side,
    pub signature_type: i32,
    pub order_type: OrderType,
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
            side,
            signature_type: 1, // Polymarket linked wallet
            order_type,
        }
    }

    pub fn build_order_query_body(&self, salt: &str, nonce: i32, api_key: &str, pk: &str) -> String {
        let signature = build_l1_signature(self, salt, nonce, pk).to_string();

        log::debug!("Signature added to msg: {}", signature);

        let signed_order = SignedOrderRequest {
            salt,
            maker: &self.maker,
            signer: &self.signer,
            taker: &self.taker,
            token_id: &self.token_id,
            maker_amount: self.maker_amount,
            taker_amount: self.taker_amount,
            expiration: self.expiration,
            nonce,
            fee_rate_bps: self.fee_rate_bps,
            side: self.side.to_string(),
            signature_type: self.signature_type,
            signature,
        };

        let body = OrderQueryBody {
            order: signed_order,
            owner: api_key,
            order_type: self.order_type.to_string(),
        };

        serde_json::to_string(&body).expect("Error serializing order query body")
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

    fn get_message_values(&self, salt: &str, nonce: i32) -> DynSolValue {
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
            DynSolValue::Uint(U256::from(nonce), 256),
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
    const TEST_TOKEN_ID: &str =
        "9791340778034406846471990250402404386251253109836550662455900621767083631393";
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
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.signature_type, 1);
        assert_eq!(order.order_type, OrderType::FOK);
    }

    #[test]
    fn test_order_with_different_order_types() {
        let order_types = vec![
            OrderType::FOK,
            OrderType::FAK,
            OrderType::GTC,
            OrderType::GTD,
        ];

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

        let expected_body = r#"{"order":{"salt":123456789,"maker":"0x1234567890123456789012345678901234567890","signer":"0x1234567890123456789012345678901234567890","taker":"0x0000000000000000000000000000000000000000","tokenId":"9791340778034406846471990250402404386251253109836550662455900621767083631393","makerAmount":"100","takerAmount":"50","expiration":"9999999999","nonce":"0","feeRateBps":"10","side":"BUY","signatureType":1,"signature":"0x513f7e9ebe22fc80d12446263dc6c89404932a7668aa7c4d54d2d1074d63ef1c31259122bf8c6490dac0a3c1fb7dfcf3285d6fe1d8179cc0a8384b33288787371b"},"owner":"test_api_key","orderType":"FOK"}"#;

        let salt = "123456789";
        let nonce = 0;
        let api_key = "test_api_key";
        let pk = "0x1234567890123456789012345678901234567890123456789012345678901234";

        let body = order.build_order_query_body(salt, nonce, api_key, pk);

        // Verify the body is as expected
        assert!(expected_body.eq(&body));
    }

    #[test]
    fn test_build_order_query_body_order_type_fok() {
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

        let body = order.build_order_query_body(
            "123",
            0,
            "key",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );
        assert!(body.contains("\"orderType\":\"FOK\""));
    }

    #[test]
    fn test_build_order_query_body_order_type_fak() {
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
            OrderType::FAK,
        );

        let body = order.build_order_query_body(
            "123",
            0,
            "key",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );
        assert!(body.contains("\"orderType\":\"FAK\""));
    }

    #[test]
    fn test_build_order_query_body_order_type_gtc() {
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

        let body = order.build_order_query_body(
            "123",
            0,
            "key",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );
        assert!(body.contains("\"orderType\":\"GTC\""));
    }

    #[test]
    fn test_build_order_query_body_order_type_gtd() {
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
            OrderType::GTD,
        );

        let body = order.build_order_query_body(
            "123",
            0,
            "key",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );
        assert!(body.contains("\"orderType\":\"GTD\""));
    }

    #[test]
    fn test_build_order_query_body_buy_side() {
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

        let body = order.build_order_query_body(
            "123",
            0,
            "key",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );
        assert!(body.contains("\"side\":\"BUY\""));
    }

    #[test]
    fn test_build_order_query_body_sell_side() {
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

        let body = order.build_order_query_body(
            "123",
            0,
            "key",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );
        assert!(body.contains("\"side\":\"SELL\""));
    }

    #[test]
    fn test_build_order_query_body_contains_signature() {
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

        let body = order.build_order_query_body(
            "123",
            0,
            "key",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );

        // Signature should be present in the body
        assert!(body.contains("\"signature\":\"0x"));
    }

    #[test]
    fn test_build_order_query_body_includes_api_key() {
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

        let api_key = "my_test_api_key_12345";
        let body = order.build_order_query_body(
            "123",
            0,
            api_key,
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );

        assert!(body.contains(&format!("\"owner\":\"{}\"", api_key)));
    }

    #[test]
    fn test_build_order_query_body_includes_salt() {
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

        let salt = "987654321";
        let body = order.build_order_query_body(
            salt,
            0,
            "key",
            "0x1234567890123456789012345678901234567890123456789012345678901234",
        );

        assert!(body.contains(&format!("\"salt\":{}", salt)));
    }
}
