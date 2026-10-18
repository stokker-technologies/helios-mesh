/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Buffered zero-delimited frame extraction with corrupt-frame resync.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    collections::VecDeque,
    io, mem,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

// Third Party Imports
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// Local Imports
use crate::{Error, Result};
use hnm_protocol::codec::{MAX_FRAME_SIZE, encode_request};
use hnm_protocol::wire::RequestEnvelope;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct FrameReader<R> {
    stream: R,
    pending: Vec<u8>,
    frames: VecDeque<Vec<u8>>,
    discarding: bool,
    corrupt: Arc<AtomicU64>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl<R: AsyncRead + Unpin> FrameReader<R> {
    pub fn new(stream: R, corrupt: Arc<AtomicU64>) -> Self {
        Self { stream, pending: Vec::with_capacity(MAX_FRAME_SIZE), frames: VecDeque::new(), discarding: false, corrupt }
    }

    pub async fn next_frame(&mut self) -> io::Result<Option<Vec<u8>>> {
        loop {
            if let Some(frame) = self.frames.pop_front() {
                return Ok(Some(frame));
            }
            let mut chunk = [0_u8; MAX_FRAME_SIZE];
            let read = self.stream.read(&mut chunk).await?;
            if read == 0 {
                return Ok(None);
            }
            self.ingest(&chunk[..read]);
        }
    }

    fn ingest(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if self.discarding {
                if byte == 0 {
                    self.discarding = false;
                }
                continue;
            }
            self.pending.push(byte);
            if byte == 0 {
                let frame = mem::take(&mut self.pending);
                // A lone delimiter is resync residue, not a frame.
                if frame.len() > 1 {
                    self.frames.push_back(frame);
                }
                continue;
            }
            if self.pending.len() >= MAX_FRAME_SIZE {
                self.pending.clear();
                self.discarding = true;
                self.corrupt.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/* === Public Functions === */

pub async fn write_request<W: AsyncWrite + Unpin>(stream: &mut W, request: &RequestEnvelope) -> Result<()> {
    let mut outgoing = [0; MAX_FRAME_SIZE];
    let frame = encode_request(request, &mut outgoing).map_err(|_| Error::InvalidArgument("request does not fit one USB frame"))?;
    stream.write_all(frame).await.map_err(|error| Error::Connection(error.to_string()))?;
    stream.flush().await.map_err(|error| Error::Connection(error.to_string()))
}

/* === Private Functions === */

/* === Tests === */
