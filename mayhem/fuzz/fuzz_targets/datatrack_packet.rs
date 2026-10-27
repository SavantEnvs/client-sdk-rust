#![no_main]

use bytes::Bytes;
use libfuzzer_sys::{fuzz_target, Corpus};
use livekit_datatrack::__fuzz::Packet;

fuzz_target!(|data: &[u8]| -> Corpus {
    let Ok(packet) = Packet::deserialize(Bytes::copy_from_slice(data)) else {
        return Corpus::Reject;
    };
    let len = packet.header.serialized_len() + packet.payload.len();
    let out = packet.clone().serialize();
    assert_eq!(out.len(), len);
    // What we serialise must parse back to the same header and payload.
    let again = Packet::deserialize(out).expect("re-deserialize of own serialization failed");
    assert_eq!(again.header, packet.header);
    assert_eq!(again.payload, packet.payload);
    Corpus::Keep
});
