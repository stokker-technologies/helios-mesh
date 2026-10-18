/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Preserve Firmware Domain Errors At The Crate Boundary

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::{convert::Infallible, fmt::Debug};

// Third Party Imports

// Local Imports
use crate::{boot::CatalogError, jobs::JobError, mesh, node::storage};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

pub type Result<T, E = Infallible> = core::result::Result<T, Error<E>>;

/* === Structs === */

/* === Enums === */

#[derive(Debug, thiserror::Error)]
pub enum Error<E: Debug + 'static = Infallible> {
    #[error("boot catalog failed: {0}")]
    Catalog(#[from] CatalogError),

    #[error("job failed: {primary}; cleanup: {cleanup:?}")]
    Job {
        #[source]
        primary: JobError,

        cleanup: Option<helios_mesh_protocol::error::Error>,
    },

    #[error("job cleanup failed: {0}")]
    Cleanup(#[source] helios_mesh_protocol::error::Error),

    #[error("storage failed: {0}")]
    Storage(#[from] storage::Error<E>),

    #[error("mesh link failed: {0}")]
    Mesh(#[from] mesh::link::Error),

    #[error("mesh coordinator failed: {0}")]
    Coordinator(#[from] mesh::coordinator::CoordinatorError),
}

/* === Traits === */

/* === Implementations === */

impl<E: Debug + 'static> defmt::Format for Error<E> {
    fn format(&self, formatter: defmt::Formatter<'_>) {
        defmt::write!(formatter, "{:?}", defmt::Debug2Format(self));
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use core::error::Error as _;

    use super::*;
    use crate::jobs::JobOutcome;
    use helios_mesh_protocol::error::Error as Rejection;

    #[test]
    fn job_retains_cleanup() {
        let outcome = JobOutcome { result: Err(JobError::Rejected(Rejection::Conflict)), cleanup: Some(Err(Rejection::Unavailable)) };
        let error = outcome.into_result().unwrap_err();
        assert!(error.source().unwrap().source().is_some());
        assert!(matches!(error, Error::Job { primary: JobError::Rejected(Rejection::Conflict), cleanup: Some(Rejection::Unavailable) }));

        let cleanup = JobOutcome { result: Ok(()), cleanup: Some(Err(Rejection::Unavailable)) };
        assert!(matches!(cleanup.into_result(), Err(Error::Cleanup(Rejection::Unavailable))));
        assert!(JobOutcome { result: Ok(()), cleanup: Some(Ok(())) }.into_result().is_ok());
    }

    #[test]
    fn storage_retains_device() {
        #[derive(Debug)]
        struct DeviceFault(u8);

        let error = Error::from(storage::Error::Device(DeviceFault(7)));
        assert!(error.source().is_some());
        assert!(matches!(error, Error::Storage(storage::Error::Device(DeviceFault(7)))));
    }
}
