/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Transport-independent request and response messages.

/* === Modules === */

#[macro_use]
mod conversion;
pub mod gateway;
pub mod mesh;
pub mod motion;
pub mod node;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use serde::{Deserialize, Serialize};

// Local Imports
use crate::{
    error::Error,
    node::{EventId, NodeId, RequestId},
};

/* === Constants === */

pub const PROTOCOL_VERSION: u8 = 14;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub version: u8,

    pub request_id: RequestId,

    pub destination: NodeId,

    pub request: Request,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub version: u8,

    pub request_id: RequestId,

    pub response: Response,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,

    pub payload: EventPayload,
}

/* === Enums === */

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Request {
    Node(node::Request),

    Gateway(gateway::Request),

    Motion(motion::Request),
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Node(node::Response),

    Gateway(gateway::Response),

    Motion(motion::Response),

    Error(Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventPayload {
    Motion(motion::Event),
}

/* === Traits === */

/* === Implementations === */

impl Response {
    #[must_use]
    pub const fn is_success(&self) -> bool {
        !matches!(self, Self::Error(_))
    }
}

impl From<node::Request> for Request {
    fn from(request: node::Request) -> Self {
        Self::Node(request)
    }
}

impl From<gateway::Request> for Request {
    fn from(request: gateway::Request) -> Self {
        Self::Gateway(request)
    }
}

impl From<motion::Request> for Request {
    fn from(request: motion::Request) -> Self {
        Self::Motion(request)
    }
}

impl From<node::Response> for Response {
    fn from(response: node::Response) -> Self {
        Self::Node(response)
    }
}

impl From<gateway::Response> for Response {
    fn from(response: gateway::Response) -> Self {
        Self::Gateway(response)
    }
}

impl From<motion::Response> for Response {
    fn from(response: motion::Response) -> Self {
        Self::Motion(response)
    }
}

impl From<Error> for Response {
    fn from(error: Error) -> Self {
        Self::Error(error)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
