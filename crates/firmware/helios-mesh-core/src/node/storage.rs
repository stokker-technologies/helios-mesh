/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Canonical node flash layout and typed durable-region access.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embedded_storage::nor_flash::NorFlash;
use serde::{Serialize, de::DeserializeOwned};

// Local Imports

/* === Constants === */

pub const SECTOR_SIZE: u32 = 4096;
const CONFIGURATION_SLOTS: usize = 2;
const CONFIGURATION_SECTORS: u32 = 2;
const CONFIGURATION_SIZE: u32 = SECTOR_SIZE * CONFIGURATION_SECTORS;
const DOCUMENT_BUFFER_SIZE: usize = 256;
const HEADER_SIZE: usize = 11;
const CHECKSUM_SIZE: usize = 4;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Storage<F> {
    flash: F,
}

pub struct Configuration<'a, F> {
    flash: &'a mut F,
}

#[derive(Clone, Copy)]
struct Record<D> {
    generation: u32,
    document: D,
}

/* === Enums === */

#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    #[error("configuration storage is outside flash bounds")]
    OutOfBounds,

    #[error("configuration document is invalid")]
    InvalidDocument,

    #[error("configuration document encoding failed")]
    Codec,

    #[error("configuration write verification failed")]
    Verification,

    // NorFlashError Requires Debug, Not core::error::Error. Keep The Typed Cause.
    #[error("flash device failed: {0:?}")]
    Device(E),
}

/* === Traits === */

pub trait Document: Copy + Eq + Serialize + DeserializeOwned {
    const MAGIC: u32;

    const SCHEMA: u8;

    fn is_valid(&self) -> bool;
}

/* === Implementations === */

impl Document for () {
    const MAGIC: u32 = 0;
    const SCHEMA: u8 = 0;

    fn is_valid(&self) -> bool {
        true
    }
}

impl<F> Storage<F>
where
    F: NorFlash,
{
    #[must_use]
    pub const fn new(flash: F) -> Self {
        Self { flash }
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.flash.capacity()
    }

    pub const fn configuration(&mut self) -> Configuration<'_, F> {
        Configuration { flash: &mut self.flash }
    }
}

impl<F> Configuration<'_, F>
where
    F: NorFlash,
{
    pub fn load<D>(&mut self) -> Result<Option<D>, Error<F::Error>>
    where
        D: Document,
    {
        if const { size_of::<D>() == 0 } {
            return Ok(None);
        }
        let records = [self.read::<D>(0)?, self.read::<D>(1)?];
        Ok(select(records).0.map(|record| record.document))
    }

    pub fn store<D>(&mut self, document: D) -> Result<bool, Error<F::Error>>
    where
        D: Document,
    {
        if const { size_of::<D>() == 0 } {
            return Ok(false);
        }
        if !document.is_valid() {
            return Err(Error::InvalidDocument);
        }

        let records = [self.read::<D>(0)?, self.read::<D>(1)?];
        let (current, slot) = select(records);
        if current.is_some_and(|record| record.document == document) {
            return Ok(false);
        }

        let record = Record { generation: current.map_or(1, |record| record.generation.wrapping_add(1).max(1)), document };
        let target = (slot + 1) % CONFIGURATION_SLOTS;
        let mut bytes = [0xFF; DOCUMENT_BUFFER_SIZE];
        let used = encode(record, &mut bytes)?;
        let offset = slot_offset(target)?;
        let start = self.start()?.checked_add(offset).ok_or(Error::OutOfBounds)?;
        self.flash.erase(start, start + SECTOR_SIZE).map_err(Error::Device)?;
        self.flash.write(start, &bytes[..used]).map_err(Error::Device)?;

        let Some(verified) = self.read::<D>(target)? else {
            return Err(Error::Verification);
        };
        if verified.generation != record.generation || verified.document != document {
            return Err(Error::Verification);
        }
        Ok(true)
    }

    fn read<D>(&mut self, slot: usize) -> Result<Option<Record<D>>, Error<F::Error>>
    where
        D: Document,
    {
        let mut bytes = [0; DOCUMENT_BUFFER_SIZE];
        let offset = slot_offset(slot)?;
        let start = self.start()?.checked_add(offset).ok_or(Error::OutOfBounds)?;
        self.flash.read(start, &mut bytes).map_err(Error::Device)?;
        Ok(decode(&bytes))
    }

    fn start(&self) -> Result<u32, Error<F::Error>> {
        u32::try_from(self.flash.capacity()).map_err(|_| Error::OutOfBounds)?.checked_sub(CONFIGURATION_SIZE).ok_or(Error::OutOfBounds)
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn select<D: Copy>(records: [Option<Record<D>>; CONFIGURATION_SLOTS]) -> (Option<Record<D>>, usize) {
    let slot = match records {
        [Some(first), Some(second)] if second.generation > first.generation => 1,
        [None, Some(_)] => 1,
        _ => 0,
    };
    (records[slot], slot)
}

fn slot_offset<E>(slot: usize) -> Result<u32, Error<E>> {
    u32::try_from(slot)
        .ok()
        .and_then(|slot| slot.checked_mul(SECTOR_SIZE))
        .filter(|offset| *offset < CONFIGURATION_SIZE)
        .ok_or(Error::OutOfBounds)
}

fn encode<D, E>(record: Record<D>, output: &mut [u8; DOCUMENT_BUFFER_SIZE]) -> Result<usize, Error<E>>
where
    D: Document,
{
    output[..4].copy_from_slice(&D::MAGIC.to_le_bytes());
    output[4] = D::SCHEMA;
    output[5..9].copy_from_slice(&record.generation.to_le_bytes());
    let payload_limit = output.len() - CHECKSUM_SIZE;
    let payload = postcard::to_slice(&record.document, &mut output[HEADER_SIZE..payload_limit]).map_err(|_| Error::Codec)?;
    let payload_length = u16::try_from(payload.len()).map_err(|_| Error::Codec)?;
    output[9..11].copy_from_slice(&payload_length.to_le_bytes());
    let payload_end = HEADER_SIZE + usize::from(payload_length);
    let checksum = checksum(&output[..payload_end]);
    let end = payload_end + CHECKSUM_SIZE;
    output[payload_end..end].copy_from_slice(&checksum.to_le_bytes());
    Ok(end)
}

fn decode<D>(input: &[u8; DOCUMENT_BUFFER_SIZE]) -> Option<Record<D>>
where
    D: Document,
{
    if u32::from_le_bytes(input[..4].try_into().ok()?) != D::MAGIC || input[4] != D::SCHEMA {
        return None;
    }
    let generation = u32::from_le_bytes(input[5..9].try_into().ok()?);
    let payload_length = usize::from(u16::from_le_bytes(input[9..11].try_into().ok()?));
    let payload_end = HEADER_SIZE.checked_add(payload_length)?;
    let end = payload_end.checked_add(CHECKSUM_SIZE)?;
    if end > input.len() {
        return None;
    }
    let expected = u32::from_le_bytes(input[payload_end..end].try_into().ok()?);
    if checksum(&input[..payload_end]) != expected {
        return None;
    }
    let document: D = postcard::from_bytes(&input[HEADER_SIZE..payload_end]).ok()?;
    document.is_valid().then_some(Record { generation, document })
}

const fn checksum(bytes: &[u8]) -> u32 {
    let mut value = 0x811C_9DC5_u32;
    let mut index = 0;
    while index < bytes.len() {
        value ^= bytes[index] as u32;
        value = value.wrapping_mul(0x0100_0193);
        index += 1;
    }
    value
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use embedded_storage::nor_flash::{ErrorType, NorFlashErrorKind, ReadNorFlash};
    use serde::{Deserialize, Serialize};

    use super::*;

    const CAPACITY: usize = 2 * CONFIGURATION_SIZE as usize;

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct Config {
        value: u32,
    }

    impl Document for Config {
        const MAGIC: u32 = 0x484E_4D54;
        const SCHEMA: u8 = 1;

        fn is_valid(&self) -> bool {
            self.value > 0
        }
    }

    struct Flash {
        bytes: [u8; CAPACITY],
    }

    impl ErrorType for Flash {
        type Error = NorFlashErrorKind;
    }

    impl ReadNorFlash for Flash {
        const READ_SIZE: usize = 1;

        fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
            let start = offset as usize;
            let end = start + bytes.len();
            bytes.copy_from_slice(self.bytes.get(start..end).ok_or(NorFlashErrorKind::OutOfBounds)?);
            Ok(())
        }

        fn capacity(&self) -> usize {
            self.bytes.len()
        }
    }

    impl NorFlash for Flash {
        const WRITE_SIZE: usize = 1;
        const ERASE_SIZE: usize = SECTOR_SIZE as usize;

        fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
            let start = offset as usize;
            let end = start + bytes.len();
            self.bytes.get_mut(start..end).ok_or(NorFlashErrorKind::OutOfBounds)?.copy_from_slice(bytes);
            Ok(())
        }

        fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
            self.bytes.get_mut(from as usize..to as usize).ok_or(NorFlashErrorKind::OutOfBounds)?.fill(0xFF);
            Ok(())
        }
    }

    #[test]
    fn configuration_round_documents() {
        let mut storage = Storage::new(Flash { bytes: [0xFF; CAPACITY] });
        let config = Config { value: 7 };

        assert_eq!(storage.configuration().load::<Config>().unwrap(), None);
        assert!(storage.configuration().store(config).unwrap());
        assert_eq!(storage.configuration().load::<Config>().unwrap(), Some(config));
        assert!(!storage.configuration().store(config).unwrap());

        let updated = Config { value: 8 };
        assert!(storage.configuration().store(updated).unwrap());
        assert_eq!(storage.configuration().load::<Config>().unwrap(), Some(updated));
    }

    #[test]
    fn configuration_rejects_documents() {
        let mut storage = Storage::new(Flash { bytes: [0xFF; CAPACITY] });

        assert!(matches!(storage.configuration().store(Config { value: 0 }), Err(Error::InvalidDocument)));
    }

    #[test]
    fn configuration_ignores_records() {
        let mut storage = Storage::new(Flash { bytes: [0xFF; CAPACITY] });
        storage.configuration().store(Config { value: 7 }).unwrap();
        storage.configuration().store(Config { value: 8 }).unwrap();
        let newest_slot = CAPACITY - CONFIGURATION_SIZE as usize;
        storage.flash.bytes[newest_slot] ^= 1;

        assert_eq!(storage.configuration().load::<Config>().unwrap(), Some(Config { value: 7 }));
    }

    #[test]
    fn zero_sized_device() {
        struct FailingFlash;

        impl ErrorType for FailingFlash {
            type Error = NorFlashErrorKind;
        }

        impl ReadNorFlash for FailingFlash {
            const READ_SIZE: usize = 1;

            fn read(&mut self, _offset: u32, _bytes: &mut [u8]) -> Result<(), Self::Error> {
                panic!("zero-sized document read the flash device");
            }

            fn capacity(&self) -> usize {
                0
            }
        }

        impl NorFlash for FailingFlash {
            const WRITE_SIZE: usize = 1;
            const ERASE_SIZE: usize = SECTOR_SIZE as usize;

            fn write(&mut self, _offset: u32, _bytes: &[u8]) -> Result<(), Self::Error> {
                panic!("zero-sized document wrote the flash device");
            }

            fn erase(&mut self, _from: u32, _to: u32) -> Result<(), Self::Error> {
                panic!("zero-sized document erased the flash device");
            }
        }

        let mut storage = Storage::new(FailingFlash);
        assert_eq!(storage.configuration().load::<()>().unwrap(), None);
        assert!(!storage.configuration().store(()).unwrap());
    }
}
