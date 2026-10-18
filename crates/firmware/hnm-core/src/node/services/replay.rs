/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Small fixed-capacity replay cache for idempotent mesh retries.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use hnm_protocol::{
    node::{NodeId, RequestId, SessionId},
    wire::{Request, Response},
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug, Eq, PartialEq)]
struct Entry {
    source: NodeId,
    source_session: SessionId,
    request_id: RequestId,
    request: Request,
    response: Response,
}

pub struct Cache<const N: usize> {
    entries: [Option<Entry>; N],
    next: usize,
}

/* === Enums === */

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Lookup<'a> {
    Miss,

    Replay(&'a Response),

    Conflict,
}

/* === Traits === */

/* === Implementations === */

impl<const N: usize> Cache<N> {
    #[must_use]
    pub const fn new() -> Self {
        Self { entries: [const { None }; N], next: 0 }
    }

    #[must_use]
    pub fn lookup(&self, source: NodeId, source_session: SessionId, request_id: RequestId, request: &Request) -> Lookup<'_> {
        self.entries
            .iter()
            .flatten()
            .find(|entry| entry.source == source && entry.source_session == source_session && entry.request_id == request_id)
            .map_or(Lookup::Miss, |entry| if entry.request == *request { Lookup::Replay(&entry.response) } else { Lookup::Conflict })
    }

    pub fn insert(&mut self, source: NodeId, source_session: SessionId, request_id: RequestId, request: Request, response: Response) {
        if N == 0 {
            return;
        }
        self.entries[self.next] = Some(Entry { source, source_session, request_id, request, response });
        self.next = (self.next + 1) % N;
    }
}

impl<const N: usize> Default for Cache<N> {
    fn default() -> Self {
        Self::new()
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use hnm_protocol::{
        node::{self, Poll},
        wire::Request,
    };

    use super::*;

    const SOURCE: NodeId = NodeId(273_282);
    const REQUEST_ID: RequestId = RequestId(7);

    fn request() -> Request {
        Request::Node(node::Request::Poll(Poll))
    }

    fn response() -> Response {
        Response::Node(node::Response::Polled)
    }

    #[test]
    fn request_ids_session() {
        let mut cache = Cache::<2>::new();
        cache.insert(SOURCE, SessionId(1), REQUEST_ID, request(), response());

        assert_eq!(cache.lookup(SOURCE, SessionId(1), REQUEST_ID, &request()), Lookup::Replay(&response()));
        assert_eq!(cache.lookup(SOURCE, SessionId(2), REQUEST_ID, &request()), Lookup::Miss);
    }

    #[test]
    fn reused_ids_conflict() {
        let mut cache = Cache::<2>::new();
        cache.insert(SOURCE, SessionId(1), REQUEST_ID, request(), response());

        let different = Request::Node(node::Request::Status(node::Status));
        assert_eq!(cache.lookup(SOURCE, SessionId(1), REQUEST_ID, &different), Lookup::Conflict);
    }
}
