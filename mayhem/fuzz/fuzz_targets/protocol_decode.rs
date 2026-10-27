#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use prost::Message;

// First input byte selects the message type, the rest is the protobuf wire payload.
fn decode_encode<M: Message + Default>(payload: &[u8]) -> Corpus {
    let Ok(msg) = M::decode(payload) else { return Corpus::Reject };
    let encoded = msg.encode_to_vec();
    assert_eq!(encoded.len(), msg.encoded_len());
    // Anything we serialised must be accepted again.
    M::decode(encoded.as_slice()).expect("re-decode of own encoding failed");
    Corpus::Keep
}

fuzz_target!(|data: &[u8]| -> Corpus {
    let Some((sel, payload)) = lk_fuzz::split_selector(data) else { return Corpus::Reject };
    use livekit_protocol as lp;
    match sel % 14 {
        0 => decode_encode::<lp::SignalRequest>(payload),
        1 => decode_encode::<lp::SignalResponse>(payload),
        2 => decode_encode::<lp::DataPacket>(payload),
        3 => decode_encode::<lp::JoinResponse>(payload),
        4 => decode_encode::<lp::ParticipantInfo>(payload),
        5 => decode_encode::<lp::TrackInfo>(payload),
        6 => decode_encode::<lp::Room>(payload),
        7 => decode_encode::<lp::RoomConfiguration>(payload),
        8 => decode_encode::<lp::SessionDescription>(payload),
        9 => decode_encode::<lp::TrickleRequest>(payload),
        10 => decode_encode::<lp::MetricsBatch>(payload),
        11 => decode_encode::<lp::RpcRequest>(payload),
        12 => decode_encode::<lp::agent::AgentSessionEvent>(payload),
        _ => decode_encode::<lp::agent::ChatContext>(payload),
    }
});
