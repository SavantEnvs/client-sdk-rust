#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use livekit_token_source::TokenSourceResponse;
use std::time::Duration;

// Token-source endpoint JSON body; if it parses, the embedded (unverified) JWT is inspected for
// freshness exactly as the SDK's token cache does.
fuzz_target!(|data: &[u8]| -> Corpus {
    let Ok(resp) = serde_json::from_slice::<TokenSourceResponse>(data) else {
        return Corpus::Reject;
    };
    let _ = resp.has_valid_token();
    let _ = resp.has_valid_token_with_tolerance(Duration::from_secs(u32::MAX as u64));
    let _ = resp.has_valid_token_with_tolerance(Duration::ZERO);
    Corpus::Keep
});
