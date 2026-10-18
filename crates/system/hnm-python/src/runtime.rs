/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Shared adapter between facade futures and Python awaitables.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::future::Future;

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use crate::error::map_error;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) fn awaitable<'py, T>(
    py: Python<'py>, future: impl Future<Output = hnm::Result<T>> + Send + 'static,
) -> PyResult<Bound<'py, PyAny>>
where
    T: for<'p> IntoPyObject<'p> + Send + 'static,
{
    pyo3_async_runtimes::tokio::future_into_py(py, async move { future.await.map_err(|error| map_error(&error)) })
}

/* === Private Functions === */

/* === Tests === */
