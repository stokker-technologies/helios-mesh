/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Exercise The Host Runtime Through In-Process Byte Streams.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

// Third Party Imports
use tokio::io::AsyncWriteExt;

// Local Imports
use helios_mesh::testing::FrameReader;
use helios_mesh_protocol::codec::MAX_FRAME_SIZE;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

fn reader(bytes: &[u8]) -> (FrameReader<&[u8]>, Arc<AtomicU64>) {
    let corrupt = Arc::new(AtomicU64::new(0));
    (FrameReader::new(bytes, corrupt.clone()), corrupt)
}

#[tokio::test(start_paused = true)]
async fn splits_multiple_chunk() {
    let (mut frames, corrupt) = reader(b"\x01\x02\0\x03\0");
    assert_eq!(frames.next_frame().await.unwrap().unwrap(), b"\x01\x02\0");
    assert_eq!(frames.next_frame().await.unwrap().unwrap(), b"\x03\0");
    assert_eq!(frames.next_frame().await.unwrap(), None);
    assert_eq!(corrupt.load(Ordering::Relaxed), 0);
}

#[tokio::test(start_paused = true)]
async fn reassembles_frames_reads() {
    let (host, mut device) = tokio::io::duplex(64);
    let corrupt = Arc::new(AtomicU64::new(0));
    let mut frames = FrameReader::new(host, corrupt);
    device.write_all(b"\x01\x02").await.unwrap();
    device.write_all(b"\x03\0\x04").await.unwrap();
    assert_eq!(frames.next_frame().await.unwrap().unwrap(), b"\x01\x02\x03\0");
    device.write_all(b"\0").await.unwrap();
    assert_eq!(frames.next_frame().await.unwrap().unwrap(), b"\x04\0");
}

#[tokio::test(start_paused = true)]
async fn discards_overlong_resynchronizes() {
    let mut bytes = vec![1_u8; MAX_FRAME_SIZE + 16];
    bytes.push(0);
    bytes.extend_from_slice(b"\x07\0");
    let (mut frames, corrupt) = reader(&bytes);
    assert_eq!(frames.next_frame().await.unwrap().unwrap(), b"\x07\0");
    assert_eq!(corrupt.load(Ordering::Relaxed), 1);
}

#[tokio::test(start_paused = true)]
async fn skips_lone_corruption() {
    let (mut frames, corrupt) = reader(b"\0\0\x05\0");
    assert_eq!(frames.next_frame().await.unwrap().unwrap(), b"\x05\0");
    assert_eq!(frames.next_frame().await.unwrap(), None);
    assert_eq!(corrupt.load(Ordering::Relaxed), 0);
}

/* === Tests === */
