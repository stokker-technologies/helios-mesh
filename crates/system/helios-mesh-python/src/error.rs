/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Typed exception hierarchy mapped one category per facade error kind.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::{
    Bound, PyErr, PyResult, Python, create_exception,
    exceptions::PyException,
    prelude::PyModuleMethods,
    types::{PyAnyMethods, PyModule},
};

// Local Imports

/* === Constants === */

/* === Macros === */

create_exception!(helios_mesh, HeliosMeshError, PyException, "Base of every Helios Mesh facade error.");

// Device discovery and connection.
create_exception!(
    helios_mesh,
    DeviceNotFoundError,
    HeliosMeshError,
    "No matching directly attached Helios Mesh USB node could be discovered."
);

create_exception!(
    helios_mesh,
    AmbiguousDeviceError,
    HeliosMeshError,
    "More than one candidate matched an ambiguous selection: several \
     directly attached USB devices with no single coordinator among them, \
     or several Gateway.scan_one() roster matches."
);

create_exception!(helios_mesh, TransportError, HeliosMeshError, "The USB device could not be opened or its connection was interrupted.");

create_exception!(helios_mesh, TimeoutError, HeliosMeshError, "The request produced no response within its timeout.");

create_exception!(helios_mesh, ProtocolMismatchError, HeliosMeshError, "The device speaks a different protocol version than this host.");

create_exception!(
    helios_mesh,
    UnexpectedResponseError,
    HeliosMeshError,
    "The device answered with a payload that does not match the request."
);

// Local validation.
create_exception!(
    helios_mesh,
    InvalidArgumentError,
    HeliosMeshError,
    "A locally validated argument is outside the facade's accepted range."
);

// Operation outcomes.
create_exception!(
    helios_mesh,
    OperationStoppedError,
    HeliosMeshError,
    "An awaited operation stopped before reaching its requested result."
);

create_exception!(helios_mesh, OperationFailedError, HeliosMeshError, "An awaited operation failed in its hardware driver.");

create_exception!(helios_mesh, NodeUnreachableError, HeliosMeshError, "A mesh-attached node's gateway roster confirms it has gone dark.");

// Wire rejections, mapped one class per contract error kind.
create_exception!(helios_mesh, InvalidInputError, HeliosMeshError, "Request values are malformed, out of range, or violate device limits.");

create_exception!(helios_mesh, UnsupportedError, HeliosMeshError, "The addressed node or channel cannot perform this request.");

create_exception!(helios_mesh, UnauthorizedError, HeliosMeshError, "The caller is not allowed to perform this request.");

create_exception!(helios_mesh, NotFoundError, HeliosMeshError, "The addressed node, channel, or operation does not exist.");

create_exception!(helios_mesh, BusyError, HeliosMeshError, "The resource is executing conflicting work; retry after it completes.");

create_exception!(helios_mesh, ConflictError, HeliosMeshError, "NodeHandle state does not allow the request yet; change that state first.");

create_exception!(helios_mesh, BoundElsewhereError, HeliosMeshError, "Another gateway currently owns the node's write binding.");

create_exception!(helios_mesh, UnavailableError, HeliosMeshError, "The node cannot serve the request right now; retry later.");

create_exception!(helios_mesh, DuplicateRequestError, HeliosMeshError, "A request identifier was reused with different request contents.");

create_exception!(
    helios_mesh,
    AuxGpioRejectedError,
    HeliosMeshError,
    "Auxiliary GPIO permissions, pulse duration, or active state refused the request."
);

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) fn map_error(error: &helios_mesh::Error) -> PyErr {
    use helios_mesh::Error;

    let message = error.to_string();
    match error {
        Error::DeviceNotFound | Error::DeviceIdNotFound(_) => DeviceNotFoundError::new_err(message),
        Error::AmbiguousDevices(_) => AmbiguousDeviceError::new_err(message),
        Error::Device(_) | Error::Connection(_) => TransportError::new_err(message),
        Error::Timeout => TimeoutError::new_err(message),
        Error::ProtocolMismatch { .. } => ProtocolMismatchError::new_err(message),
        Error::UnexpectedResponse => UnexpectedResponseError::new_err(message),
        Error::InvalidArgument(_) => InvalidArgumentError::new_err(message),

        Error::OperationStopped(_) => OperationStoppedError::new_err(message),
        Error::OperationFailed(_) => OperationFailedError::new_err(message),
        Error::NodeUnreachable { .. } => NodeUnreachableError::new_err(message),
        Error::AuxGpio { gpio, reason } => Python::attach(|py| {
            let exception = AuxGpioRejectedError::new_err(message);
            let value = exception.value(py);
            if let Err(error) = value.setattr("gpio", crate::node::PyAuxGpioId { inner: *gpio }) {
                return error;
            }
            if let Err(error) = value.setattr("reason", crate::node::PyAuxGpioError { inner: *reason }) {
                return error;
            }
            exception
        }),
        Error::InvalidInput => InvalidInputError::new_err(message),
        Error::Unsupported => UnsupportedError::new_err(message),
        Error::Unauthorized => UnauthorizedError::new_err(message),
        Error::NotFound => NotFoundError::new_err(message),
        Error::Busy => BusyError::new_err(message),
        Error::Conflict => ConflictError::new_err(message),
        Error::BoundElsewhere { .. } => BoundElsewhereError::new_err(message),
        Error::Unavailable => UnavailableError::new_err(message),
        Error::DuplicateRequest => DuplicateRequestError::new_err(message),
    }
}

pub(crate) fn register(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    let exceptions = [
        ("HeliosMeshError", py.get_type::<HeliosMeshError>()),
        ("AuxGpioRejectedError", py.get_type::<AuxGpioRejectedError>()),
        ("DeviceNotFoundError", py.get_type::<DeviceNotFoundError>()),
        ("AmbiguousDeviceError", py.get_type::<AmbiguousDeviceError>()),
        ("TransportError", py.get_type::<TransportError>()),
        ("TimeoutError", py.get_type::<TimeoutError>()),
        ("ProtocolMismatchError", py.get_type::<ProtocolMismatchError>()),
        ("UnexpectedResponseError", py.get_type::<UnexpectedResponseError>()),
        ("InvalidArgumentError", py.get_type::<InvalidArgumentError>()),
        ("OperationStoppedError", py.get_type::<OperationStoppedError>()),
        ("OperationFailedError", py.get_type::<OperationFailedError>()),
        ("NodeUnreachableError", py.get_type::<NodeUnreachableError>()),
        ("InvalidInputError", py.get_type::<InvalidInputError>()),
        ("UnsupportedError", py.get_type::<UnsupportedError>()),
        ("UnauthorizedError", py.get_type::<UnauthorizedError>()),
        ("NotFoundError", py.get_type::<NotFoundError>()),
        ("BusyError", py.get_type::<BusyError>()),
        ("ConflictError", py.get_type::<ConflictError>()),
        ("BoundElsewhereError", py.get_type::<BoundElsewhereError>()),
        ("UnavailableError", py.get_type::<UnavailableError>()),
        ("DuplicateRequestError", py.get_type::<DuplicateRequestError>()),
    ];
    for (name, exception) in exceptions {
        module.add(name, exception)?;
    }
    Ok(())
}

/* === Private Functions === */

/* === Tests === */
