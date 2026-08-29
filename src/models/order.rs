use crate::api::auth::{build_l1_signature, EIP712Struct};
use crate::api::error::{Result, SerializationError};
use crate::models::{OrderType, Side};
use crate::ValidationError;
use serde::Serialize;
use typed_builder::TypedBuilder;

use alloy::{
    dyn_abi::DynSolValue,
    primitives::{address, keccak256, Address, B256, FixedBytes, U256},
};
use chrono::Utc;

use std::str::FromStr;

const NAME: &str = "Polymarket CTF Exchange";
const VERSION: &str = "2";
use crate::constants::{MIN_POLY_TOKEN_QUANTITY, POLYGON_CHAIN_ID, RAW_UNIT_MULTIPLIER};

// `POLYGON_CHAIN_ID` is already `u64` and this is only ever widened to `U256`;
// narrowing to `i32` in between served no purpose.
const CHAIN_ID: u64 = POLYGON_CHAIN_ID;

// V2 Non-Neg Risk markets
const NON_NEG_RISK_VERIFYING_CONTRACT: Address =
    address!("E111180000d2663C0091e4f400237545B87B996B");
// V2 Neg Risk markets
const NEG_RISK_VERIFYING_CONTRACT: Address = address!("e2222d279d744050d28e00520010520000310F59");

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
    #[serde(rename = "tokenId")]
    token_id: &'a str,
    #[serde(serialize_with = "serialize_u64_as_string")]
    maker_amount: u64,
    #[serde(serialize_with = "serialize_u64_as_string")]
    taker_amount: u64,
    side: String,
    signature_type: i32,
    #[serde(serialize_with = "serialize_u64_as_string")]
    timestamp: u64,
    metadata: String,
    builder: String,
    #[serde(serialize_with = "serialize_i64_as_string")]
    expiration: i64,
    signature: String,
}

fn serialize_as_number<S>(value: &str, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::Error;
    value
        .parse::<u64>()
        .map_err(S::Error::custom)?
        .serialize(serializer)
}

fn serialize_i64_as_string<S>(value: &i64, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn serialize_u64_as_string<S>(value: &u64, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

/// Random EIP-712 salt for a new order.
#[must_use]
pub fn new_order_salt() -> String {
    let now_ms = Utc::now().timestamp_millis() as u64;
    ((rand::random::<f64>() * now_ms as f64) as u64).to_string()
}

#[derive(TypedBuilder, Clone)]
#[builder(field_defaults(setter(into)))]
pub struct Order {
    pub maker: String,
    pub signer: String,
    pub token_id: String,
    /// Amount the maker gives, in raw units (scaled by [`RAW_UNIT_MULTIPLIER`]).
    ///
    /// `u64` to mirror the on-chain `uint256`: the CLOB accepts these as decimal strings and
    /// signs them as `uint256`, so nothing in the protocol bounds them. They were `i32`,
    /// which silently capped an order at `i32::MAX / 1e6` ≈ 2147 shares or ≈$2147 notional.
    pub maker_amount: u64,
    /// Amount the taker gives, in raw units. See [`Order::maker_amount`].
    pub taker_amount: u64,
    #[builder(default = 0)]
    pub expiration: i64,
    pub side: Side,
    #[builder(default = false)]
    pub neg_risk: bool,
    #[builder(default = 1)]
    pub signature_type: i32,
    pub order_type: OrderType,
    #[builder(default = Utc::now().timestamp_millis() as u64)]
    pub timestamp: u64,
    /// EIP-712 salt. Fixed at build time so resubmitting an order produces a byte-identical
    /// signed body, which the CLOB can recognise as a duplicate instead of accepting it as a
    /// second order.
    #[builder(default = new_order_salt())]
    pub salt: String,
    #[builder(default = [0u8; 32])]
    pub metadata: [u8; 32],
    #[builder(default = [0u8; 32])]
    pub builder_bytes: [u8; 32],
}

impl Order {
    ///
    /// # Errors
    ///
    /// If the order fields cannot be serialized into the query body.
    pub fn build_order_query_body(
        &self,
        salt: &str,
        api_key: &str,
        pk: &str,
    ) -> Result<String> {
        let signature = build_l1_signature(self, salt, pk)?;

        log::debug!("Signature added to msg: {}", signature);

        let signed_order = SignedOrderRequest {
            salt,
            maker: &self.maker,
            signer: &self.signer,
            token_id: &self.token_id,
            maker_amount: self.maker_amount,
            taker_amount: self.taker_amount,
            side: self.side.to_string(),
            signature_type: self.signature_type,
            timestamp: self.timestamp,
            metadata: format!("0x{}", alloy::hex::encode(self.metadata)),
            builder: format!("0x{}", alloy::hex::encode(self.builder_bytes)),
            expiration: self.expiration,
            signature,
        };

        let body = OrderQueryBody {
            order: signed_order,
            owner: api_key,
            order_type: self.order_type.to_string(),
        };

        serde_json::to_string(&body).map_err(|e| {
            SerializationError::JsonSerialize {
                message: e.to_string(),
            }
            .into()
        })
    }

    ///
    /// # Errors
    ///
    /// A [`ValidationError`] naming the first field that fails its bounds check.
    pub fn validate_order(&self) -> Result<()> {
        //for buy orders, token quantity should be >= MIN_POLY_TOKEN_QUANTITY and USD amount should be >= 1.0
        if self.side == Side::Buy
            && (self.maker_amount <= RAW_UNIT_MULTIPLIER
                || self.taker_amount < MIN_POLY_TOKEN_QUANTITY * RAW_UNIT_MULTIPLIER)
        {
            return Err(ValidationError::InvalidAmount {
                reason: format!(
                "Buy order validation warning: Token quantity should be >= {MIN_POLY_TOKEN_QUANTITY} and USD amount should be >= 1.0. Current: maker_amount={}, taker_amount={}",
                self.maker_amount as f64 / RAW_UNIT_MULTIPLIER as f64,
                self.taker_amount as f64 / RAW_UNIT_MULTIPLIER as f64
            )
            }.into());
        }

        Ok(())
    }
}

impl EIP712Struct for Order {
    fn get_domain_type_hash(&self) -> B256 {
        keccak256(
            "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        )
    }

    fn get_domain_values(&self) -> DynSolValue {
        let verifying_contract = if self.neg_risk {
            NEG_RISK_VERIFYING_CONTRACT
        } else {
            NON_NEG_RISK_VERIFYING_CONTRACT
        };

        DynSolValue::Tuple(vec![
            DynSolValue::String(NAME.to_string()),
            DynSolValue::String(VERSION.to_string()),
            DynSolValue::Uint(U256::from(CHAIN_ID), 256),
            DynSolValue::Address(verifying_contract),
        ])
    }

    fn get_message_type_hash(&self) -> B256 {
        keccak256("Order(uint256 salt,address maker,address signer,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint8 side,uint8 signatureType,uint256 timestamp,bytes32 metadata,bytes32 builder)")
    }

    /// Returns the EIP-712 message values for the V2 Order struct.
    ///
    /// Field order matches the V2 type string exactly (order is significant for hashing):
    /// `Order(uint256 salt, address maker, address signer, uint256 tokenId,
    /// uint256 makerAmount, uint256 takerAmount, uint8 side, uint8 signatureType,
    /// uint256 timestamp, bytes32 metadata, bytes32 builder)`
    ///
    /// # Example
    ///
    /// ```
    /// use poly_clob_rs::{Order, Side, OrderType, EIP712Struct};
    ///
    /// let order = Order::builder()
    ///     .maker("0x1234567890123456789012345678901234567890")
    ///     .signer("0x1234567890123456789012345678901234567890")
    ///     .token_id("12345")
    ///     .maker_amount(100u64)
    ///     .taker_amount(50u64)
    ///     .side(Side::Buy)
    ///     .order_type(OrderType::GTC)
    ///     .timestamp(1712700000000u64)
    ///     .build();
    ///
    /// let values = order.get_message_values("123456789").unwrap();
    /// let fields = values.as_tuple().unwrap();
    /// // 11 fields: salt maker signer tokenId makerAmount takerAmount
    /// //            side signatureType timestamp metadata builder
    /// assert_eq!(fields.len(), 11);
    /// ```
    fn get_message_values(&self, salt: &str) -> Result<DynSolValue> {
        let salt_u256 = U256::from_str(salt).map_err(|e| SerializationError::FieldParse {
            field: "salt".to_string(),
            message: format!("invalid salt: {}: {}", salt, e),
        })?;
        let maker_addr =
            Address::from_str(&self.maker).map_err(|e| SerializationError::FieldParse {
                field: "maker".to_string(),
                message: format!("invalid maker address: {}: {}", self.maker, e),
            })?;
        let signer_addr =
            Address::from_str(&self.signer).map_err(|e| SerializationError::FieldParse {
                field: "signer".to_string(),
                message: format!("invalid signer address: {}: {}", self.signer, e),
            })?;
        let token_id_u256 =
            U256::from_str(&self.token_id).map_err(|e| SerializationError::FieldParse {
                field: "token_id".to_string(),
                message: format!("invalid token_id: {}: {}", self.token_id, e),
            })?;

        Ok(DynSolValue::Tuple(vec![
            DynSolValue::Uint(salt_u256, 256),
            DynSolValue::Address(maker_addr),
            DynSolValue::Address(signer_addr),
            DynSolValue::Uint(token_id_u256, 256),
            DynSolValue::Uint(U256::from(self.maker_amount), 256),
            DynSolValue::Uint(U256::from(self.taker_amount), 256),
            DynSolValue::Uint(U256::from(self.side.to_int()), 8),
            DynSolValue::Uint(U256::from(self.signature_type), 8),
            DynSolValue::Uint(U256::from(self.timestamp), 256),
            DynSolValue::FixedBytes(FixedBytes::from(self.metadata), 32),
            DynSolValue::FixedBytes(FixedBytes::from(self.builder_bytes), 32),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test values from poly_order_test
    const TEST_TOKEN_ID: &str =
        "9791340778034406846471990250402404386251253109836550662455900621767083631393";
    const TEST_MAKER: &str = "0x1234567890123456789012345678901234567890"; // Mock address
    const TEST_PK: &str = "0x1234567890123456789012345678901234567890123456789012345678901234";
    const TEST_TIMESTAMP: u64 = 1712700000000u64;

    /// Helper macro to create a test order with common fields
    macro_rules! test_order {
        ($side:expr_2021, $order_type:expr_2021) => {
            Order::builder()
                .maker(TEST_MAKER)
                .signer(TEST_MAKER)
                .token_id(TEST_TOKEN_ID)
                .maker_amount(100u64)
                .taker_amount(50u64)
                .side($side)
                .order_type($order_type)
                .timestamp(TEST_TIMESTAMP)
                .build()
        };
    }

    #[test]
    fn test_order_builder_with_all_fields() {
        let order = Order::builder()
            .maker(TEST_MAKER)
            .signer(TEST_MAKER)
            .token_id(TEST_TOKEN_ID)
            .maker_amount(100u64)
            .taker_amount(50u64)
            .expiration(9999999999_i64)
            .side(Side::Buy)
            .order_type(OrderType::FOK)
            .timestamp(TEST_TIMESTAMP)
            .build();

        assert_eq!(order.maker, TEST_MAKER);
        assert_eq!(order.signer, TEST_MAKER);
        assert_eq!(order.token_id, TEST_TOKEN_ID);
        assert_eq!(order.maker_amount, 100);
        assert_eq!(order.taker_amount, 50);
        assert_eq!(order.expiration, 9999999999);
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.signature_type, 1); // default
        assert_eq!(order.order_type, OrderType::FOK);
        assert_eq!(order.timestamp, TEST_TIMESTAMP);
        assert_eq!(order.metadata, [0u8; 32]);
        assert_eq!(order.builder_bytes, [0u8; 32]);
    }

    #[test]
    fn test_order_builder_defaults() {
        let order = test_order!(Side::Buy, OrderType::GTC);

        // Verify defaults
        assert_eq!(order.expiration, 0);
        assert!(!order.neg_risk);
        assert_eq!(order.signature_type, 1);
        assert_eq!(order.metadata, [0u8; 32]);
        assert_eq!(order.builder_bytes, [0u8; 32]);
    }

    #[test]
    fn test_order_with_different_order_types() {
        for order_type in [
            OrderType::FOK,
            OrderType::FAK,
            OrderType::GTC,
            OrderType::GTD,
        ] {
            let order = test_order!(Side::Buy, order_type);
            assert_eq!(order.order_type, order_type);
        }
    }

    #[test]
    fn test_order_with_buy_side() {
        let order = test_order!(Side::Buy, OrderType::GTC);
        assert_eq!(order.side, Side::Buy);
    }

    #[test]
    fn test_order_with_sell_side() {
        let order = test_order!(Side::Sell, OrderType::GTC);
        assert_eq!(order.side, Side::Sell);
    }

    #[test]
    fn test_build_order_query_body_structure() {
        // Known-answer test: fixed inputs must produce a deterministic V2 body.
        // Regenerate via Python py-clob-client-v2 with the same pk/salt/fields to verify cross-client parity.
        const EXPECTED: &str = r#"{"order":{"salt":123456789,"maker":"0x1234567890123456789012345678901234567890","signer":"0x1234567890123456789012345678901234567890","tokenId":"9791340778034406846471990250402404386251253109836550662455900621767083631393","makerAmount":"100","takerAmount":"50","side":"BUY","signatureType":1,"timestamp":"1712700000000","metadata":"0x0000000000000000000000000000000000000000000000000000000000000000","builder":"0x0000000000000000000000000000000000000000000000000000000000000000","expiration":"9999999999","signature":"0xd4506d0e92ca2a9c9cef22f00617bb1d277437a4e82963b03dcdfa72fbd503e17279459903a67903647792dfb282453f35fc5e679f3e9abe7aa6feb9bed457971b"},"owner":"test_api_key","orderType":"FOK"}"#;

        let order = Order::builder()
            .maker(TEST_MAKER)
            .signer(TEST_MAKER)
            .token_id(TEST_TOKEN_ID)
            .maker_amount(100u64)
            .taker_amount(50u64)
            .expiration(9999999999_i64)
            .side(Side::Buy)
            .order_type(OrderType::FOK)
            .timestamp(TEST_TIMESTAMP)
            .build();

        let body = order
            .build_order_query_body("123456789", "test_api_key", TEST_PK)
            .unwrap();

        assert_eq!(body, EXPECTED);
    }

    #[test]
    fn test_build_order_query_body_order_type_fok() {
        let order = test_order!(Side::Buy, OrderType::FOK);
        let body = order
            .build_order_query_body("123", "key", TEST_PK)
            .unwrap();
        assert!(body.contains("\"orderType\":\"FOK\""));
    }

    #[test]
    fn test_build_order_query_body_order_type_fak() {
        let order = test_order!(Side::Buy, OrderType::FAK);
        let body = order
            .build_order_query_body("123", "key", TEST_PK)
            .unwrap();
        assert!(body.contains("\"orderType\":\"FAK\""));
    }

    #[test]
    fn test_build_order_query_body_order_type_gtc() {
        let order = test_order!(Side::Buy, OrderType::GTC);
        let body = order
            .build_order_query_body("123", "key", TEST_PK)
            .unwrap();
        assert!(body.contains("\"orderType\":\"GTC\""));
    }

    #[test]
    fn test_build_order_query_body_order_type_gtd() {
        let order = test_order!(Side::Buy, OrderType::GTD);
        let body = order
            .build_order_query_body("123", "key", TEST_PK)
            .unwrap();
        assert!(body.contains("\"orderType\":\"GTD\""));
    }

    #[test]
    fn test_build_order_query_body_buy_side() {
        let order = test_order!(Side::Buy, OrderType::GTC);
        let body = order
            .build_order_query_body("123", "key", TEST_PK)
            .unwrap();
        assert!(body.contains("\"side\":\"BUY\""));
    }

    #[test]
    fn test_build_order_query_body_sell_side() {
        let order = test_order!(Side::Sell, OrderType::GTC);
        let body = order
            .build_order_query_body("123", "key", TEST_PK)
            .unwrap();
        assert!(body.contains("\"side\":\"SELL\""));
    }

    #[test]
    fn test_build_order_query_body_contains_signature() {
        let order = test_order!(Side::Buy, OrderType::FOK);
        let body = order
            .build_order_query_body("123", "key", TEST_PK)
            .unwrap();
        assert!(body.contains("\"signature\":\"0x"));
    }

    #[test]
    fn test_build_order_query_body_includes_api_key() {
        let order = test_order!(Side::Buy, OrderType::FOK);
        let api_key = "my_test_api_key_12345";
        let body = order
            .build_order_query_body("123", api_key, TEST_PK)
            .unwrap();
        assert!(body.contains(&format!("\"owner\":\"{}\"", api_key)));
    }

    #[test]
    fn test_build_order_query_body_includes_salt() {
        let order = test_order!(Side::Buy, OrderType::FOK);
        let salt = "987654321";
        let body = order
            .build_order_query_body(salt, "key", TEST_PK)
            .unwrap();
        assert!(body.contains(&format!("\"salt\":{}", salt)));
    }

    #[test]
    fn test_order_neg_risk() {
        let order = Order::builder()
            .maker(TEST_MAKER)
            .signer(TEST_MAKER)
            .token_id(TEST_TOKEN_ID)
            .maker_amount(100u64)
            .taker_amount(50u64)
            .side(Side::Buy)
            .order_type(OrderType::GTC)
            .neg_risk(true)
            .build();

        assert!(order.neg_risk);
    }
}
