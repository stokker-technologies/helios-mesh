/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Makes I2C1 baud-rate config changes actually land on hardware.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, Ordering};

// Third Party Imports
use embassy_embedded_hal::SetConfig;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_rp::i2c::{Async, Config, ConfigError, Error, I2c};
use embassy_rp::pac;
use embassy_rp::peripherals::I2C1;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::mutex::Mutex;
use embedded_hal::i2c::I2c as BlockingI2c;
use embedded_hal_async::i2c::{self as async_i2c, I2c as AsyncI2c};

// Local Imports
use super::i2c::{I2cBatch, SharedI2c};

/* === Constants === */

const UNSET_HZ: u32 = 0;
static LAST_APPLIED_FREQUENCY_HZ: AtomicU32 = AtomicU32::new(UNSET_HZ);
static LAST_APPLIED_HCNT: AtomicU16 = AtomicU16::new(0);
static LAST_APPLIED_LCNT: AtomicU16 = AtomicU16::new(0);
static LAST_APPLIED_SPKLEN: AtomicU8 = AtomicU8::new(0);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppliedBaud {
    pub frequency_hz: u32,

    pub hcnt: u16,

    pub lcnt: u16,

    pub spklen: u8,
}

pub struct PolledI2cDevice<'a, M: RawMutex> {
    bus: &'a Mutex<M, I2c<'static, I2C1, Async>>,
    config: Config,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl<M: RawMutex> SharedI2c<M, I2c<'static, I2C1, Async>> {
    pub fn polled_device(&self, config: Config) -> PolledI2cDevice<'_, M> {
        PolledI2cDevice { bus: &self.bus, config }
    }
}

impl<M: RawMutex> Clone for PolledI2cDevice<'_, M> {
    fn clone(&self) -> Self {
        Self { bus: self.bus, config: self.config }
    }
}

impl<M: RawMutex> async_i2c::ErrorType for PolledI2cDevice<'_, M> {
    type Error = I2cDeviceError<Error>;
}

impl<M: RawMutex> AsyncI2c for PolledI2cDevice<'_, M> {
    async fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        apply(&mut bus, &self.config).map_err(|_| I2cDeviceError::Config)?;
        AsyncI2c::read(&mut *bus, address, buffer).await.map_err(I2cDeviceError::I2c)
    }

    async fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        apply(&mut bus, &self.config).map_err(|_| I2cDeviceError::Config)?;
        AsyncI2c::write(&mut *bus, address, bytes).await.map_err(I2cDeviceError::I2c)
    }

    async fn write_read(&mut self, address: u8, write: &[u8], read: &mut [u8]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        apply(&mut bus, &self.config).map_err(|_| I2cDeviceError::Config)?;
        AsyncI2c::write_read(&mut *bus, address, write, read).await.map_err(I2cDeviceError::I2c)
    }

    async fn transaction(&mut self, address: u8, operations: &mut [async_i2c::Operation<'_>]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        apply(&mut bus, &self.config).map_err(|_| I2cDeviceError::Config)?;
        AsyncI2c::transaction(&mut *bus, address, operations).await.map_err(I2cDeviceError::I2c)
    }
}

impl<M: RawMutex> I2cBatch for PolledI2cDevice<'_, M> {
    async fn write_batch(&mut self, address: u8, writes: &[&[u8]]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        apply(&mut bus, &self.config).map_err(|_| I2cDeviceError::Config)?;
        for write in writes {
            BlockingI2c::write(&mut *bus, address, write).map_err(I2cDeviceError::I2c)?;
        }
        Ok(())
    }
}

/* === Public Functions === */

#[must_use]
pub fn last_applied() -> Option<AppliedBaud> {
    let frequency_hz = LAST_APPLIED_FREQUENCY_HZ.load(Ordering::Acquire);
    if frequency_hz == UNSET_HZ {
        return None;
    }
    Some(AppliedBaud {
        frequency_hz,
        hcnt: LAST_APPLIED_HCNT.load(Ordering::Acquire),
        lcnt: LAST_APPLIED_LCNT.load(Ordering::Acquire),
        spklen: LAST_APPLIED_SPKLEN.load(Ordering::Acquire),
    })
}

/* === Private Functions === */

fn apply(bus: &mut I2c<'static, I2C1, Async>, config: &Config) -> Result<(), ConfigError> {
    if LAST_APPLIED_FREQUENCY_HZ.load(Ordering::Acquire) == config.frequency {
        return Ok(());
    }
    let i2c1 = pac::I2C1;
    i2c1.ic_enable().write(|w| w.set_enable(false));
    // Unbounded, matching every other blocking wait in this driver stack
    // (`write_blocking_internal`'s `tx_empty`/`stop_det` polls): a disable
    // that never completes reflects a stuck bus those waits couldn't have
    // recovered from either.
    while i2c1.ic_enable_status().read().ic_en() {}
    bus.set_config(config).inspect_err(|_| {
        // embassy-rp 0.10.0's `set_config_inner` can return `Err`
        // (`ClockTooFast`/`ClockTooSlow`) after its own disable and
        // before its own re-enable (`embassy-rp-0.10.0/src/i2c.rs:
        // 431-462`) -- a third upstream gap of the same shape as the
        // unset `TX_EMPTY_CTRL` bit (`i2c1_handshake`) and the missing
        // disable-poll this module works around, worth its own defect
        // report. Unreachable today: every frequency this board ever
        // requests (400kHz, `motor_wing::MAXIMUM_I2C_FREQUENCY_HZ`;
        // 400kHz/1MHz, the displays) is comfortably in range. Re-asserting
        // the enable here means a future config this driver hasn't
        // anticipated can never leave the peripheral silently disabled
        // for whatever runs next on the bus.
        i2c1.ic_enable().write(|w| w.set_enable(true));
    })?;
    // Register-level confirmation of the fix, not just its effect: read
    // back what `set_config_inner` actually wrote, once per genuine switch
    // (never per repeated call, so this can never become log spam on the
    // wing's hot path). `hcnt`/`lcnt` should track `config.frequency`
    // directly (1MHz -> hcnt=50/lcnt=75 given this board's 125MHz
    // `clk_peri`); if a rerun ever shows the same hcnt/lcnt across two
    // different requested frequencies, the baud write is still not
    // landing and the fault is upstream of this module, not in it.
    let hcnt = i2c1.ic_fs_scl_hcnt().read().ic_fs_scl_hcnt();
    let lcnt = i2c1.ic_fs_scl_lcnt().read().ic_fs_scl_lcnt();
    let spklen = i2c1.ic_fs_spklen().read().ic_fs_spklen();
    defmt::info!(
        "i2c1_baud: applied frequency_hz={} -> ic_fs_scl_hcnt={} ic_fs_scl_lcnt={} ic_fs_spklen={}",
        config.frequency,
        hcnt,
        lcnt,
        spklen,
    );
    // Store the readback before the frequency: `last_applied` treats a
    // non-zero frequency as "hcnt/lcnt/spklen are valid", so publishing
    // them first means a concurrent reader on the other core never
    // observes the new frequency paired with a stale or zeroed count.
    LAST_APPLIED_HCNT.store(hcnt, Ordering::Release);
    LAST_APPLIED_LCNT.store(lcnt, Ordering::Release);
    LAST_APPLIED_SPKLEN.store(spklen, Ordering::Release);
    LAST_APPLIED_FREQUENCY_HZ.store(config.frequency, Ordering::Release);
    Ok(())
}

/* === Tests === */
