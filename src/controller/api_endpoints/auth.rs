//"/auth/api-key"

use crate::model::Signer;

pub static MSG_TO_SIGN: &str = "This message attests that I control the given wallet";

pub static CLOB_DOMAIN_NAME: &str = "ClobAuthDomain";
pub static CLOB_VERSION: &str = "1";
pub static POLYGON_CHAIN_ID: i32 = 137;

pub fn create_level_1_headers(signer: Signer, nonce: i32) {
    /*
        n = 0

        signature = sign_clob_auth_message(signer, timestamp, n)
        headers = {
            POLY_ADDRESS: signer.address(),
            POLY_SIGNATURE: signature,
            POLY_TIMESTAMP: str(timestamp),
            POLY_NONCE: str(n),
        }
    */
}
