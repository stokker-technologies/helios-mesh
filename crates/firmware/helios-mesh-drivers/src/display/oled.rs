/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Adafruit 128 by 64 SH1107 OLED `FeatherWing` driver.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::convert::Infallible;

// Third Party Imports
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    Pixel,
    geometry::{OriginDimensions, Size},
    mono_font::ascii::FONT_5X7,
    pixelcolor::BinaryColor,
    prelude::DrawTarget,
};
use embedded_hal::digital::InputPin;
use embedded_hal_async::i2c::I2c;

// Local Imports
use super::{Controls, Metrics, Palette, Panel, SnapshotReceiver};

/* === Constants === */

pub const ADDRESS: u8 = 0x3C;
pub const MAXIMUM_I2C_FREQUENCY_HZ: u32 = 400_000;
const WIDTH: usize = 128;
const HEIGHT: usize = 64;
const PHYSICAL_WIDTH: usize = 64;
const PAGES: usize = 16;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Frame {
    buffer: [u8; PHYSICAL_WIDTH * PAGES],
}

pub struct Oled<I2C, A, C> {
    i2c: I2C,
    button_a: A,
    button_c: C,
    frame: Frame,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Frame {
    const fn new() -> Self {
        Self { buffer: [0; PHYSICAL_WIDTH * PAGES] }
    }
}

impl OriginDimensions for Frame {
    fn size(&self) -> Size {
        Size::new(128, 64)
    }
}

impl DrawTarget for Frame {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x < 0 || point.x >= 128 || point.y < 0 || point.y >= 64 {
                continue;
            }
            let logical_x = usize::try_from(point.x).unwrap_or_default();
            let logical_y = usize::try_from(point.y).unwrap_or_default();
            let physical_x = 63 - logical_y;
            let physical_y = logical_x;
            let offset = (physical_y / 8) * PHYSICAL_WIDTH + physical_x;
            let bit = 1 << (physical_y % 8);
            match color {
                BinaryColor::On => self.buffer[offset] |= bit,
                BinaryColor::Off => self.buffer[offset] &= !bit,
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.buffer.fill(if color == BinaryColor::On { 0xFF } else { 0x00 });
        Ok(())
    }
}

impl<I2C, A, C> Oled<I2C, A, C>
where
    I2C: I2c,
    A: InputPin,
    C: InputPin,
{
    pub const fn new(i2c: I2C, button_a: A, button_c: C) -> Self {
        Self { i2c, button_a, button_c, frame: Frame::new() }
    }

    pub async fn run(mut self, snapshots: SnapshotReceiver) -> ! {
        if initialize(&mut self.i2c).await.is_err() {
            defmt::error!("OLED initialization failed; display engine parked");
            core::future::pending::<()>().await;
        }
        super::run(self, snapshots).await
    }
}

impl<I2C, A, C> Panel for Oled<I2C, A, C>
where
    I2C: I2c,
    A: InputPin,
    C: InputPin,
{
    type Color = BinaryColor;
    type Band = Frame;
    type Error = I2C::Error;

    fn metrics(&self) -> Metrics<BinaryColor> {
        Metrics {
            width: WIDTH,
            height: HEIGHT,
            band_height: HEIGHT,
            title_font: &FONT_5X7,
            body_font: &FONT_5X7,
            palette: Palette { background: BinaryColor::Off, foreground: BinaryColor::On, muted: BinaryColor::On, accent: BinaryColor::On },
            margin: 1,
            header_height: 8,
            rows_start: 8,
            row_pitch: 8,
            footer_height: 8,
            separators: false,
            refresh_interval: Duration::from_millis(0),
        }
    }

    fn begin_band(&mut self, _y: usize) -> &mut Frame {
        let _ = self.frame.clear(BinaryColor::Off);
        &mut self.frame
    }

    async fn present_band(&mut self, _y: usize) -> Result<(), Self::Error> {
        upload(&mut self.i2c, &self.frame).await
    }

    fn sample(&mut self) -> impl core::future::Future<Output = Controls> {
        core::future::poll_fn(|_| {
            core::task::Poll::Ready(Controls {
                previous: self.button_a.is_low().unwrap_or(false),
                next: self.button_c.is_low().unwrap_or(false),
            })
        })
    }
}

/* === Public Functions === */

pub async fn detect<I: I2c>(i2c: &mut I) -> bool {
    i2c.write(ADDRESS, &[0x00, 0xAE]).await.is_ok()
}

/* === Private Functions === */

async fn initialize<I: I2c>(i2c: &mut I) -> Result<(), I::Error> {
    i2c.write(
        ADDRESS,
        &[
            0x00, 0xAE, 0xD5, 0x51, 0x20, 0x81, 0x4F, 0xAD, 0x8A, 0xA0, 0xC0, 0xDC, 0x00, 0xD3, 0x60, 0xD9, 0x22, 0xDB, 0x35, 0xA8, 0x3F,
            0xA4, 0xA6,
        ],
    )
    .await?;
    Timer::after_millis(100).await;
    i2c.write(ADDRESS, &[0x00, 0xAF]).await
}

async fn upload<I: I2c>(i2c: &mut I, frame: &Frame) -> Result<(), I::Error> {
    for page in 0..PAGES {
        i2c.write(ADDRESS, &[0x00, 0xB0 + u8::try_from(page).unwrap_or_default(), 0x10, 0x00]).await?;
        let mut data = [0; PHYSICAL_WIDTH + 1];
        data[0] = 0x40;
        let start = page * PHYSICAL_WIDTH;
        data[1..].copy_from_slice(&frame.buffer[start..start + PHYSICAL_WIDTH]);
        i2c.write(ADDRESS, &data).await?;
    }
    Ok(())
}

/* === Tests === */
