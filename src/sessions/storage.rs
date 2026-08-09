use defmt::{error, info};
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use esp_hal::peripherals::FLASH as Flash;
use esp_storage::FlashStorage;

const SECTOR0_ADDR: u32 = 0x3F0000;
const SECTOR1_ADDR: u32 = 0x3F1000;
const SECTOR_SIZE: u32 = 4096;
const SLOT_SIZE: u32 = 16;
const SLOTS_PER_SECTOR: u16 = (SECTOR_SIZE / SLOT_SIZE) as u16;
pub const SLOT_COUNT: u16 = SLOTS_PER_SECTOR * 2;

#[derive(Clone, Copy)]
pub struct SessionRecord {
    pub start_epoch: u32,
    pub end_epoch: u32,
    pub steps: u32,
}

pub struct FlashRing<'a> {
    flash: FlashStorage<'a>,
    head: u16,
    count: u16,
}

impl<'a> FlashRing<'a> {
    pub fn new(flash: Flash<'a>) -> Self {
        Self {
            flash: FlashStorage::new(flash),
            head: 0,
            count: 0,
        }
    }

    fn slot_offset(index: u16) -> u32 {
        SECTOR0_ADDR + index as u32 * SLOT_SIZE
    }

    pub fn init(&mut self) {
        info!("storage: flash capacity: {}B", self.flash.capacity());

        let mut first_empty = None;
        let mut count = 0u16;

        for index in 0..SLOT_COUNT {
            if !self.slot_is_empty(index) {
                count += 1;
            } else if first_empty.is_none() {
                first_empty = Some(index);
            }
        }

        self.head = first_empty.unwrap_or(0);
        self.count = count;

        info!(
            "storage: loaded {} sessions from flash (head={})",
            self.count, self.head
        );
    }

    fn slot_is_empty(&mut self, index: u16) -> bool {
        let offset = Self::slot_offset(index);
        let mut buf = [0u8; 4];
        if let Err(e) = self.flash.read(offset, &mut buf) {
            error!(
                "storage: slot[{}] read failed: {:?}",
                index,
                defmt::Debug2Format(&e)
            );
            return true;
        }
        u32::from_le_bytes(buf) == u32::MAX
    }

    fn slot_at(&mut self, index: u16) -> SessionRecord {
        let offset = Self::slot_offset(index);
        let mut buf = [0u8; 16];
        if let Err(e) = self.flash.read(offset, &mut buf) {
            error!(
                "storage: slot_at[{}] read failed: {}",
                index,
                defmt::Debug2Format(&e)
            );
        }
        SessionRecord {
            start_epoch: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            end_epoch: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            steps: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
        }
    }

    pub fn write_session(&mut self, start_epoch: u32, end_epoch: u32, steps: u32) -> u16 {
        let evicted = if self.count == SLOT_COUNT - 1 {
            let (start, end) = if self.head < SLOTS_PER_SECTOR {
                (SECTOR1_ADDR, SECTOR1_ADDR + SECTOR_SIZE)
            } else {
                (SECTOR0_ADDR, SECTOR0_ADDR + SECTOR_SIZE)
            };

            if let Err(e) = self.flash.erase(start, end) {
                error!("storage: archive erase failed: {}", defmt::Debug2Format(&e));
            } else {
                info!("storage: archive erased, rotating sectors");
            }

            self.count -= SLOTS_PER_SECTOR;
            SLOTS_PER_SECTOR
        } else {
            0
        };

        let index = self.head;
        let mut buf = [0xFFu8; 16];
        buf[0..4].copy_from_slice(&start_epoch.to_le_bytes());
        buf[4..8].copy_from_slice(&end_epoch.to_le_bytes());
        buf[8..12].copy_from_slice(&steps.to_le_bytes());

        let offset = Self::slot_offset(index);
        if let Err(e) = self.flash.write(offset, &buf) {
            info!("storage: write failed: {}", defmt::Debug2Format(&e));
        }

        self.head = match index {
            index if index == SLOT_COUNT - 1 => 0,
            _ => index + 1,
        };
        self.count += 1;
        evicted
    }

    pub fn sessions(&mut self) -> impl Iterator<Item = SessionRecord> + '_ {
        let count = self.count;
        let archive_is_sector1 = self.head < SLOTS_PER_SECTOR && count >= SLOTS_PER_SECTOR;
        let mut logical_index = 0;

        core::iter::from_fn(move || {
            if logical_index >= count {
                return None;
            }

            let index = if archive_is_sector1 {
                (logical_index + SLOTS_PER_SECTOR) % SLOT_COUNT
            } else {
                logical_index
            };
            logical_index += 1;
            Some(self.slot_at(index))
        })
    }

    pub fn count(&self) -> u16 {
        self.count
    }

    pub fn erase_all(&mut self) {
        if let Err(e) = self.flash.erase(SECTOR0_ADDR, SECTOR0_ADDR + SECTOR_SIZE) {
            error!(
                "storage: erase_all sector 0 failed: {}",
                defmt::Debug2Format(&e)
            );
        }
        if let Err(e) = self.flash.erase(SECTOR1_ADDR, SECTOR1_ADDR + SECTOR_SIZE) {
            error!(
                "storage: erase_all sector 1 failed: {}",
                defmt::Debug2Format(&e)
            );
        }
        self.head = 0;
        self.count = 0;
    }
}
