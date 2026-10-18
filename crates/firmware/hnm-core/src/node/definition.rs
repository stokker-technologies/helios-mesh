/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// The node contract: one node's genuine behavior behind the shared chassis.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::future::Future;

// Third Party Imports

// Local Imports
use super::{ctx::Ctx, storage::Document};
use crate::ui::RoundTripStats;
use hnm_protocol::wire;

/* === Constants === */

pub const MAX_STATUS_ROWS: usize = 6;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug)]
pub struct Outcome {
    pub(crate) response: wire::Response,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusRow {
    pub label: &'static str,

    pub value: StatusValue,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StatusRows {
    rows: heapless::Vec<StatusRow, MAX_STATUS_ROWS>,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusValue {
    Count(u32),

    Ratio(u16, u16),

    RoundTrip(RoundTripStats),

    Text(&'static str),
}

/* === Traits === */

pub trait Node: Sized {
    type Request: TryFrom<wire::Request>;

    type Config: Document;

    const NODE_TYPE: hnm_protocol::nodes::NodeType;

    fn channels(&self) -> impl Iterator<Item = crate::jobs::ChannelRef<'_>> {
        core::iter::empty()
    }

    fn handle_request(&mut self, cx: &mut Ctx<'_, Self>, request: Self::Request) -> impl Future<Output = Outcome>;

    fn before_request(&mut self, _request: &mut wire::Request) -> Result<(), hnm_protocol::error::Error> {
        Ok(())
    }

    fn after_response(&mut self, _response: &mut wire::Response) {}

    fn cancel_operation(&mut self, id: hnm_protocol::node::OperationId) -> Result<(), hnm_protocol::error::Error> {
        self.channels().find(|channel| channel.status(id).is_some()).ok_or(hnm_protocol::error::Error::NotFound)?.cancel(id)
    }

    fn operation_status(&self, id: hnm_protocol::node::OperationId) -> Option<hnm_protocol::node::OperationSnapshot> {
        self.channels().find_map(|channel| channel.snapshot(id))
    }

    fn restore(&mut self, _config: &Self::Config) {}

    fn tick(&mut self, _cx: &mut Ctx<'_, Self>) {}

    fn status(&self, _out: &mut StatusRows) {}
}

/* === Implementations === */

impl Outcome {
    #[must_use]
    pub fn reply(response: impl Into<wire::Response>) -> Self {
        Self { response: response.into() }
    }
}

impl StatusRows {
    #[must_use]
    pub const fn new() -> Self {
        Self { rows: heapless::Vec::new() }
    }

    pub fn push(&mut self, label: &'static str, value: StatusValue) {
        let _ = self.rows.push(StatusRow { label, value });
    }

    #[must_use]
    pub fn rows(&self) -> &[StatusRow] {
        &self.rows
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
