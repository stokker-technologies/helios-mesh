/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Pure, complete electrical declarations validated before board construction.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::wings::{Motor, Oled, TftV2, Wing};
#[cfg(any(target_arch = "arm", test))]
use hnm_protocol::node::AuxGpioError;
use hnm_protocol::node::{AuxGpioAccess, AuxGpioId, AuxGpioMode};

/* === Constants === */

pub const AUXILIARY_PINS: [u8; 4] = [11, 12, 24, 25];

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug)]
pub struct Hardware {
    known: bool,
    tft: Option<TftV2>,
    oled: Option<Oled>,
    motor: bool,
    auxiliary: [Option<Auxiliary>; 4],
    error: Option<HardwareError>,
}

#[derive(Clone, Copy, Debug)]
pub struct ValidatedHardware {
    pub(crate) hardware: Hardware,
    pub(crate) access: [AuxGpioAccess; 4],

    #[cfg_attr(not(any(target_arch = "arm", test)), allow(dead_code))]
    idle: [Idle; 30],

    i2c: bool,
}

pub(crate) struct Reservation {
    used: [bool; 30],
    idle: [Idle; 30],
    addresses: [bool; 128],
    i2c: bool,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Drive {
    Ma2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Auxiliary {
    InputOnly,

    PushPull { initial: AuxGpioMode, drive: Drive },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(target_arch = "arm", derive(defmt::Format))]
pub enum HardwareError {
    #[error("invalid auxiliary GPIO allocation")]
    InvalidAuxiliary,

    #[error("pin {0} is invalid")]
    InvalidPin(u8),

    #[error("I2C address {0:#x} is already reserved")]
    AddressConflict(u8),

    #[error("pin {0} is already reserved")]
    PinConflict(u8),

    #[error("a wing was selected more than once")]
    DuplicateWing,

    #[error("unknown hardware cannot declare header capabilities")]
    UnknownAllocation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Idle {
    HighImpedance,
    PullUp,
}

/* === Traits === */

/* === Implementations === */

impl Default for Hardware {
    fn default() -> Self {
        Self::new()
    }
}

impl Hardware {
    #[must_use]
    pub const fn new() -> Self {
        Self { known: true, tft: None, oled: None, motor: false, auxiliary: [None; 4], error: None }
    }
    #[must_use]
    pub const fn unknown() -> Self {
        Self { known: false, ..Self::new() }
    }
    #[must_use]
    pub const fn wing(self, wing: Wing) -> Self {
        match wing {
            Wing::Tft(wing) => self.tft(wing),
            Wing::Oled(wing) => self.oled(wing),
            Wing::Motor(wing) => self.motor(wing),
        }
    }

    #[must_use]
    pub const fn tft(mut self, wing: TftV2) -> Self {
        if self.tft.is_some() {
            self.error = Some(HardwareError::DuplicateWing);
        }
        self.tft = Some(wing);
        self
    }
    #[must_use]
    pub const fn oled(mut self, wing: Oled) -> Self {
        if self.oled.is_some() {
            self.error = Some(HardwareError::DuplicateWing);
        }
        self.oled = Some(wing);
        self
    }
    #[must_use]
    pub const fn motor(mut self, _wing: Motor) -> Self {
        if self.motor {
            self.error = Some(HardwareError::DuplicateWing);
        }
        self.motor = true;
        self
    }
    #[must_use]
    pub fn auxiliary(mut self, id: AuxGpioId, allocation: Auxiliary) -> Self {
        if let Some(slot) = id.0.checked_sub(1).and_then(|n| self.auxiliary.get_mut(usize::from(n))) {
            if slot.is_some() {
                self.error = Some(HardwareError::InvalidAuxiliary);
            }
            *slot = Some(allocation);
        } else {
            self.error = Some(HardwareError::InvalidAuxiliary);
        }
        self
    }
    pub fn validate(self) -> crate::Result<ValidatedHardware> {
        if let Some(error) = self.error {
            return Err(error.into());
        }
        if !self.known && (self.tft.is_some() || self.oled.is_some() || self.motor || self.auxiliary.iter().any(Option::is_some)) {
            return Err(HardwareError::UnknownAllocation.into());
        }
        let mut reservation = Reservation::new();
        if let Some(tft) = self.tft {
            tft.reserve(&mut reservation)?;
        }
        if let Some(oled) = self.oled {
            oled.reserve(&mut reservation)?;
        }
        if self.motor {
            Motor::standard().reserve(&mut reservation)?;
        }
        let mut access = [AuxGpioAccess::NotAllocated; 4];
        for (index, pin) in AUXILIARY_PINS.into_iter().enumerate() {
            if reservation.used[usize::from(pin)] {
                access[index] = AuxGpioAccess::Reserved;
            }
            if let Some(allocation) = self.auxiliary[index] {
                reservation.pin(pin, Idle::HighImpedance)?;
                access[index] = match allocation {
                    Auxiliary::InputOnly => AuxGpioAccess::InputOnly,
                    Auxiliary::PushPull { .. } => AuxGpioAccess::PushPull,
                };
            }
        }
        Ok(ValidatedHardware { hardware: self, access, idle: reservation.idle, i2c: reservation.i2c })
    }
}

impl ValidatedHardware {
    #[must_use]
    pub const fn buses_enabled(&self) -> bool {
        self.hardware.known
    }
    #[must_use]
    pub const fn auxiliary_access(&self) -> [AuxGpioAccess; 4] {
        self.access
    }
    #[cfg(target_arch = "arm")]
    pub(crate) const fn motor(&self) -> bool {
        self.hardware.motor
    }
    #[must_use]
    pub const fn i2c_enabled(&self) -> bool {
        self.i2c
    }
    #[cfg(any(target_arch = "arm", test))]
    pub(crate) fn idle(&self, pin: u8) -> Idle {
        self.idle[usize::from(pin)]
    }
    #[cfg(target_arch = "arm")]
    pub(crate) const fn tft(&self) -> Option<TftV2> {
        self.hardware.tft
    }
    #[cfg(target_arch = "arm")]
    pub(crate) const fn oled(&self) -> Option<Oled> {
        self.hardware.oled
    }
    #[cfg(any(target_arch = "arm", test))]
    pub(crate) fn initial(&self, index: usize) -> AuxGpioMode {
        match self.hardware.auxiliary[index] {
            Some(Auxiliary::PushPull { initial, .. }) => initial,
            _ => AuxGpioMode::HighImpedance,
        }
    }
}

impl Reservation {
    fn new() -> Self {
        let mut used = [false; 30];
        // Fixed onboard SPI/radio and I2C wiring cannot be remapped to controls.
        for pin in [2, 3, 8, 14, 15, 16, 17, 21, 22] {
            used[pin] = true;
        }
        Self { used, idle: [Idle::HighImpedance; 30], addresses: [false; 128], i2c: false }
    }
    pub(crate) fn pin(&mut self, pin: u8, idle: Idle) -> Result<(), HardwareError> {
        claim(&mut self.used, pin)?;
        self.idle[usize::from(pin)] = idle;
        Ok(())
    }
    pub(crate) fn i2c_address(&mut self, address: u8) -> Result<(), HardwareError> {
        if self.addresses[usize::from(address)] {
            return Err(HardwareError::AddressConflict(address));
        }
        self.addresses[usize::from(address)] = true;
        self.i2c = true;
        Ok(())
    }
}

/* === Public Functions === */

#[cfg(any(target_arch = "arm", test))]
pub(crate) fn authorize(access: [AuxGpioAccess; 4], id: AuxGpioId, mode: AuxGpioMode) -> Result<usize, AuxGpioError> {
    let index = id.0.checked_sub(1).map(usize::from).filter(|n| *n < 4).ok_or(AuxGpioError::InvalidId)?;
    match access[index] {
        AuxGpioAccess::NotAllocated => Err(AuxGpioError::NotAllocated),
        AuxGpioAccess::Reserved => Err(AuxGpioError::Reserved),
        AuxGpioAccess::InputOnly => match mode {
            AuxGpioMode::HighImpedance => Ok(index),
            AuxGpioMode::DriveHigh | AuxGpioMode::DriveLow => Err(AuxGpioError::ModeNotAllowed),
        },
        AuxGpioAccess::PushPull => Ok(index),
    }
}

/* === Private Functions === */

fn claim(used: &mut [bool; 30], pin: u8) -> Result<(), HardwareError> {
    if !matches!(pin, 0 | 1 | 2 | 3 | 5 | 6 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 24 | 25 | 26 | 27 | 28 | 29) {
        return Err(HardwareError::InvalidPin(pin));
    }
    if used[usize::from(pin)] {
        return Err(HardwareError::PinConflict(pin));
    }
    used[usize::from(pin)] = true;
    Ok(())
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    fn output() -> Auxiliary {
        Auxiliary::PushPull { initial: AuxGpioMode::HighImpedance, drive: Drive::Ma2 }
    }

    #[test]
    fn default_allocates_nothing() {
        let hardware = Hardware::new().validate().unwrap();
        assert_eq!(hardware.auxiliary_access(), [AuxGpioAccess::NotAllocated; 4]);
        assert!(hardware.buses_enabled());
    }

    #[test]
    fn known_wings_buses() {
        assert!(Hardware::new().tft(TftV2::standard()).motor(Motor::standard()).validate().is_ok());
        assert!(Hardware::new().oled(Oled::standard()).motor(Motor::standard()).validate().is_ok());
        assert!(matches!(
            Hardware::new().tft(TftV2::standard()).oled(Oled::standard()).validate(),
            Err(crate::Error::Hardware(HardwareError::PinConflict(9)))
        ));
        assert!(Hardware::new().motor(Motor::standard()).motor(Motor::standard()).validate().is_err());
    }

    #[test]
    fn remaps_reserve_auxiliary() {
        let tft = TftV2 { cs: 11, ..TftV2::standard() };
        let hardware = Hardware::new().tft(tft).validate().unwrap();
        assert_eq!(hardware.auxiliary_access()[0], AuxGpioAccess::Reserved);
        assert!(matches!(
            Hardware::new().tft(tft).auxiliary(AuxGpioId(1), output()).validate(),
            Err(crate::Error::Hardware(HardwareError::PinConflict(11)))
        ));
        for pin in [2, 3, 8, 14, 15, 16, 17, 21, 22] {
            assert!(Hardware::new().tft(TftV2 { cs: pin, ..tft }).validate().is_err());
        }
        assert!(Hardware::new().tft(TftV2 { cs: 10, ..tft }).validate().is_err());
    }

    #[test]
    fn remapped_selects_high() {
        let hardware = Hardware::new().tft(TftV2 { cs: 11, sd_cs: 12, ..TftV2::standard() }).validate().unwrap();
        for pin in [11, 12] {
            assert_eq!(hardware.idle(pin), Idle::PullUp);
        }
        assert_eq!(hardware.idle(5), Idle::HighImpedance);
        assert_eq!(hardware.auxiliary_access()[..2], [AuxGpioAccess::Reserved; 2]);
        assert!(!Hardware::new().validate().unwrap().i2c_enabled());
        assert!(hardware.i2c_enabled());
    }

    #[test]
    fn conflicting_addresses_refused() {
        let mut reservation = Reservation::new();
        Motor::standard().reserve(&mut reservation).unwrap();
        for address in [0x60, 0x70] {
            assert_eq!(reservation.i2c_address(address), Err(HardwareError::AddressConflict(address)));
        }
        Oled::standard().reserve(&mut reservation).unwrap();
    }

    #[test]
    fn unknown_never_buses() {
        assert!(!Hardware::unknown().validate().unwrap().buses_enabled());
        assert!(Hardware::unknown().tft(TftV2::standard()).validate().is_err());
        assert!(Hardware::unknown().auxiliary(AuxGpioId(1), output()).validate().is_err());
    }

    #[test]
    fn wing_catalog_reservations() {
        let gateway = Hardware::new().wing(Wing::Tft(TftV2::standard())).validate().unwrap();
        assert_eq!(gateway.idle(9), Idle::PullUp);
        assert_eq!(gateway.idle(5), Idle::PullUp);

        let sidecar = Hardware::new()
            .wing(Wing::Oled(Oled::standard()))
            .auxiliary(AuxGpioId(1), output())
            .auxiliary(AuxGpioId(2), output())
            .validate()
            .unwrap();
        assert_eq!(sidecar.auxiliary_access()[..2], [AuxGpioAccess::PushPull; 2]);
        assert!(Hardware::new().wing(Wing::Motor(Motor::standard())).validate().is_ok());
        assert!(matches!(
            Hardware::new().wing(Wing::Tft(TftV2::standard())).wing(Wing::Oled(Oled::standard())).validate(),
            Err(crate::Error::Hardware(HardwareError::PinConflict(_)))
        ));
    }

    #[test]
    fn explicit_auxiliary_authority() {
        let hardware = Hardware::new().auxiliary(AuxGpioId(1), output()).auxiliary(AuxGpioId(2), Auxiliary::InputOnly).validate().unwrap();
        assert_eq!(
            hardware.auxiliary_access(),
            [AuxGpioAccess::PushPull, AuxGpioAccess::InputOnly, AuxGpioAccess::NotAllocated, AuxGpioAccess::NotAllocated]
        );
        assert_eq!(hardware.initial(0), AuxGpioMode::HighImpedance);
    }
}
