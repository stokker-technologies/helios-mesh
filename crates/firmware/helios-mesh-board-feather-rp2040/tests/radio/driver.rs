/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Exercise The Published SX1276 Driver Through The Feather Board Interface.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{cell::RefCell, rc::Rc};

// Third Party Imports
use embedded_hal::spi::{Error, ErrorKind, ErrorType};
use embedded_hal_async::spi::{Operation, SpiDevice};
use lora_phy::{
    LoRa, RxMode,
    mod_params::{Bandwidth, CodingRate, RadioError, SpreadingFactor},
    sx127x::{Config, Sx127x, Sx1276},
};

// Local Imports
use super::{Context, Delay, Future, Input, Interface, Reset, Signal, Waker, block_on, pin};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

struct Registers {
    values: [u8; 128],
    fifo: [u8; 256],
    pointer: usize,
    signals: [Rc<Signal>; 2],
    reception: Reception,
    transmitted: Vec<Vec<u8>>,
    fail_next: bool,
}

struct Bus(Rc<RefCell<Registers>>);

/* === Enums === */

#[derive(Debug)]
enum BusError {
    Fault,
}

enum Reception {
    Waiting,
    Timeout,
    Packet(Vec<u8>),
}

/* === Traits === */

/* === Implementations === */

impl Error for BusError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl ErrorType for Bus {
    type Error = BusError;
}

impl SpiDevice for Bus {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> impl core::future::Future<Output = Result<(), Self::Error>> {
        let mut registers = self.0.borrow_mut();
        if std::mem::take(&mut registers.fail_next) {
            return core::future::ready(Err(BusError::Fault));
        }
        let mut command: Option<(u8, bool)> = None;
        for operation in operations {
            match operation {
                Operation::Write(bytes) => {
                    for &byte in *bytes {
                        if let Some((address, writing)) = command {
                            assert!(writing, "payload write requires a write command");
                            registers.write(address, byte);
                            command = Some((if address == 0 { 0 } else { address + 1 }, writing));
                        } else {
                            command = Some((byte & 0x7f, byte & 0x80 != 0));
                        }
                    }
                }
                Operation::Read(bytes) => {
                    let (mut address, writing) = command.expect("read requires a command");
                    assert!(!writing, "read requires a read command");
                    for byte in bytes.iter_mut() {
                        *byte = registers.read(address);
                        if address != 0 {
                            address += 1;
                        }
                    }
                    command = Some((address, writing));
                }
                _ => return core::future::ready(Err(BusError::Fault)),
            }
        }
        core::future::ready(Ok(()))
    }
}

impl Registers {
    fn new() -> Self {
        let mut values = [0; 128];
        values[0x42] = 0x12;
        Self {
            values,
            fifo: [0; 256],
            pointer: 0,
            signals: [Rc::new(Signal::default()), Rc::new(Signal::default())],
            reception: Reception::Waiting,
            transmitted: Vec::new(),
            fail_next: false,
        }
    }

    fn read(&mut self, address: u8) -> u8 {
        if address == 0 {
            let byte = self.fifo[self.pointer];
            self.pointer = (self.pointer + 1) % 256;
            byte
        } else {
            self.values[usize::from(address)]
        }
    }

    fn write(&mut self, address: u8, value: u8) {
        match address {
            0 => {
                self.fifo[self.pointer] = value;
                self.pointer = (self.pointer + 1) % 256;
            }
            0x0d => {
                self.pointer = usize::from(value);
                self.values[0x0d] = value;
            }
            0x12 => self.values[0x12] &= !value,
            _ => self.values[usize::from(address)] = value,
        }
        if address == 1 {
            match value & 7 {
                3 => {
                    self.transmitted.push(self.fifo[..usize::from(self.values[0x22])].to_vec());
                    self.values[0x12] |= 0x08;
                }
                5 | 6 => match &self.reception {
                    Reception::Timeout => self.values[0x12] |= 0x80,
                    Reception::Packet(bytes) => {
                        self.fifo[..bytes.len()].copy_from_slice(bytes);
                        self.values[0x10] = 0;
                        self.values[0x13] = u8::try_from(bytes.len()).unwrap();
                        self.values[0x12] |= 0x40;
                    }
                    Reception::Waiting => {}
                },
                _ => {}
            }
        }
        let pending = self.values[0x12] & !self.values[0x11];
        let mapping = self.values[0x40];
        let dio0 = match mapping >> 6 {
            0 => pending & 0x40 != 0,
            1 => pending & 0x08 != 0,
            _ => false,
        };
        let dio1 = (mapping >> 4).trailing_zeros() >= 2 && pending & 0x80 != 0;
        for (signal, high) in self.signals.iter().zip([dio0, dio1]) {
            if high {
                signal.trigger();
            } else {
                signal.high.set(false);
            }
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[test]
fn radio_recovery() {
    let registers = Rc::new(RefCell::new(Registers::new()));
    let signals = registers.borrow().signals.clone();
    let reset = Rc::new(RefCell::new(Vec::new()));
    let interface = Interface::new(Reset(reset.clone()), Input(signals[0].clone()), Input(signals[1].clone()));
    let driver = Sx127x::new(Bus(registers.clone()), interface, Config { chip: Sx1276, tcxo_used: false, tx_boost: true, rx_boost: false });
    let mut radio = block_on(LoRa::new(driver, false, Delay::default())).unwrap();
    let modulation = radio.create_modulation_params(SpreadingFactor::_7, Bandwidth::_250KHz, CodingRate::_4_5, 869_525_000).unwrap();
    let mut tx = radio.create_tx_packet_params(8, false, true, false, &modulation).unwrap();
    let rx = radio.create_rx_packet_params(8, false, 255, true, false, &modulation).unwrap();
    let mut buffer = [0; 255];

    assert_eq!(*reset.borrow(), [false, true]);
    assert_eq!(registers.borrow().values[0x39], 0x12);
    block_on(radio.prepare_for_tx(&modulation, &mut tx, 10, b"first")).unwrap();
    block_on(radio.tx()).unwrap();
    assert_eq!(registers.borrow().transmitted, [b"first".to_vec()]);

    registers.borrow_mut().reception = Reception::Timeout;
    block_on(radio.prepare_for_rx(RxMode::Single(8), &modulation, &rx)).unwrap();
    assert!(matches!(block_on(radio.rx(&rx, &mut buffer)), Err(RadioError::ReceiveTimeout)));
    assert_eq!(registers.borrow().values[1] & 7, 1);
    assert!(signals[0].cancelled.get() > 0);

    registers.borrow_mut().reception = Reception::Packet(b"reply".to_vec());
    block_on(radio.prepare_for_rx(RxMode::Single(8), &modulation, &rx)).unwrap();
    let (length, _) = block_on(radio.rx(&rx, &mut buffer)).unwrap();
    assert_eq!(&buffer[..usize::from(length)], b"reply");

    registers.borrow_mut().reception = Reception::Waiting;
    block_on(radio.prepare_for_rx(RxMode::Single(8), &modulation, &rx)).unwrap();
    {
        let mut pending = pin!(radio.rx(&rx, &mut buffer));
        assert!(pending.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
    }
    assert!(!signals[0].high.get());
    assert!(!signals[1].high.get());
    block_on(radio.prepare_for_tx(&modulation, &mut tx, 10, b"after cancel")).unwrap();
    registers.borrow_mut().fail_next = true;
    assert!(matches!(block_on(radio.tx()), Err(RadioError::SPI)));
    block_on(radio.prepare_for_tx(&modulation, &mut tx, 10, b"after fault")).unwrap();
    block_on(radio.tx()).unwrap();
    assert_eq!(registers.borrow().transmitted.last().unwrap(), b"after fault");

    registers.borrow_mut().reception = Reception::Packet(b"recovered".to_vec());
    block_on(radio.prepare_for_rx(RxMode::Single(8), &modulation, &rx)).unwrap();
    let (length, _) = block_on(radio.rx(&rx, &mut buffer)).unwrap();
    assert_eq!(&buffer[..usize::from(length)], b"recovered");
}
