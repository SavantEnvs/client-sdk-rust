#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::{fuzz_target, Corpus};
use livekit_common::{EncryptionType, ParticipantIdentity};
use livekit_data_stream::api::{AnyStreamReader, StreamReader};
use livekit_data_stream::backend::incoming::{InputEvent, Manager, OutputEvent, PacketReceived};
use livekit_data_stream::backend::{Chunk, Header, Packet, Trailer};
use livekit_protocol::data_packet::Value;
use livekit_protocol::DataPacket;
use prost::Message;

const MAX_PACKETS: usize = 64;
const MAX_PAYLOAD_BYTES: usize = 1 << 20;

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread().build().expect("tokio runtime")
    })
}

// The input is a sequence of length-delimited `DataPacket` protobufs. Those carrying a data-stream
// header/chunk/trailer are fed, in order, to the incoming stream manager, which is then shut down;
// every stream it opened is drained to completion.
fuzz_target!(|data: &[u8]| -> Corpus {
    let mut buf = data;
    let mut packets = Vec::new();
    while !buf.is_empty() && packets.len() < MAX_PACKETS {
        let Ok(dp) = DataPacket::decode_length_delimited(&mut buf) else { break };
        let identity = ParticipantIdentity::from(dp.participant_identity.clone());
        let packet = match dp.value {
            Some(Value::StreamHeader(h)) => {
                Packet::Header { header: Header::from(h), encryption_type: EncryptionType::None }
            }
            Some(Value::StreamChunk(c)) => {
                Packet::Chunk { chunk: Chunk::from(c), encryption_type: EncryptionType::None }
            }
            Some(Value::StreamTrailer(t)) => {
                Packet::Trailer { trailer: Trailer::from(t), encryption_type: EncryptionType::None }
            }
            _ => continue,
        };
        packets.push(PacketReceived::new(packet, identity));
    }
    if packets.is_empty() {
        return Corpus::Reject;
    }

    runtime().block_on(async move {
        let (manager, input, mut output) = Manager::new(Some(MAX_PAYLOAD_BYTES));
        for p in packets {
            input.send(InputEvent::PacketReceived(p)).expect("manager loop alive");
        }
        input.send(InputEvent::Shutdown).expect("manager loop alive");
        manager.run().await;

        while let Ok(event) = output.try_recv() {
            if let OutputEvent::StreamOpened(opened) = event {
                let _ = match opened.stream_reader {
                    AnyStreamReader::Byte(r) => r.read_all().await.map(|b| b.len()),
                    AnyStreamReader::Text(r) => r.read_all().await.map(|s| s.len()),
                };
            }
        }
    });
    Corpus::Keep
});
