use alloy::{dyn_abi::DynSolValue, primitives::B256};
use anyhow::Result;

pub trait EIP712Struct {
    fn get_domain_type_hash(&self) -> B256;
    fn get_domain_values(&self) -> DynSolValue;
    fn get_message_type_hash(&self) -> B256;
    fn get_message_values(&self, salt: &str, nonce: i32) -> Result<DynSolValue>;
}
