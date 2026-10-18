/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Generic framed USB transport loops feeding the node dispatcher.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::future::Future;

// Third Party Imports
use embassy_time::{Duration, with_timeout};

// Local Imports
use crate::node::{Handle, Node, Origin, Submission};
use helios_mesh_protocol::{
    codec::{MAX_FRAME_SIZE, decode_request, encode_response},
    wire::RequestEnvelope,
};

/* === Constants === */

const READ_CHUNK: usize = 64;
const WRITE_TIMEOUT: Duration = Duration::from_secs(1);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

struct Accumulator {
    buffer: [u8; MAX_FRAME_SIZE],
    length: usize,
}

/* === Enums === */

/* === Traits === */

pub trait FrameRead {
    type Error;

    fn wait_connected(&mut self) -> impl Future<Output = ()>;

    fn read(&mut self, buffer: &mut [u8]) -> impl Future<Output = Result<usize, Self::Error>>;
}

pub trait FrameWrite {
    type Error;

    fn wait_connected(&mut self) -> impl Future<Output = ()>;

    fn write(&mut self, frame: &[u8]) -> impl Future<Output = Result<(), Self::Error>>;
}

/* === Implementations === */

impl Accumulator {
    const fn new() -> Self {
        Self { buffer: [0; MAX_FRAME_SIZE], length: 0 }
    }

    fn reset(&mut self) {
        self.length = 0;
    }

    fn push(&mut self, byte: u8) -> Option<RequestEnvelope> {
        if self.length == self.buffer.len() {
            self.length = 0;
        }
        self.buffer[self.length] = byte;
        self.length += 1;
        if byte != 0 {
            return None;
        }
        let frame_length = self.length;
        self.length = 0;
        decode_request(&mut self.buffer[..frame_length]).ok()
    }
}

/* === Public Functions === */

pub async fn run_usb_reader<T, R>(mut transport: T, node: Handle<R>) -> !
where
    T: FrameRead,
    R: Node + 'static,
{
    let mut frames = Accumulator::new();
    loop {
        transport.wait_connected().await;
        defmt::info!("USB request endpoint connected");
        frames.reset();
        let mut packet = [0; READ_CHUNK];
        while let Ok(length) = transport.read(&mut packet).await {
            for &byte in &packet[..length] {
                if let Some(envelope) = frames.push(byte) {
                    node.submit(Submission {
                        origin: Origin::Usb,
                        version: envelope.version,
                        request_id: envelope.request_id,
                        destination: envelope.destination,
                        request: envelope.request,
                    })
                    .await;
                }
            }
        }
        defmt::info!("USB request endpoint disconnected");
    }
}

pub async fn run_usb_writer<T, R>(mut transport: T, node: Handle<R>) -> !
where
    T: FrameWrite,
    R: Node + 'static,
{
    loop {
        transport.wait_connected().await;
        defmt::info!("USB response endpoint connected");
        loop {
            let response = node.next_usb_response().await;
            let mut outgoing = [0; MAX_FRAME_SIZE];
            let Ok(frame) = encode_response(&response, &mut outgoing) else {
                defmt::warn!("USB response frame overflow");
                continue;
            };
            match with_timeout(WRITE_TIMEOUT, transport.write(frame)).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => break,
                Err(_) => {
                    defmt::warn!("USB response write timed out; host not draining, dropping frame");
                    break;
                }
            }
        }
    }
}

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use helios_mesh_protocol::{
        codec::encode_request,
        node::{self, NodeId, RequestId},
        wire::{PROTOCOL_VERSION, Request},
    };

    use super::Accumulator;

    fn envelope() -> helios_mesh_protocol::wire::RequestEnvelope {
        helios_mesh_protocol::wire::RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: RequestId(7),
            destination: NodeId(566_114),
            request: Request::Node(node::Request::Status(node::Status)),
        }
    }

    #[test]
    fn corrupt_frames_delimiter() {
        let mut frames = Accumulator::new();
        let mut buffer = [0; helios_mesh_protocol::codec::MAX_FRAME_SIZE];
        let frame = encode_request(&envelope(), &mut buffer).unwrap();

        // A garbage prefix terminated by a zero is dropped silently.
        for byte in [0x17, 0x2A, 0x00] {
            assert!(frames.push(byte).is_none());
        }
        // The next complete frame decodes.
        let mut decoded = None;
        for &byte in frame {
            decoded = frames.push(byte);
        }
        assert_eq!(decoded, Some(envelope()));
    }
}
