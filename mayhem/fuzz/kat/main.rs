// Known-answer probe: dynamically linked, built with the project's normal flags (no sanitizer).
// Prints one `KAT <name> ok|FAIL` line per check and a final `KAT-SUMMARY passed=N failed=M`.
use std::time::Duration;

use bytes::Bytes;
use livekit_common::{EncryptionType, ParticipantIdentity};
use livekit_data_stream::api::{AnyStreamReader, StreamReader};
use livekit_data_stream::backend::incoming::{InputEvent, Manager, OutputEvent, PacketReceived};
use livekit_data_stream::backend::{Chunk, Header, Packet, Trailer};
use livekit_datatrack::__fuzz::Packet as TrackPacket;
use livekit_protocol as lp;
use livekit_token::{AccessToken, TokenVerifier, VideoGrants};
use livekit_token_source::TokenSourceResponse;
use prost::Message;

const API_KEY: &str = "myapikey";
const API_SECRET: &str = "thiskeyistotallyunsafe";
const TEST_TOKEN: &str = include_str!("../../../livekit-token/src/test_token.txt");

struct Kat {
    passed: u32,
    failed: u32,
}

impl Kat {
    fn check(&mut self, name: &str, ok: bool) {
        if ok {
            self.passed += 1;
            println!("KAT {name} ok");
        } else {
            self.failed += 1;
            println!("KAT {name} FAIL");
        }
    }
}

fn data_stream_text(kat: &mut Kat) {
    let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let got = rt.block_on(async {
        let (manager, input, mut output) = Manager::new(None);
        let who = ParticipantIdentity::from("alice");
        let header = lp::data_stream::Header {
            stream_id: "s1".into(),
            topic: "chat".into(),
            mime_type: "text/plain".into(),
            content_header: Some(lp::data_stream::header::ContentHeader::TextHeader(
                lp::data_stream::TextHeader::default(),
            )),
            ..Default::default()
        };
        let chunk = |i: u64, s: &str| lp::data_stream::Chunk {
            stream_id: "s1".into(),
            chunk_index: i,
            content: s.as_bytes().to_vec(),
            ..Default::default()
        };
        let trailer = lp::data_stream::Trailer { stream_id: "s1".into(), ..Default::default() };
        let enc = EncryptionType::None;
        let pk = |p| InputEvent::PacketReceived(PacketReceived::new(p, who.clone()));
        input.send(pk(Packet::Header { header: Header::from(header), encryption_type: enc })).unwrap();
        input.send(pk(Packet::Chunk { chunk: Chunk::from(chunk(0, "hel")), encryption_type: enc })).unwrap();
        input.send(pk(Packet::Chunk { chunk: Chunk::from(chunk(1, "lo")), encryption_type: enc })).unwrap();
        input.send(pk(Packet::Trailer { trailer: Trailer::from(trailer), encryption_type: enc })).unwrap();
        input.send(InputEvent::Shutdown).unwrap();
        manager.run().await;
        let mut text = None;
        while let Ok(ev) = output.try_recv() {
            if let OutputEvent::StreamOpened(opened) = ev {
                if let AnyStreamReader::Text(r) = opened.stream_reader {
                    text = r.read_all().await.ok();
                }
            }
        }
        text
    });
    kat.check("data_stream_text_reassembly_hello", got.as_deref() == Some("hello"));
}

fn main() {
    let mut kat = Kat { passed: 0, failed: 0 };

    // JWT: the checked-in test token verifies under its key and carries the expected claims.
    let claims = TokenVerifier::with_api_key(API_KEY, API_SECRET).verify(TEST_TOKEN.trim());
    kat.check("token_verify_test_token", claims.is_ok());
    if let Ok(c) = &claims {
        kat.check("token_sub_identity", c.sub == "identity");
        kat.check("token_name", c.name == "name");
        kat.check("token_room", c.video.room == "my-room");
    }
    kat.check(
        "token_wrong_secret_rejected",
        TokenVerifier::with_api_key(API_KEY, "wrong").verify(TEST_TOKEN.trim()).is_err(),
    );
    let jwt = AccessToken::with_api_key(API_KEY, API_SECRET)
        .with_ttl(Duration::from_secs(600))
        .with_identity("carol")
        .with_grants(VideoGrants { room_join: true, room: "r1".into(), ..Default::default() })
        .to_jwt()
        .unwrap();
    let back = TokenVerifier::with_api_key(API_KEY, API_SECRET).verify(&jwt).unwrap();
    kat.check("token_roundtrip_sub_room", back.sub == "carol" && back.video.room == "r1");

    // Token source response: JSON body parse + freshness of a garbage token.
    let resp: TokenSourceResponse =
        serde_json::from_str(r#"{"server_url":"wss://x.example","participant_token":"a.b.c"}"#).unwrap();
    kat.check("token_source_parse", resp.server_url == "wss://x.example");
    kat.check("token_source_garbage_token_invalid", !resp.has_valid_token());

    // Protobuf wire: Room{sid:"RM_1", name:"r"} <-> bytes.
    let wire = [0x0a, 0x04, b'R', b'M', b'_', b'1', 0x12, 0x01, b'r'];
    let room = lp::Room::decode(&wire[..]).unwrap();
    kat.check("protocol_decode_room", room.sid == "RM_1" && room.name == "r");
    let enc = lp::Room { sid: "RM_1".into(), name: "r".into(), ..Default::default() }.encode_to_vec();
    kat.check("protocol_encode_room", enc == wire);
    kat.check("protocol_json_room", serde_json::to_string(&room).unwrap().contains("\"sid\":\"RM_1\""));

    // Data track packet: parse known bytes, re-serialise to identical bytes.
    let dt_wire: &[u8] = &DATATRACK_WIRE;
    let pkt = TrackPacket::deserialize(Bytes::copy_from_slice(dt_wire)).unwrap();
    kat.check(
        "datatrack_header_fields",
        pkt.header.sequence == 0x0102
            && pkt.header.frame_number == 0x0304
            && pkt.header.marker == livekit_datatrack::__fuzz::FrameMarker::Single,
    );
    kat.check("datatrack_payload", &pkt.payload[..] == DATATRACK_PAYLOAD);
    kat.check("datatrack_reserialize_identical", &pkt.serialize()[..] == dt_wire);

    data_stream_text(&mut kat);

    println!("KAT-SUMMARY passed={} failed={}", kat.passed, kat.failed);
    std::process::exit(if kat.failed == 0 { 0 } else { 1 });
}

const DATATRACK_PAYLOAD: &[u8] = b"hi";
const DATATRACK_WIRE: [u8; 14] =
    [0x18, 0x00, 0x00, 0x07, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, b'h', b'i'];
