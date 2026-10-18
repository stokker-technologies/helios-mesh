/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Define Shared Python Enum Bindings Before Their Consumers.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports

/* === Constants === */

/* === Macros === */

macro_rules! unit_enum {
    (
        $(#[$meta:meta])*
        $wrapper:ident, $inner:ty, $python_name:literal, $module:literal {
            $($variant:ident => $attr:ident, $label:literal;)+
        }
        $(methods { $($extra:tt)* })?
    ) => {
        impl $wrapper {
            pub(crate) const fn wrap(inner: $inner) -> Self {
                Self { inner }
            }
        }

        #[pyo3::pymethods]
        impl $wrapper {
            $(
                #[classattr]
                const $attr: Self = Self::wrap(<$inner>::$variant);
            )+

                        #[getter]
            const fn name(&self) -> &'static str {
                match self.inner {
                    $(<$inner>::$variant => $label,)+
                }
            }

            const fn __str__(&self) -> &'static str {
                self.name()
            }

            fn __repr__(&self) -> String {
                format!("{}.{}", $python_name, self.name().to_ascii_uppercase())
            }

            fn __eq__(&self, other: &Self) -> bool {
                self.inner == other.inner
            }

            fn __hash__(&self) -> u64 {
                // FNV-1a over the stable name: deterministic per variant.
                let mut hash = 0xcbf2_9ce4_8422_2325_u64;
                let mut bytes = self.name().as_bytes();
                while let [byte, rest @ ..] = bytes {
                    hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
                    bytes = rest;
                }
                hash
            }

            $($($extra)*)?
        }
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
