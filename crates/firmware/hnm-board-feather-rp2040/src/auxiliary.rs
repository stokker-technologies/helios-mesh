/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Owned auxiliary GPIO state and authorization, exercised with real mutation seams.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::hardware::{AUXILIARY_PINS, ValidatedHardware, authorize};
use hnm_protocol::node::{AuxGpioAccess, AuxGpioError, AuxGpioId, AuxGpioMode};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub(crate) struct AuxiliaryGpio<P> {
    pins: [Option<P>; 4],
    modes: [AuxGpioMode; 4],
    access: [AuxGpioAccess; 4],
    claimed: [bool; 4],
}

/* === Enums === */

/* === Traits === */

pub(crate) trait Pin {
    fn set_mode(&mut self, mode: AuxGpioMode);
}

/* === Implementations === */

impl<P: Pin> AuxiliaryGpio<P> {
    pub(crate) fn new(hardware: ValidatedHardware, mut claim: impl FnMut(u8) -> P) -> Self {
        let access = hardware.auxiliary_access();
        let modes = core::array::from_fn(|index| hardware.initial(index));
        let pins = core::array::from_fn(|index| match access[index] {
            AuxGpioAccess::InputOnly | AuxGpioAccess::PushPull => {
                let mut pin = claim(AUXILIARY_PINS[index]);
                pin.set_mode(modes[index]);
                Some(pin)
            }
            AuxGpioAccess::NotAllocated | AuxGpioAccess::Reserved => None,
        });
        Self { pins, modes, access, claimed: [false; 4] }
    }
    pub(crate) fn configure(&mut self, id: AuxGpioId, mode: AuxGpioMode) -> Result<(), AuxGpioError> {
        let index = authorize(self.access, id, mode)?;
        self.pins[index].as_mut().expect("allocated pad ownership").set_mode(mode);
        self.modes[index] = mode;
        Ok(())
    }
    pub(crate) fn claim(&mut self, id: AuxGpioId) -> Result<(), AuxGpioError> {
        let index = authorize(self.access, id, AuxGpioMode::HighImpedance)?;
        if self.claimed[index] {
            return Err(AuxGpioError::Busy);
        }
        self.claimed[index] = true;
        Ok(())
    }

    pub(crate) const fn modes(&self) -> [AuxGpioMode; 4] {
        self.modes
    }
    pub(crate) const fn access(&self) -> [AuxGpioAccess; 4] {
        self.access
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Auxiliary, Drive, Hardware};
    struct FakePin;
    impl Pin for FakePin {
        fn set_mode(&mut self, _: AuxGpioMode) {}
    }
    #[test]
    fn board_grants_once() {
        let hardware = Hardware::new()
            .auxiliary(AuxGpioId(1), Auxiliary::PushPull { initial: AuxGpioMode::HighImpedance, drive: Drive::Ma2 })
            .validate()
            .unwrap();
        let mut pins = AuxiliaryGpio::new(hardware, |_| FakePin);
        assert_eq!(pins.claim(AuxGpioId(1)), Ok(()));
        assert_eq!(pins.claim(AuxGpioId(1)), Err(AuxGpioError::Busy));
        assert_eq!(pins.claim(AuxGpioId(2)), Err(AuxGpioError::NotAllocated));
        assert_eq!(pins.configure(AuxGpioId(2), AuxGpioMode::DriveHigh), Err(AuxGpioError::NotAllocated));
        assert_eq!(pins.modes()[1], AuxGpioMode::HighImpedance);
        assert_eq!(pins.access()[1], AuxGpioAccess::NotAllocated);
        pins.configure(AuxGpioId(1), AuxGpioMode::DriveHigh).unwrap();
        assert_eq!(pins.modes()[0], AuxGpioMode::DriveHigh);
    }
    #[test]
    fn refused_writes_pins() {
        use crate::wings::TftV2;
        use core::cell::Cell;
        struct Spy<'a>(&'a Cell<usize>);
        impl Pin for Spy<'_> {
            fn set_mode(&mut self, _: AuxGpioMode) {
                self.0.set(self.0.get() + 1);
            }
        }
        let changes = Cell::new(0);
        let tft = TftV2 { cs: 11, ..TftV2::standard() };
        let hardware = Hardware::new().tft(tft).auxiliary(AuxGpioId(2), Auxiliary::InputOnly).validate().unwrap();
        let mut pins = AuxiliaryGpio::new(hardware, |physical| {
            assert_eq!(physical, 12);
            Spy(&changes)
        });
        assert_eq!(changes.get(), 1);
        for mode in [AuxGpioMode::DriveHigh, AuxGpioMode::DriveLow] {
            assert_eq!(pins.configure(AuxGpioId(1), mode), Err(AuxGpioError::Reserved));
            assert_eq!(pins.configure(AuxGpioId(2), mode), Err(AuxGpioError::ModeNotAllowed));
        }
        assert_eq!(pins.claim(AuxGpioId(1)), Err(AuxGpioError::Reserved));
        assert_eq!(changes.get(), 1);
        assert_eq!(pins.modes(), [AuxGpioMode::HighImpedance; 4]);
    }
}
