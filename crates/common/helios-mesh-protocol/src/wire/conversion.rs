/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Define Request Conversion Implementations Before Their Parent Re-Exports.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports

/* === Constants === */

/* === Macros === */

macro_rules! wrap_requests {
    ($group:ident: $($payload:ident),* $(,)?) => {
        $(
            impl From<$payload> for Request {
                fn from(request: $payload) -> Self {
                    Self::$payload(request)
                }
            }

            impl TryFrom<$crate::wire::Request> for $payload {
                type Error = $crate::wire::Request;
                fn try_from(request: $crate::wire::Request) -> Result<Self, Self::Error> {
                    match request {
                        $crate::wire::Request::$group(Request::$payload(payload)) => Ok(payload),
                        other => Err(other),
                    }
                }
            }

            impl From<$payload> for $crate::wire::Request {
                fn from(request: $payload) -> Self {
                    Self::$group(request.into())
                }
            }
        )*
    };
}

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
