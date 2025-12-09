use alloy::{
    dyn_abi::DynSolValue,
    hex,
    primitives::{keccak256, Address, B256, U256},
    signers::{local::PrivateKeySigner, Signer as AlloySigner},
};
use anyhow::{bail, Context, Result};
use futures::executor::block_on;
use std::str::FromStr;

use crate::Account;

use super::EIP712Struct;
use base64::engine::general_purpose::URL_SAFE;
use base64::prelude::*;
use chrono::prelude::*;
use hmac::Mac;
use reqwest::header::HeaderMap;

fn generate_values_hash(value: &DynSolValue) -> Result<Vec<u8>> {
    let mut encoded_values: Vec<u8> = Vec::new();

    let tup = value.as_tuple().context("expected tuple value")?;
    for val in tup {
        log::trace!("Value: {:?}", val);

        let typ = val.as_type().context("failed to get type from value")?;

        log::trace!("Type: {}", typ.to_string());
        match typ.to_string().as_str() {
            "string" => {
                let str = val.as_str().context("expected string value")?;
                let encoded_str = keccak256(str);
                log::trace!("Result: {encoded_str}");
                encoded_values.extend_from_slice(encoded_str.as_slice());
            }
            "uint8" => {
                let uint8 = val.as_uint().context("expected uint8 value")?;
                let x: [u8; 32] = uint8.0.to_be_bytes();
                let encoded_uint8: [u8; 32] = U256::from_be_slice(&x).to_be_bytes();
                log::trace!("Result: {:?}", &encoded_uint8);
                encoded_values.extend_from_slice(&encoded_uint8);
            }
            "uint256" => {
                let uint256 = val.as_uint().context("expected uint256 value")?;
                let x: [u8; 32] = uint256.0.to_be_bytes();
                log::trace!("Result: {:?}", x);
                encoded_values.extend_from_slice(&x);
            }
            "address" => {
                let address: Address = val.as_address().context("expected address value")?;
                let address_slice = address.as_slice();

                let encoded_address: [u8; 32] = U256::from_be_slice(address_slice).to_be_bytes();

                log::trace!("Result: {:?}", encoded_address);
                encoded_values.extend_from_slice(&encoded_address);
            }
            other => bail!("unknown EIP712 type: {other}"),
        }
    }

    Ok(encoded_values)
}

fn get_encoded_domain(eip712_struct: &dyn EIP712Struct) -> Result<B256> {
    let domain_type_hash = eip712_struct.get_domain_type_hash();

    let encoded_domain_values = generate_values_hash(&eip712_struct.get_domain_values())?;

    let encoded_domain_full_bytes = [&domain_type_hash[..], &encoded_domain_values[..]].concat();

    Ok(keccak256(encoded_domain_full_bytes))
}

pub fn build_l1_signature(
    eip712_struct: &dyn EIP712Struct,
    salt: &str,
    nonce: i32,
    signer_pk: &str,
) -> Result<String> {
    let encoded_domain = get_encoded_domain(eip712_struct)?;

    let message_value = eip712_struct.get_message_values(salt, nonce)?;
    let eip712_message_type_hash = eip712_struct.get_message_type_hash();
    let encoded_message_values = generate_values_hash(&message_value)?;

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

    log::trace!("Signable bytes: {:?}", &signable_bytes);
    log::trace!("Message hash: {:?}", &eip712_hash);

    let wallet = PrivateKeySigner::from_str(signer_pk)
        .context("invalid private key")?;
    log::trace!("\nSigner address: {}", wallet.address());

    let signature = block_on(wallet.sign_hash(&eip712_hash))
        .context("failed to sign EIP712 hash")?;
    log::trace!("Signature: 0x{}", hex::encode(signature.as_bytes()));

    let mut result = "0x".to_string();
    result.push_str(hex::encode(signature.as_bytes()).as_str());

    Ok(result)
}

pub fn build_l2_headers(
    signer: &Account,
    method: &str,
    request_path: &str,
    body: &str,
    salt: &str,
) -> Result<HeaderMap> {
    let poly_address = &signer.pub_key;
    let api_key = &signer.api_key;
    let api_secret = &signer.api_secret;
    let api_passphrase = &signer.api_passphrase;

    let mut headers = HeaderMap::new();

    headers.append(
        "POLY_ADDRESS",
        poly_address.parse().context("invalid POLY_ADDRESS header")?,
    );
    headers.append(
        "POLY_API_KEY",
        api_key.parse().context("invalid POLY_API_KEY header")?,
    );
    headers.append(
        "POLY_PASSPHRASE",
        api_passphrase.parse().context("invalid POLY_PASSPHRASE header")?,
    );

    let timestamp = if salt.is_empty() {
        get_timestamp()
    } else {
        salt.to_string()
    };

    headers.append(
        "POLY_TIMESTAMP",
        timestamp.parse().context("invalid POLY_TIMESTAMP header")?,
    );
    let signature = build_hmac_signature(api_secret, &timestamp, method, request_path, body)?;
    headers.append(
        "POLY_SIGNATURE",
        signature.parse().context("invalid POLY_SIGNATURE header")?,
    );

    Ok(headers)
}

pub fn build_hmac_signature(
    api_secret: &str,
    timestamp: &str,
    method: &str,
    request_path: &str,
    request_body: &str,
) -> Result<String> {
    let message = timestamp.to_string() + method + request_path + request_body;

    let b64_decoded_secret = URL_SAFE
        .decode(api_secret)
        .context("failed to decode API secret from base64")?;
    let b64_decoded_secret_slice = b64_decoded_secret.as_slice();

    type HmacSha256 = hmac::Hmac<sha2::Sha256>;
    let mut mac = HmacSha256::new_from_slice(b64_decoded_secret_slice)
        .context("failed to create HMAC")?;
    mac.update(message.as_bytes());

    let bytes = mac.finalize().into_bytes();

    let mut signature: String = Default::default();
    URL_SAFE.encode_string(bytes, &mut signature);

    Ok(signature)
}

pub fn get_zero_address() -> String {
    "0x0000000000000000000000000000000000000000".to_string()
}

pub fn get_timestamp() -> String {
    let now = Utc::now();
    now.timestamp().to_string()
}
