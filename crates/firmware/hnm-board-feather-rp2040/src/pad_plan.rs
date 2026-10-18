/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Own and quiesce the entire header before minting shared-bus tokens.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::mem::ManuallyDrop;

// Third Party Imports
use embassy_rp::{
    Peri,
    gpio::{Flex, Pin, Pull},
    peripherals::{
        PIN_0, PIN_1, PIN_2, PIN_3, PIN_5, PIN_6, PIN_8, PIN_9, PIN_10, PIN_11, PIN_12, PIN_13, PIN_14, PIN_15, PIN_24, PIN_25, PIN_26,
        PIN_27, PIN_28, PIN_29,
    },
};

// Local Imports
use crate::hardware::{Idle, ValidatedHardware};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub(crate) struct SpiPads {
    sck: Peri<'static, PIN_14>,
    mosi: Peri<'static, PIN_15>,
    miso: Peri<'static, PIN_8>,
}

pub(crate) struct I2cPads {
    sda: Peri<'static, PIN_2>,
    scl: Peri<'static, PIN_3>,
}

pub(crate) struct Header {
    pins: [Option<ManuallyDrop<Flex<'static>>>; 30],
}

pub(crate) struct PadPlan;

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Header {
    pub(crate) fn take(&mut self, pin: u8) -> ManuallyDrop<Flex<'static>> {
        self.pins[usize::from(pin)].take().expect("validated exclusive pad")
    }
}

impl PadPlan {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn quiesce(
        plan: &ValidatedHardware, p0: Peri<'static, PIN_0>, p1: Peri<'static, PIN_1>, p2: Peri<'static, PIN_2>, p3: Peri<'static, PIN_3>,
        p5: Peri<'static, PIN_5>, p6: Peri<'static, PIN_6>, p8: Peri<'static, PIN_8>, p9: Peri<'static, PIN_9>, p10: Peri<'static, PIN_10>,
        p11: Peri<'static, PIN_11>, p12: Peri<'static, PIN_12>, p13: Peri<'static, PIN_13>, p14: Peri<'static, PIN_14>,
        p15: Peri<'static, PIN_15>, p24: Peri<'static, PIN_24>, p25: Peri<'static, PIN_25>, p26: Peri<'static, PIN_26>,
        p27: Peri<'static, PIN_27>, p28: Peri<'static, PIN_28>, p29: Peri<'static, PIN_29>,
    ) -> (Option<SpiPads>, Option<I2cPads>, Header) {
        let mut header = Header { pins: core::array::from_fn(|_| None) };
        header.pins[0] = Some(idle(p0, 0, plan));
        header.pins[1] = Some(idle(p1, 1, plan));
        header.pins[5] = Some(idle(p5, 5, plan));
        header.pins[6] = Some(idle(p6, 6, plan));
        header.pins[9] = Some(idle(p9, 9, plan));
        header.pins[10] = Some(idle(p10, 10, plan));
        header.pins[11] = Some(idle(p11, 11, plan));
        header.pins[12] = Some(idle(p12, 12, plan));
        header.pins[13] = Some(idle(p13, 13, plan));
        header.pins[24] = Some(idle(p24, 24, plan));
        header.pins[25] = Some(idle(p25, 25, plan));
        header.pins[26] = Some(idle(p26, 26, plan));
        header.pins[27] = Some(idle(p27, 27, plan));
        header.pins[28] = Some(idle(p28, 28, plan));
        header.pins[29] = Some(idle(p29, 29, plan));

        if !plan.buses_enabled() {
            header.pins[2] = Some(idle(p2, 2, plan));
            header.pins[3] = Some(idle(p3, 3, plan));
            header.pins[8] = Some(idle(p8, 8, plan));
            header.pins[14] = Some(idle(p14, 14, plan));
            header.pins[15] = Some(idle(p15, 15, plan));

            return (None, None, header);
        }
        // All header pads, not only chip selects, are parked before bus tokens
        // leave this owner. The typed tokens remain owned while PAC sets idle.
        for pin in [2, 3, 8, 14, 15] {
            idle_bus_pin(pin);
        }
        let i2c = if plan.i2c_enabled() {
            Some(I2cPads { sda: p2, scl: p3 })
        } else {
            header.pins[2] = Some(idle(p2, 2, plan));
            header.pins[3] = Some(idle(p3, 3, plan));
            None
        };
        (Some(SpiPads { sck: p14, mosi: p15, miso: p8 }), i2c, header)
    }
}

impl SpiPads {
    pub(crate) fn into_parts(self) -> (Peri<'static, PIN_14>, Peri<'static, PIN_15>, Peri<'static, PIN_8>) {
        (self.sck, self.mosi, self.miso)
    }
}

impl I2cPads {
    pub(crate) fn into_parts(self) -> (Peri<'static, PIN_3>, Peri<'static, PIN_2>) {
        (self.scl, self.sda)
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn idle(pin: Peri<'static, impl Pin>, number: u8, plan: &ValidatedHardware) -> ManuallyDrop<Flex<'static>> {
    let mut pin = Flex::new(pin);
    pin.set_as_input();
    pin.set_drive_strength(embassy_rp::gpio::Drive::_2mA);
    pin.set_pull(Pull::None);
    if plan.idle(number) == Idle::PullUp {
        pin.set_high();
        pin.set_pull(Pull::Up);
    }
    ManuallyDrop::new(pin)
}

fn idle_bus_pin(index: u8) {
    let bit = 1_u32 << index;
    rp_pac::SIO.gpio_out(0).value_clr().write_value(bit);
    rp_pac::SIO.gpio_oe(0).value_clr().write_value(bit);
    rp_pac::PADS_BANK0.gpio(usize::from(index)).modify(|value| {
        value.set_ie(true);
        value.set_pue(false);
        value.set_pde(false);
    });
}

/* === Tests === */
