/* ***
 *
 * Helios Mesh
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
use crate::hardware::{AUXILIARY_PINS, Auxiliary, Drive, Pull, ValidatedHardware, authorize};
use helios_mesh_protocol::node::{AuxGpioAccess, AuxGpioError, AuxGpioId, AuxGpioMode};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub(crate) struct AuxiliaryGpio<P> {
    pins: [Option<P>; 4],
    modes: [AuxGpioMode; 4],
    access: [AuxGpioAccess; 4],
    claimed: [bool; 4],
    allocations: [Option<Auxiliary>; 4],
}

/* === Enums === */

/* === Traits === */

pub(crate) trait Pin {
    fn configure(&mut self, drive: Drive, pull: Pull);
    fn set_mode(&mut self, mode: AuxGpioMode);
}

/* === Implementations === */

impl<P: Pin> AuxiliaryGpio<P> {
    pub(crate) fn new(hardware: ValidatedHardware, mut claim: impl FnMut(u8) -> P) -> Self {
        let access = hardware.auxiliary_access();
        let modes = core::array::from_fn(|index| hardware.initial(index));
        let allocations = hardware.hardware.auxiliary_allocations();
        let pins = core::array::from_fn(|index| match access[index] {
            AuxGpioAccess::InputOnly | AuxGpioAccess::PushPull => {
                let mut pin = claim(AUXILIARY_PINS[index]);
                let (drive, pull) = match allocations[index] {
                    Some(Auxiliary::PushPull { drive, pull, .. }) => (drive, pull),
                    _ => (Drive::Ma2, Pull::None),
                };
                pin.configure(drive, pull);
                pin.set_mode(modes[index]);
                Some(pin)
            }
            AuxGpioAccess::NotAllocated | AuxGpioAccess::Reserved => None,
        });
        Self {
            pins,
            modes,
            access,
            claimed: [false; 4],
            allocations,
        }
    }
    pub(crate) fn allocation(&self, id: AuxGpioId) -> Auxiliary {
        self.allocations[usize::from(id.0 - 1)].expect("allocated pad ownership")
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
        fn configure(&mut self, _: Drive, _: Pull) {}
        fn set_mode(&mut self, _: AuxGpioMode) {}
    }
    #[test]
    fn board_grants_once() {
        let hardware = Hardware::new()
            .auxiliary(
                AuxGpioId(1),
                Auxiliary::PushPull {
                    initial: AuxGpioMode::HighImpedance,
                    drive: Drive::Ma2,
                    pull: Pull::None,
                },
            )
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
            fn configure(&mut self, _: Drive, _: Pull) {}
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

#[cfg(test)]
mod pulse_tests {
    use super::*;
    use crate::Hardware;
    use core::cell::RefCell;

    #[derive(Default)]
    struct State {
        electrical: Option<(Drive, Pull)>,
        modes: [Option<AuxGpioMode>; 3],
        changes: usize,
    }

    struct Spy<'a>(&'a RefCell<State>);

    impl Pin for Spy<'_> {
        fn configure(&mut self, drive: Drive, pull: Pull) {
            self.0.borrow_mut().electrical = Some((drive, pull));
        }
        fn set_mode(&mut self, mode: AuxGpioMode) {
            let mut state = self.0.borrow_mut();
            let index = state.changes;
            state.modes[index] = Some(mode);
            state.changes += 1;
        }
    }

    #[test]
    fn allocation_configures_pad_and_pulse_transitions() {
        for pull in [Pull::Up, Pull::None, Pull::Down] {
            let state = RefCell::new(State::default());
            let allocation = Auxiliary::PushPull {
                initial: AuxGpioMode::HighImpedance,
                drive: Drive::Ma12,
                pull,
            };
            let hardware = Hardware::new().auxiliary(AuxGpioId(1), allocation).validate().unwrap();
            let expected_idle = match pull {
                Pull::Up => crate::hardware::Idle::PullUp,
                Pull::Down => crate::hardware::Idle::PullDown,
                Pull::None => crate::hardware::Idle::HighImpedance,
            };
            assert_eq!(hardware.idle(11), expected_idle);
            let mut pins = AuxiliaryGpio::new(hardware, |_| Spy(&state));
            pins.claim(AuxGpioId(1)).unwrap();
            let (active, idle) = pins.allocation(AuxGpioId(1)).pulse_modes();
            pins.configure(AuxGpioId(1), active).unwrap();
            pins.configure(AuxGpioId(1), idle).unwrap();
            let state = state.borrow();
            assert_eq!(state.electrical, Some((Drive::Ma12, pull)));
            let expected = match pull {
                Pull::Up => [AuxGpioMode::HighImpedance, AuxGpioMode::DriveLow, AuxGpioMode::HighImpedance],
                Pull::None | Pull::Down => [AuxGpioMode::HighImpedance, AuxGpioMode::DriveHigh, AuxGpioMode::DriveLow],
            };
            assert_eq!(state.modes, expected.map(Some));
            assert_eq!(state.changes, 3);
        }
    }
}
