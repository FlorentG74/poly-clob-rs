use alloy::{
    dyn_abi::DynSolValue,
    hex,
    primitives::{keccak256, Address, B256, U256},
    signers::{local::PrivateKeySigner, SignerSync},
};
use std::str::FromStr;

use crate::api::error::{AuthError, Result};
use crate::Account;

use super::EIP712Struct;
use base64::engine::general_purpose::URL_SAFE;
use base64::prelude::*;
use chrono::prelude::*;
use hmac::{KeyInit, Mac};
use reqwest::header::HeaderMap;

fn generate_values_hash(value: &DynSolValue) -> Result<Vec<u8>> {
    let mut encoded_values: Vec<u8> = Vec::new();

    let tup = value
        .as_tuple()
        .ok_or_else(|| AuthError::SignatureFailed {
            message: "expected tuple value".to_string(),
        })?;

    for val in tup {
        log::trace!("Value: {:?}", val);

        let typ = val.as_type().ok_or_else(|| AuthError::SignatureFailed {
            message: "failed to get type from value".to_string(),
        })?;

        log::trace!("Type: {}", typ.to_string());
        match typ.to_string().as_str() {
            "string" => {
                let str = val.as_str().ok_or_else(|| AuthError::SignatureFailed {
                    message: "expected string value".to_string(),
                })?;
                let encoded_str = keccak256(str);
                log::trace!("Result: {encoded_str}");
                encoded_values.extend_from_slice(encoded_str.as_slice());
            }
            "uint8" => {
                let uint8 = val.as_uint().ok_or_else(|| AuthError::SignatureFailed {
                    message: "expected uint8 value".to_string(),
                })?;
                let x: [u8; 32] = uint8.0.to_be_bytes();
                let encoded_uint8: [u8; 32] = U256::from_be_slice(&x).to_be_bytes();
                log::trace!("Result: {:?}", encoded_uint8);
                encoded_values.extend_from_slice(&encoded_uint8);
            }
            "uint256" => {
                let uint256 = val.as_uint().ok_or_else(|| AuthError::SignatureFailed {
                    message: "expected uint256 value".to_string(),
                })?;
                let x: [u8; 32] = uint256.0.to_be_bytes();
                log::trace!("Result: {:?}", x);
                encoded_values.extend_from_slice(&x);
            }
            "address" => {
                let address: Address =
                    val.as_address()
                        .ok_or_else(|| AuthError::SignatureFailed {
                            message: "expected address value".to_string(),
                        })?;
                let address_slice = address.as_slice();

                let encoded_address: [u8; 32] = U256::from_be_slice(address_slice).to_be_bytes();

                log::trace!("Result: {:?}", encoded_address);
                encoded_values.extend_from_slice(&encoded_address);
            }
            "bytes32" => {
                let (bytes, _size) = val.as_fixed_bytes().ok_or_else(|| AuthError::SignatureFailed {
                    message: "expected bytes32 value".to_string(),
                })?;
                encoded_values.extend_from_slice(bytes);
            }
            other => {
                return Err(AuthError::SignatureFailed {
                    message: format!("unknown EIP712 type: {}", other),
                }
                .into())
            }
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
    signer_pk: &str,
) -> Result<String> {
    let encoded_domain = get_encoded_domain(eip712_struct)?;

    let message_value = eip712_struct.get_message_values(salt)?;
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

    log::trace!("Signable bytes: {:?}", signable_bytes);
    log::trace!("Message hash: {:?}", eip712_hash);

    let wallet = PrivateKeySigner::from_str(signer_pk).map_err(|e| AuthError::InvalidPrivateKey {
        message: e.to_string(),
    })?;
    log::trace!("\nSigner address: {}", wallet.address());

    let signature = wallet.sign_hash_sync(&eip712_hash).map_err(|e| {
        AuthError::SignatureFailed {
            message: format!("failed to sign EIP712 hash: {}", e),
        }
    })?;
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
        poly_address
            .parse()
            .map_err(|e| AuthError::HeaderBuildFailed {
                message: format!("invalid POLY_ADDRESS header: {}", e),
            })?,
    );
    headers.append(
        "POLY_API_KEY",
        api_key.parse().map_err(|e| AuthError::HeaderBuildFailed {
            message: format!("invalid POLY_API_KEY header: {}", e),
        })?,
    );
    headers.append(
        "POLY_PASSPHRASE",
        api_passphrase
            .parse()
            .map_err(|e| AuthError::HeaderBuildFailed {
                message: format!("invalid POLY_PASSPHRASE header: {}", e),
            })?,
    );

    let timestamp = if salt.is_empty() {
        get_timestamp()
    } else {
        salt.to_string()
    };

    headers.append(
        "POLY_TIMESTAMP",
        timestamp
            .parse()
            .map_err(|e| AuthError::HeaderBuildFailed {
                message: format!("invalid POLY_TIMESTAMP header: {}", e),
            })?,
    );
    let signature = build_hmac_signature(api_secret, &timestamp, method, request_path, body)?;
    headers.append(
        "POLY_SIGNATURE",
        signature
            .parse()
            .map_err(|e| AuthError::HeaderBuildFailed {
                message: format!("invalid POLY_SIGNATURE header: {}", e),
            })?,
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

    let b64_decoded_secret = URL_SAFE.decode(api_secret).map_err(|e| {
        AuthError::HeaderBuildFailed {
            message: format!("failed to decode API secret from base64: {}", e),
        }
    })?;
    let b64_decoded_secret_slice = b64_decoded_secret.as_slice();

    type HmacSha256 = hmac::Hmac<sha2::Sha256>;
    let mut mac = HmacSha256::new_from_slice(b64_decoded_secret_slice).map_err(|e| {
        AuthError::HeaderBuildFailed {
            message: format!("failed to create HMAC: {}", e),
        }
    })?;
    mac.update(message.as_bytes());

    let bytes = mac.finalize().into_bytes();

    let mut signature: String = Default::default();
    URL_SAFE.encode_string(bytes, &mut signature);

    Ok(signature)
}

pub fn get_timestamp() -> String {
    let now = Utc::now();
    now.timestamp().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::auth::l1_header::L1Header;

    // Hardhat/Anvil test key #0 — never use in production.
    const TEST_PK: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    const TEST_ADDR: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
    // base64-url encoding of the 11-byte string "test_secret"
    const TEST_SECRET: &str = "dGVzdF9zZWNyZXQ=";

    #[test]
    fn build_hmac_signature_known_answer() {
        let sig = build_hmac_signature(TEST_SECRET, "1234567890", "GET", "/order", "").unwrap();
        assert_eq!(sig, "88x8_EbYF1_5tnRn6m0trRwtOUfiyc_GlorAmkGnmw0=");
    }

    #[test]
    fn build_hmac_signature_with_body() {
        let body = r#"{"price":"0.50","size":"10"}"#;
        let sig = build_hmac_signature(TEST_SECRET, "9999999999", "POST", "/order", body).unwrap();
        assert_eq!(sig, "suII_40EB21_4TzXFbEfmxWJ1-0V5KdJmiYgB0txRT8=");
    }

    #[test]
    fn build_hmac_signature_invalid_secret_errors() {
        let result = build_hmac_signature("not-valid-base64!!!", "1234567890", "GET", "/", "");
        assert!(result.is_err());
    }

    #[test]
    fn build_l1_signature_known_answer() {
        let l1 = L1Header::new(TEST_ADDR);
        let sig = build_l1_signature(&l1, "1234567890", TEST_PK).unwrap();
        assert!(sig.starts_with("0x"), "signature must have 0x prefix");
        assert_eq!(sig.len(), 132, "signature must be 65 bytes (130 hex + 0x)");
        assert_eq!(
            sig,
            "0x62517b928ac379abcc72209fb9099da9f6154a55f7e3057d060e532d00537a3a4bebc22bc943836755d1b3d576be2bec7e20d0288fed4b1252ff6a342323d2551c"
        );
    }

    #[test]
    fn build_l1_signature_invalid_pk_errors() {
        let l1 = L1Header::new(TEST_ADDR);
        let result = build_l1_signature(&l1, "1234567890", "not_a_private_key");
        assert!(result.is_err());
    }

    #[test]
    fn build_l1_signature_invalid_address_errors() {
        let l1 = L1Header::new("not_an_address");
        let result = build_l1_signature(&l1, "1234567890", TEST_PK);
        assert!(result.is_err());
    }
}
