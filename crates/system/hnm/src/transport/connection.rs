/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Correlated request driver over one framed USB byte stream.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    collections::HashMap,
    future::pending,
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};

// Third Party Imports
use serde::Deserialize;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::{Instant, sleep_until},
};
use tokio_serial::SerialPortBuilderExt;

// Local Imports
use super::framing::{FrameReader, write_request};
use crate::{Error, Result};
use hnm_protocol::{
    call::Call,
    codec::decode_response,
    node::{NodeId, RequestId},
    wire::{PROTOCOL_VERSION, Request, RequestEnvelope, Response, ResponseEnvelope},
};

/* === Constants === */

pub const MAX_IN_FLIGHT: usize = 16;
const QUEUE_CAPACITY: usize = 32;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(8);
pub(crate) const EXTENDED_TIMEOUT: Duration = Duration::from_secs(20);
const EXCLUSIVE_OPEN_RETRY_ATTEMPTS: u32 = 5;
const EXCLUSIVE_OPEN_RETRY_DELAY: Duration = Duration::from_millis(100);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone)]
pub struct Connection {
    outbound: mpsc::Sender<Outbound>,
    // Releases the reader and driver tasks once the last `Connection` clone
    // referencing them goes away (its `Drop` side effect); `Self::close`
    // unwraps it directly for the deterministic alternative to that `Drop`.
    guard: Arc<TaskGuard>,
}

struct TaskGuard {
    reader: Option<JoinHandle<()>>,
    driver: Option<JoinHandle<()>>,
}

struct Outbound {
    destination: NodeId,
    request: Request,
    timeout: Duration,
    completion: oneshot::Sender<Result<Response>>,
}

struct Pending {
    deadline: Instant,
    completion: oneshot::Sender<Result<Response>>,
}

#[derive(Deserialize)]
struct EnvelopeHead {
    version: u8,
    request_id: RequestId,
}

/* === Enums === */

enum Inbound {
    // v7 grew `ResponseEnvelope` (a capability-bearing gateway roster page
    // can now inline several entries by value) enough to dwarf the other
    // variants; boxed rather than `#[allow]`ed since the host runs with a
    // real heap, unlike the no_std firmware side of this same growth.
    Response(Box<ResponseEnvelope>),
    Mismatch { request_id: RequestId, device: u8 },
    Closed(String),
}

/* === Traits === */

/* === Implementations === */

impl TaskGuard {
    async fn close(&mut self) {
        let reader = self.reader.take();
        let driver = self.driver.take();
        if let Some(reader) = &reader {
            reader.abort();
        }
        if let Some(driver) = &driver {
            driver.abort();
        }
        if let Some(reader) = reader {
            let _ = reader.await;
        }
        if let Some(driver) = driver {
            let _ = driver.await;
        }
    }
}

impl Connection {
    pub async fn open(path: &str) -> Result<Self> {
        let stream = open_serial_stream(path).await?;
        let (reader, writer) = tokio::io::split(stream);
        Ok(Self::spawn(reader, writer))
    }

    pub fn spawn<R, W>(reader: R, writer: W) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        // `FrameReader`'s own resync counter (framing.rs) has independent
        // test value proving an overlong frame is detected and discarded;
        // nothing downstream of `Connection` reads it any longer, so it
        // stays FrameReader-local instead of also threading through here.
        let corrupt_frames = Arc::new(AtomicU64::new(0));
        let (outbound, requests) = mpsc::channel(QUEUE_CAPACITY);
        let (events, inbound) = mpsc::channel(QUEUE_CAPACITY);
        let reader = tokio::spawn(run_reader(FrameReader::new(reader, corrupt_frames), events));
        let driver = tokio::spawn(run_driver(writer, requests, inbound));
        Self { outbound, guard: Arc::new(TaskGuard { reader: Some(reader), driver: Some(driver) }) }
    }

    pub async fn call<C: Call>(&self, destination: NodeId, timeout: Duration, call: C) -> Result<C::Output> {
        let (completion, response) = oneshot::channel();
        self.outbound
            .send(Outbound { destination, request: call.into(), timeout, completion })
            .await
            .map_err(|_| Error::Connection("transport driver stopped".into()))?;
        let response = response.await.map_err(|_| Error::Connection("transport driver stopped".into()))??;
        C::decode(response).map_err(Error::from)
    }

    pub async fn close(self) {
        let Self { outbound, guard } = self;
        drop(outbound);
        debug_assert_eq!(
            Arc::strong_count(&guard),
            1,
            "Connection::close expects to be the sole owner of its tasks; a live \
             sibling clone means this connection's tasks are silently left \
             running instead of force-closed out from under it"
        );
        if let Ok(mut guard) = Arc::try_unwrap(guard) {
            guard.close().await;
        }
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        // Fire-and-forget fallback for the common case: a caller simply
        // lets its last `Connection` clone go out of scope. `Self::close`
        // above is the deterministic path used wherever a caller must be
        // sure the OS handle is released before proceeding, rather than
        // whenever the runtime next polls the aborted task to completion.
        if let Some(reader) = self.reader.take() {
            reader.abort();
        }
        if let Some(driver) = self.driver.take() {
            driver.abort();
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

async fn open_serial_stream(path: &str) -> Result<tokio_serial::SerialStream> {
    let mut attempts_left = EXCLUSIVE_OPEN_RETRY_ATTEMPTS;
    loop {
        attempts_left -= 1;
        let owned = path.to_owned();
        let outcome = tokio::task::spawn_blocking(move || tokio_serial::new(&owned, 115_200).open_native_async())
            .await
            .map_err(|error| Error::Device(error.to_string()))?;
        match outcome {
            Ok(stream) => return Ok(stream),
            Err(error) if attempts_left > 0 && is_exclusive_open_race(&error) => {
                tokio::time::sleep(EXCLUSIVE_OPEN_RETRY_DELAY).await;
            }
            Err(error) => return Err(Error::Device(error.to_string())),
        }
    }
}

fn is_exclusive_open_race(error: &tokio_serial::Error) -> bool {
    matches!(error.kind(), tokio_serial::ErrorKind::NoDevice | tokio_serial::ErrorKind::Io(std::io::ErrorKind::PermissionDenied)) || {
        let message = error.to_string().to_ascii_lowercase();
        message.contains("access is denied") || message.contains("permission denied")
    }
}

async fn run_reader<R: AsyncRead + Unpin>(mut reader: FrameReader<R>, events: mpsc::Sender<Inbound>) {
    loop {
        let event = match reader.next_frame().await {
            Ok(Some(frame)) => match classify_frame(frame) {
                Some(event) => event,
                None => continue,
            },
            Ok(None) => Inbound::Closed("device closed the connection".into()),
            Err(error) => Inbound::Closed(error.to_string()),
        };
        let closed = matches!(event, Inbound::Closed(_));
        if events.send(event).await.is_err() || closed {
            return;
        }
    }
}

fn classify_frame(mut frame: Vec<u8>) -> Option<Inbound> {
    let mut decodable = frame.clone();
    if let Ok(envelope) = decode_response(&mut decodable) {
        if envelope.version == PROTOCOL_VERSION {
            return Some(Inbound::Response(Box::new(envelope)));
        }
        return Some(Inbound::Mismatch { request_id: envelope.request_id, device: envelope.version });
    }
    // The payload encoding of a foreign version is unknown, but the envelope
    // prefix (version, request identifier) stays readable.
    if let Ok(head) = postcard::from_bytes_cobs::<EnvelopeHead>(&mut frame)
        && head.version != PROTOCOL_VERSION
    {
        return Some(Inbound::Mismatch { request_id: head.request_id, device: head.version });
    }
    None
}

async fn run_driver<W: AsyncWrite + Unpin>(mut writer: W, requests: mpsc::Receiver<Outbound>, inbound: mpsc::Receiver<Inbound>) {
    drive_requests(&mut writer, requests, inbound).await;
    // Best-effort end-of-stream signal for the rare case this loop exits on
    // its own (the outbound channel closing) rather than through
    // `TaskGuard` aborting it; the abort path skips this and drops the
    // writer directly.
    let _ = tokio::io::AsyncWriteExt::shutdown(&mut writer).await;
}

async fn drive_requests<W: AsyncWrite + Unpin>(
    writer: &mut W, mut requests: mpsc::Receiver<Outbound>, mut inbound: mpsc::Receiver<Inbound>,
) {
    let mut awaiting: HashMap<u32, Pending> = HashMap::new();
    let mut next_request_id = 1_u32;
    loop {
        let next_deadline = awaiting.values().map(|pending| pending.deadline).min();
        let expiry = async {
            if let Some(deadline) = next_deadline {
                sleep_until(deadline).await;
            } else {
                pending::<()>().await;
            }
        };
        tokio::select! {
            request = requests.recv(), if awaiting.len() < MAX_IN_FLIGHT => {
                let Some(request) = request else {
                    fail_pending(&mut awaiting, "connection dropped");
                    return;
                };
                let request_id = allocate_request_id(&mut next_request_id, |id| awaiting.contains_key(&id));
                let envelope = RequestEnvelope {
                    version: PROTOCOL_VERSION,
                    request_id,
                    destination: request.destination,
                    request: request.request,
                };
                awaiting.insert(request_id.0, Pending {
                    deadline: Instant::now() + request.timeout,
                    completion: request.completion,
                });
                if let Err(error) = write_request(writer, &envelope).await {
                    fail_pending(&mut awaiting, &error.to_string());
                    return;
                }
            }
            event = inbound.recv() => {
                match event {
                    Some(Inbound::Response(envelope)) => complete_pending(&mut awaiting, *envelope),
                    Some(Inbound::Mismatch { request_id, device }) => {
                        if let Some(pending) = awaiting.remove(&request_id.0) {
                            let _ = pending.completion.send(Err(Error::ProtocolMismatch {
                                host: PROTOCOL_VERSION,
                                device: Some(device),
                            }));
                        }
                    }
                    Some(Inbound::Closed(message)) => {
                        fail_pending(&mut awaiting, &message);
                        return;
                    }
                    None => {
                        fail_pending(&mut awaiting, "USB response reader stopped");
                        return;
                    }
                }
            }
            () = expiry => {
                let now = Instant::now();
                let expired: Vec<_> = awaiting
                    .iter()
                    .filter_map(|(&request_id, pending)| {
                        (pending.deadline <= now).then_some(request_id)
                    })
                    .collect();
                for request_id in expired {
                    if let Some(pending) = awaiting.remove(&request_id) {
                        let _ = pending.completion.send(Err(Error::Timeout));
                    }
                }
            }
        }
    }
}

fn allocate_request_id(next: &mut u32, occupied: impl Fn(u32) -> bool) -> RequestId {
    loop {
        let candidate = *next;
        *next = next.wrapping_add(1).max(1);
        if !occupied(candidate) {
            return RequestId(candidate);
        }
    }
}

fn complete_pending(awaiting: &mut HashMap<u32, Pending>, envelope: ResponseEnvelope) {
    if let Some(pending) = awaiting.remove(&envelope.request_id.0) {
        let _ = pending.completion.send(Ok(envelope.response));
    }
}

fn fail_pending(awaiting: &mut HashMap<u32, Pending>, message: &str) {
    for (_, pending) in awaiting.drain() {
        let _ = pending.completion.send(Err(Error::Connection(message.to_owned())));
    }
}

/* === Tests === */

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn request_id_identifiers() {
        let occupied = |id| id == 2;
        let mut next = 1_u32;
        assert_eq!(allocate_request_id(&mut next, occupied), RequestId(1));
        assert_eq!(allocate_request_id(&mut next, occupied), RequestId(3));
        let mut wrapped = u32::MAX;
        assert_eq!(allocate_request_id(&mut wrapped, occupied), RequestId(u32::MAX));
        assert_eq!(allocate_request_id(&mut wrapped, occupied), RequestId(1));
    }

    #[test]
    fn exclusive_open_locale() {
        let windows_access_denied = tokio_serial::Error::new(tokio_serial::ErrorKind::NoDevice, "Access is denied.");
        assert!(is_exclusive_open_race(&windows_access_denied));

        // A non-English Windows install renders the same `ERROR_ACCESS_DENIED`
        // through a localized message; matching on `kind()` is what actually
        // catches this, not the (here, Italian) text.
        let windows_access_denied_localized = tokio_serial::Error::new(tokio_serial::ErrorKind::NoDevice, "Accesso negato.");
        assert!(is_exclusive_open_race(&windows_access_denied_localized));

        let unix_permission_denied =
            tokio_serial::Error::new(tokio_serial::ErrorKind::Io(std::io::ErrorKind::PermissionDenied), "Permission denied (os error 13)");
        assert!(is_exclusive_open_race(&unix_permission_denied));

        // Windows folds a plain missing device into the very same `NoDevice`
        // kind as `ERROR_ACCESS_DENIED` -- there is no locale-independent way
        // to tell them apart, so this is retried too. The accepted price is a
        // few hundred milliseconds before the correct error surfaces, not a
        // hang; see `is_exclusive_open_race`'s doc comment.
        let windows_missing = tokio_serial::Error::new(tokio_serial::ErrorKind::NoDevice, "The system cannot find the file specified.");
        assert!(is_exclusive_open_race(&windows_missing));

        let unrelated_failure = tokio_serial::Error::new(tokio_serial::ErrorKind::InvalidInput, "invalid baud rate");
        assert!(!is_exclusive_open_race(&unrelated_failure));
    }
}
