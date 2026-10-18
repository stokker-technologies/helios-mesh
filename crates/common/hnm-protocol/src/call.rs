/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Typed host-side request dispatch.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::fmt;

// Third Party Imports

// Local Imports
use crate::{
    diagnostics::{JournalPage, NodeDiagnostics},
    error::Error,
    gateway::{self, NodePage},
    motion,
    node::{self, MAX_CHANNELS, NodeDescription, NodeStatus, OperationId, OperationSnapshot},
    wire,
};

/* === Constants === */

/* === Macros === */

macro_rules! call {
    ($request:ty => $output:ty, $pattern:pat => $extract:expr) => {
        impl Call for $request {
            type Output = $output;

            fn decode(response: wire::Response) -> Result<Self::Output, CallError> {
                match response {
                    $pattern => Ok($extract),
                    wire::Response::Error(error) => Err(CallError::Rejected(error)),
                    _ => Err(CallError::UnexpectedResponse),
                }
            }
        }
    };
}

call!(node::Describe => NodeDescription,
    wire::Response::Node(node::Response::Description(description)) => description);

call!(node::Status => NodeStatus,
    wire::Response::Node(node::Response::Status(status)) => status);

call!(node::Identify => (),
    wire::Response::Node(node::Response::Identified) => ());

call!(node::ReadDiagnostics => NodeDiagnostics,
    wire::Response::Node(node::Response::Diagnostics(diagnostics)) => diagnostics);

call!(node::ReadJournal => JournalPage,
    wire::Response::Node(node::Response::Journal(page)) => page);

call!(node::CancelOperation => (), wire::Response::Node(node::Response::OperationCancelled) => ());

call!(node::OperationStatus => OperationSnapshot,
    wire::Response::Node(node::Response::OperationStatus(snapshot)) => snapshot);

call!(node::ConfigureAuxGpio => (),
    wire::Response::Node(node::Response::AuxGpioConfigured { .. }) => ());

call!(node::PulseAuxGpio => node::OperationId,
    wire::Response::Node(node::Response::AuxGpioPulseScheduled { operation, .. }) => operation);

call!(node::Restart => (),
    wire::Response::Node(node::Response::RestartScheduled { .. }) => ());

call!(node::Poll => (),
    wire::Response::Node(node::Response::Polled) => ());

call!(gateway::Nodes => NodePage,
    wire::Response::Gateway(gateway::Response::Nodes(page)) => page);

call!(motion::MoveTo => OperationId,
    wire::Response::Motion(motion::Response::Accepted { operation }) => operation);

call!(motion::MoveRelative => OperationId,
    wire::Response::Motion(motion::Response::Accepted { operation }) => operation);

call!(motion::MoveTogether => heapless::Vec<OperationId, MAX_CHANNELS>,
    wire::Response::Motion(motion::Response::AcceptedTogether { operations }) => operations);

call!(motion::Stop => OperationId,
    wire::Response::Motion(motion::Response::Accepted { operation }) => operation);

call!(motion::SetHome => OperationId,
    wire::Response::Motion(motion::Response::Accepted { operation }) => operation);

call!(motion::Configure => (),
    wire::Response::Motion(motion::Response::Configured) => ());

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallError {
    Rejected(Error),

    UnexpectedResponse,
}

/* === Traits === */

pub trait Call: Into<wire::Request> {
    type Output;

    fn decode(response: wire::Response) -> Result<Self::Output, CallError>;
}

/* === Implementations === */

impl fmt::Display for CallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(error) => write!(formatter, "request rejected: {error:?}"),
            Self::UnexpectedResponse => formatter.write_str("response payload does not match the request"),
        }
    }
}

impl std::error::Error for CallError {}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_decodes_operation() {
        let response =
            node::Response::AuxGpioPulseScheduled { operation: crate::node::OperationId(1), gpio: node::AuxGpioId(2), duration_ms: 60_000 }
                .into();
        assert_eq!(node::PulseAuxGpio::decode(response), Ok(node::OperationId(1)));
    }

    #[test]
    fn decode_extracts_output() {
        let response = wire::Response::Motion(motion::Response::Accepted { operation: OperationId(7) });
        assert_eq!(motion::MoveTo::decode(response), Ok(OperationId(7)));
    }

    #[test]
    fn decode_maps_errors() {
        let response = wire::Response::Error(Error::Busy);
        assert_eq!(node::Describe::decode(response), Err(CallError::Rejected(Error::Busy)));
    }

    #[test]
    fn decode_rejects_shapes() {
        let response = wire::Response::Node(node::Response::Identified);
        assert_eq!(node::Describe::decode(response), Err(CallError::UnexpectedResponse));
    }
}
