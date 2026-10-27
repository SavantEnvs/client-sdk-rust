#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use serde::{de::DeserializeOwned, Serialize};

// First input byte selects the message type, the rest is the protobuf-JSON (pbjson) document.
fn json_roundtrip<M: DeserializeOwned + Serialize>(payload: &[u8]) -> Corpus {
    let Ok(msg) = serde_json::from_slice::<M>(payload) else { return Corpus::Reject };
    let text = serde_json::to_string(&msg).expect("serialising a parsed message must work");
    serde_json::from_str::<M>(&text).expect("re-parse of own JSON failed");
    Corpus::Keep
}

fuzz_target!(|data: &[u8]| -> Corpus {
    let Some((sel, payload)) = lk_fuzz::split_selector(data) else { return Corpus::Reject };
    use livekit_protocol as lp;
    match sel % 8 {
        0 => json_roundtrip::<lp::SignalRequest>(payload),
        1 => json_roundtrip::<lp::SignalResponse>(payload),
        2 => json_roundtrip::<lp::DataPacket>(payload),
        3 => json_roundtrip::<lp::JoinResponse>(payload),
        4 => json_roundtrip::<lp::ParticipantInfo>(payload),
        5 => json_roundtrip::<lp::TrackInfo>(payload),
        6 => json_roundtrip::<lp::RoomConfiguration>(payload),
        _ => json_roundtrip::<lp::Room>(payload),
    }
});
