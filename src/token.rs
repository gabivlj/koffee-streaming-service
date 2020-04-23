use chrono::offset::Utc;
use chrono::Duration;
#[cfg(feature = "v2")]
use ring::signature::Ed25519KeyPair;
use serde_json::json;
use std::io::prelude::*;
use std::num::ParseIntError;

pub struct Token;

fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

impl Token {
    // #[warn(dead_code)]
    pub fn validate_token(s: &str) -> Option<u64> {
        // Get the paseto public key (must be the same as the one that the Authorization server uses)
        let verified = paseto::validate_public_token(&s, None, get_public_key());
        let val = match verified {
            Ok(value) => value,
            Err(_) => {
                return None;
            }
        };
        return match val.get("id") {
            // I really trust where this token is comming from.
            Some(value) => Some(value.as_str().unwrap().parse().unwrap()),
            _ => None,
        };
    }

    pub fn generate_token(path: &str, duration: f64) -> Result<String, &str> {
        /*
             FOR FUTURE REFERENCE IF WE WANNA CREATE A NEW KEY
              let rng = ring::rand::SystemRandom::new();
              let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkc8(&rng).unwrap();
              let pkcs8_bytes_more: &[u8] = pkcs8_bytes.as_ref();
        */

        let as_key = ring::signature::Ed25519KeyPair::from_pkcs8(get_key().as_ref()).unwrap();
        let exp = Utc::now() + Duration::days(1);
        match paseto::tokens::PasetoBuilder::new()
            .set_ed25519_key(as_key)
            .set_issued_at(Some(Utc::now()))
            .set_expiration(exp)
            .set_claim(String::from("path"), json!(path))
            .set_claim(String::from("duration"), json!(duration.to_string()))
            .set_footer(String::from("xd"))
            .build()
        {
            Ok(value) => Ok(value),
            Err(_) => Err("Error generating token"),
        }
    }
}

pub fn init(path: &str) {
    let mut f = std::fs::File::open(path).unwrap();
    let mut buff: Vec<u8> = vec![];
    f.read_to_end(&mut buff).unwrap();
    // let as_key = ring::signature::Ed25519KeyPair::from_pkcs8(buff.as_ref()).unwrap();
    let as_key = dotenv::var("PASETO_PUBLIC_KEY").expect("Failed to parse keypair");
    let val = decode_hex(as_key.as_str()).unwrap();
    unsafe {
        KEY = buff;
        PUBLIC_KEY = paseto::tokens::PasetoPublicKey::ED25519PublicKey(val);
    }
}

/**
 * This is static unsafe stuff of the application and shouldn't be changed from anywhere.
 *
 * Use the get methods to get the references
 */

static mut KEY: Vec<u8> = Vec::new();
static mut PUBLIC_KEY: paseto::PasetoPublicKey =
    paseto::tokens::PasetoPublicKey::ED25519PublicKey(Vec::new());

fn get_key() -> &'static Vec<u8> {
    let k = unsafe { &KEY };
    k
}

fn get_public_key() -> &'static paseto::PasetoPublicKey {
    let p_key = unsafe { &PUBLIC_KEY };
    p_key
}
