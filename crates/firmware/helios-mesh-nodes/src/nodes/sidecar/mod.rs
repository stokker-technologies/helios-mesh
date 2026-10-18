/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Sidecar routes and complete pulse jobs over two independently owned pins.

/* === Modules === */

pub mod jobs;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_executor::Spawner;

// Local Imports
use helios_mesh::{
    AuxGpioConfigured, AuxGpioId, AuxGpioPulseScheduled, AuxiliaryPin, Board, Channel, ConfigureAuxGpio, Context, Node, NodeType,
    PulseAuxGpio, RequestError,
};
use jobs::{Jobs, Pulse};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[helios_mesh::node]
pub struct Sidecar {
    channel_1: Channel<Jobs, AuxiliaryPin>,
    channel_2: Channel<Jobs, AuxiliaryPin>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Sidecar {
    pub fn initialize(board: &mut Board, spawner: Spawner) -> Self {
        Self::start(spawner, board.take_auxiliary(AuxGpioId(1)).ok(), board.take_auxiliary(AuxGpioId(2)).ok())
    }

    fn channel(&self, id: AuxGpioId) -> Result<&Channel<Jobs, AuxiliaryPin>, RequestError> {
        match id.0 {
            1 => Ok(&self.channel_1),
            2 => Ok(&self.channel_2),
            _ => Err(RequestError::NotFound),
        }
    }
}

#[helios_mesh::node]
impl Node for Sidecar {
    type Config = ();

    const NODE_TYPE: helios_mesh::NodeType = NodeType::Sidecar;

    #[helios_mesh::route]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "Node route signatures are asynchronous by the helios_mesh::node contract."
    )]
    async fn set_gpio(&mut self, _cx: &mut Context<'_, Self>, request: ConfigureAuxGpio) -> Result<AuxGpioConfigured, RequestError> {
        self.channel(request.gpio)?.try_resources()?.set(request.mode)?;
        Ok(AuxGpioConfigured { gpio: request.gpio, mode: request.mode })
    }

    #[helios_mesh::route]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "Node route signatures are asynchronous by the helios_mesh::node contract."
    )]
    async fn pulse_gpio(&mut self, _cx: &mut Context<'_, Self>, request: PulseAuxGpio) -> Result<AuxGpioPulseScheduled, RequestError> {
        let operation = self.channel(request.gpio)?.submit(Pulse::new(request.duration_ms)?)?;
        Ok(AuxGpioPulseScheduled { gpio: request.gpio, duration_ms: request.duration_ms, operation: operation.id() })
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
