/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Transport-established request context and the role-facing service views.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::marker::PhantomData;

// Third Party Imports

// Local Imports
use super::{definition::Node, storage::Document};
use crate::diagnostics::Diagnostics;
use helios_mesh_protocol::node::{EventId, NodeId, RequestId, SessionId};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Ctx<'a, R: Node> {
    pub(crate) node: NodeId,
    pub(crate) origin: Option<Origin>,
    pub(crate) request_id: Option<RequestId>,
    pub(crate) now_ms: u64,
    pub(crate) config: &'a mut ConfigSlot<R::Config>,
    pub(crate) diagnostics: &'a Diagnostics,
    pub(crate) role: PhantomData<R>,
}

pub struct ConfigCtx<'a, C: Document> {
    slot: &'a mut ConfigSlot<C>,
}

pub(crate) struct ConfigSlot<C> {
    pub(crate) current: Option<C>,
    pub(crate) pending: Option<C>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDocument;

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Origin {
    Usb,

    Lora { peer: NodeId, session: SessionId, acknowledged_session: SessionId, acknowledged_event: EventId, broadcast: bool },
}

/* === Traits === */

/* === Implementations === */

impl<R: Node> Ctx<'_, R> {
    #[must_use]
    pub const fn node_id(&self) -> NodeId {
        self.node
    }

    #[must_use]
    pub const fn origin(&self) -> Option<Origin> {
        self.origin
    }

    #[must_use]
    pub const fn request_id(&self) -> Option<RequestId> {
        self.request_id
    }

    #[must_use]
    pub const fn now_ms(&self) -> u64 {
        self.now_ms
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        self.diagnostics
    }

    pub fn config(&mut self) -> ConfigCtx<'_, R::Config> {
        ConfigCtx { slot: self.config }
    }
}

impl<C: Document> ConfigCtx<'_, C> {
    #[must_use]
    pub fn get(&self) -> Option<&C> {
        self.slot.current.as_ref()
    }

    pub fn store(&mut self, document: C) -> Result<(), InvalidDocument> {
        if !document.is_valid() {
            return Err(InvalidDocument);
        }
        self.slot.current = Some(document);
        self.slot.pending = Some(document);
        Ok(())
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
