/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Standard onboard `NeoPixel` runner shared by every Feather RP2040 node.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_rp::{
    peripherals::PIO0,
    pio_programs::ws2812::{Grb, PioWs2812},
};
use embassy_time::{Duration, Ticker, Timer};
use smart_leds::RGB8;

// Local Imports
use helios_mesh_core::platform::led::{Listener, Message};

/* === Constants === */

const TICK: Duration = Duration::from_millis(10);
const TRAFFIC_TICKS: u8 = 3;
const DITHER_THRESHOLDS: [u8; 8] = [0, 128, 32, 160, 192, 64, 224, 96];

/* === Macros === */

/* === Type Aliases === */

pub(crate) type StatusPixel = PioWs2812<'static, PIO0, 0, 1, Grb>;

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[embassy_executor::task]
pub(crate) async fn run(mut pixel: StatusPixel, mut signals: Listener) -> ! {
    // Boot self-test: two visible white flashes prove the runner, PIO, DMA,
    // and pixel are alive before any animation logic runs. A board that
    // never flashes white at power-on has this path wedged.
    for _ in 0..2 {
        pixel.write(&[RGB8::new(24, 24, 24)]).await;
        Timer::after_millis(150).await;
        pixel.write(&[RGB8::default()]).await;
        Timer::after_millis(150).await;
    }

    let mut phase = 0_u16;
    let mut frame = 0_u8;
    let mut traffic_ticks = 0_u8;
    let mut ticker = Ticker::every(TICK);

    loop {
        while let Some(message) = signals.try_next() {
            match message {
                Message::Communicating if traffic_ticks == 0 => traffic_ticks = TRAFFIC_TICKS,
                Message::Communicating => {}
                Message::Identify => {
                    identify_pattern(&mut pixel).await;
                }
            }
        }

        let color = if traffic_ticks > 0 {
            traffic_ticks -= 1;
            RGB8::new(1, 10, 8)
        } else {
            let brightness = breathing_brightness(phase, frame);
            RGB8::new(0, brightness / 5, brightness)
        };

        pixel.write(&[color]).await;
        phase = (phase + 1) & 0x01ff;
        frame = frame.wrapping_add(1);
        ticker.next().await;
    }
}

/* === Private Functions === */

const fn breathing_brightness(phase: u16, frame: u8) -> u8 {
    let triangle = if phase < 256 { phase } else { 511 - phase };
    let x = triangle as u32;
    let eased = ((3 * x * x * 255) - (2 * x * x * x)) / (255 * 255);
    let brightness_q8 = (2_u32 << 8) + eased * 30;
    let bytes = brightness_q8.to_le_bytes();
    let whole = bytes[1];
    let fraction = bytes[0];
    if fraction > DITHER_THRESHOLDS[(frame & 7) as usize] { whole + 1 } else { whole }
}

async fn identify_pattern(pixel: &mut StatusPixel) {
    for _ in 0..3 {
        pixel.write(&[RGB8::new(48, 0, 64)]).await;
        Timer::after_millis(140).await;
        pixel.write(&[RGB8::default()]).await;
        Timer::after_millis(110).await;
    }
}

/* === Tests === */
