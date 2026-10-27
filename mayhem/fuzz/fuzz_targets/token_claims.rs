#![no_main]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use libfuzzer_sys::{fuzz_target, Corpus};
use livekit_token::{Claims, TokenVerifier};
use sha2::Sha256;

const API_KEY: &str = "myapikey";
const API_SECRET: &str = "thiskeyistotallyunsafe";

// Selector 0: the rest is a raw JWT string. Selector 1: the rest is a JSON claims payload that is
// wrapped in a correctly HS256-signed JWT, so the verified-claims decoding path is reachable
// (the fuzzer cannot forge an HMAC on its own).
fuzz_target!(|data: &[u8]| -> Corpus {
    let Some((sel, rest)) = lk_fuzz::split_selector(data) else { return Corpus::Reject };
    let token = match sel & 1 {
        0 => match std::str::from_utf8(rest) {
            Ok(s) => s.to_owned(),
            Err(_) => return Corpus::Reject,
        },
        _ => {
            let head = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
            let body = URL_SAFE_NO_PAD.encode(rest);
            let signing_input = format!("{head}.{body}");
            let mut mac = Hmac::<Sha256>::new_from_slice(API_SECRET.as_bytes()).unwrap();
            mac.update(signing_input.as_bytes());
            let sig = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
            format!("{signing_input}.{sig}")
        }
    };
    let unverified = Claims::from_unverified(&token);
    let verified = TokenVerifier::with_api_key(API_KEY, API_SECRET).verify(&token);
    if let (Ok(a), Ok(b)) = (&unverified, &verified) {
        // A token that verifies must decode to the same claims without verification.
        assert_eq!(a.sub, b.sub);
        assert_eq!(a.iss, b.iss);
    }
    if unverified.is_ok() || verified.is_ok() {
        Corpus::Keep
    } else {
        Corpus::Reject
    }
});
