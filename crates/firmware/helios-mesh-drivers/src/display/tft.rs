/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Adafruit 2.4 inch TFT `FeatherWing` V2 driver.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::convert::Infallible;

// Third Party Imports
use embassy_futures::yield_now;
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::{
    Pixel,
    geometry::{OriginDimensions, Size},
    mono_font::ascii::{FONT_6X10, FONT_10X20},
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::{DrawTarget, RawData},
};
use embedded_hal::digital::{ErrorType, OutputPin};
use embedded_hal_async::{i2c::I2c, spi::SpiDevice};
use sky_ili9341::{AsyncBuilder, AsyncDisplay, AsyncSpiInterface, ColorOrder, Orientation};

// Local Imports
use super::{Controls, Metrics, Palette, Panel, SnapshotReceiver};

/* === Constants === */

pub const TOUCH_ADDRESS: u8 = 0x48;
pub const MAXIMUM_TOUCH_I2C_FREQUENCY_HZ: u32 = 400_000;
pub const MAXIMUM_SPI_FREQUENCY_HZ: u32 = 32_000_000;
const WIDTH: usize = 320;
const HEIGHT: usize = 240;
const BAND_HEIGHT: usize = 10;
const BAND_BYTES: usize = WIDTH * BAND_HEIGHT * 2;
const TOUCH_PRESSURE_MIN: u16 = 100;
const TOUCH_HORIZONTAL_MIDPOINT: u16 = 2_000;
const BACKGROUND: Rgb565 = Rgb565::new(2, 3, 3);
const FOREGROUND: Rgb565 = Rgb565::new(28, 57, 27);
const MUTED: Rgb565 = Rgb565::new(16, 31, 16);
const ACCENT: Rgb565 = Rgb565::new(5, 35, 29);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Band {
    y: i32,
    pixels: [Rgb565; WIDTH * BAND_HEIGHT],
}

pub struct Tft<SPI, DC, I2C> {
    spi: SPI,
    dc: DC,
    i2c: I2C,
}

struct TftPanel<SPI, DC, I2C> {
    display: AsyncDisplay<AsyncSpiInterface<SPI, DC>>,
    touch: I2C,
    band: Band,
    pixels: [u8; BAND_BYTES],
}

struct NoReset;

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Band {
    fn new(y: usize) -> Self {
        Self { y: i32::try_from(y).unwrap_or_default(), pixels: [BACKGROUND; WIDTH * BAND_HEIGHT] }
    }

    fn encode(&self, output: &mut [u8; BAND_BYTES]) {
        for (bytes, color) in output.as_chunks_mut::<2>().0.iter_mut().zip(self.pixels.iter().copied()) {
            bytes.copy_from_slice(&RawU16::from(color).into_inner().to_be_bytes());
        }
    }
}

impl OriginDimensions for Band {
    fn size(&self) -> Size {
        Size::new(320, 240)
    }
}

impl DrawTarget for Band {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x < 0 || point.x >= 320 || point.y < self.y || point.y >= self.y + 10 {
                continue;
            }
            let row = usize::try_from(point.y - self.y).unwrap_or_default();
            let column = usize::try_from(point.x).unwrap_or_default();
            self.pixels[row * WIDTH + column] = color;
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.pixels.fill(color);
        Ok(())
    }
}

impl ErrorType for NoReset {
    type Error = Infallible;
}

impl OutputPin for NoReset {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<SPI, DC, I2C> Tft<SPI, DC, I2C>
where
    SPI: SpiDevice<u8>,
    DC: OutputPin,
    I2C: I2c,
{
    pub const fn new(spi: SPI, dc: DC, i2c: I2C) -> Self {
        Self { spi, dc, i2c }
    }

    pub async fn run(self, snapshots: SnapshotReceiver) -> ! {
        let interface = AsyncSpiInterface::new(self.spi, self.dc);
        let mut reset = NoReset;
        let mut delay = Delay;
        let Ok(display) = AsyncBuilder::new(interface)
            .orientation(Orientation::Landscape)
            .color_order(ColorOrder::Bgr)
            .init(&mut reset, &mut delay)
            .await
        else {
            defmt::error!("TFT initialization failed; display engine parked");
            core::future::pending().await
        };
        let panel = TftPanel { display, touch: self.i2c, band: Band::new(0), pixels: [0; BAND_BYTES] };
        super::run(panel, snapshots).await
    }
}

impl<SPI, DC, I2C> Panel for TftPanel<SPI, DC, I2C>
where
    SPI: SpiDevice<u8>,
    DC: OutputPin,
    I2C: I2c,
{
    type Color = Rgb565;
    type Band = Band;
    type Error = sky_ili9341::Error<SPI::Error>;

    fn metrics(&self) -> Metrics<Rgb565> {
        metrics()
    }

    fn begin_band(&mut self, y: usize) -> &mut Band {
        self.band = Band::new(y);
        &mut self.band
    }

    async fn present_band(&mut self, y: usize) -> Result<(), Self::Error> {
        self.band.encode(&mut self.pixels);
        let start_y = u16::try_from(y).unwrap_or_default();
        let end_y = u16::try_from(y + BAND_HEIGHT - 1).unwrap_or_default();
        self.display.set_address_window(0, start_y, u16::try_from(WIDTH - 1).unwrap_or_default(), end_y).await?;
        self.display.write_pixels_raw(&self.pixels).await?;
        // Give latency-critical radio work an opportunity to acquire the
        // shared SPI bus between bounded display bands.
        yield_now().await;
        Ok(())
    }

    async fn sample(&mut self) -> Controls {
        // In the TFT's landscape orientation, the panel's raw Y axis runs
        // from the left arrow to the right arrow. Treat the full left and
        // right halves as generous touch targets for the visible controls.
        match read_touch(&mut self.touch).await {
            Some(horizontal) if horizontal < TOUCH_HORIZONTAL_MIDPOINT => Controls { previous: true, next: false },
            Some(_) => Controls { previous: false, next: true },
            None => Controls::default(),
        }
    }
}

/* === Public Functions === */

pub async fn detect<I: I2c>(i2c: &mut I) -> bool {
    tsc_command(i2c, 0, 0).await.is_some()
}

/* === Private Functions === */

fn metrics() -> Metrics<Rgb565> {
    Metrics {
        width: WIDTH,
        height: HEIGHT,
        band_height: BAND_HEIGHT,
        title_font: &FONT_10X20,
        body_font: &FONT_6X10,
        palette: Palette { background: BACKGROUND, foreground: FOREGROUND, muted: MUTED, accent: ACCENT },
        margin: 10,
        header_height: 40,
        rows_start: 48,
        row_pitch: 24,
        footer_height: 46,
        separators: true,
        refresh_interval: Duration::from_secs(1),
    }
}

async fn read_touch<I: I2c>(i2c: &mut I) -> Option<u16> {
    let x = tsc_command(i2c, 12, 1).await?;
    let y = tsc_command(i2c, 13, 1).await?;
    let pressure = tsc_command(i2c, 14, 1).await?;
    let _ = tsc_command(i2c, 0, 0).await;

    if pressure <= TOUCH_PRESSURE_MIN || x == 4_095 || y == 4_095 {
        return None;
    }
    Some(y)
}

async fn tsc_command<I: I2c>(i2c: &mut I, function: u8, power: u8) -> Option<u16> {
    let command = (function << 4) | (power << 2);
    i2c.write(TOUCH_ADDRESS, &[command]).await.ok()?;
    Timer::after_micros(500).await;
    let mut reply = [0; 2];
    i2c.read(TOUCH_ADDRESS, &mut reply).await.ok()?;
    Some((u16::from(reply[0]) << 4) | (u16::from(reply[1]) >> 4))
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::prelude::RgbColor;

    #[test]
    fn band_encoding_order() {
        let band = Band { y: 0, pixels: [Rgb565::RED; WIDTH * BAND_HEIGHT] };
        let mut encoded = [0_u8; BAND_BYTES];

        band.encode(&mut encoded);

        assert_eq!(&encoded[..2], &[0xf8, 0x00]);
        assert!(encoded.as_chunks::<2>().0.iter().all(|pixel| *pixel == [0xf8, 0x00]));
    }

    #[test]
    fn band_accepts_render() {
        let mut snapshot = super::super::Snapshot::default();
        let _ = snapshot.local.title.push_str("Local 273282");
        let order = super::super::PageOrder::new(&snapshot);
        let metrics = metrics();
        let rendered = super::super::compose(&snapshot, &order, super::super::Page::Local, 0, &metrics);
        for y in (0..HEIGHT).step_by(BAND_HEIGHT) {
            let mut band = Band::new(y);
            super::super::draw_page(&mut band, &metrics, &rendered);
            assert_eq!(band.pixels.len(), WIDTH * BAND_HEIGHT);
        }
    }
}
