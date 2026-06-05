use defmt::{info, error};
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use esp_hal::peripherals::FLASH as Flash;
use esp_storage::FlashStorage;

const SECTOR_ADDR: u32 = 0x3F0000;
const SECTOR_SIZE: u32 = 4096;
const SLOT_SIZE: u32 = 16;
pub const SLOT_COUNT: u8 = (SECTOR_SIZE / SLOT_SIZE - 1) as u8; // 255: ring with wraparound

const FLAGS_EMPTY: u32 = 0xFFFF_FFFF;
const FLAGS_UNSYNCED: u32 = 0xFFFF_FFFE;
const FLAGS_SYNCED: u32 = 0xFFFF_FFFC;

pub struct FlashRing<'a> {
    flash: FlashStorage<'a>,
    head: u8,
    count: u8,
}

impl<'a> FlashRing<'a> {
    pub fn new(flash: Flash<'a>) -> Self {
        Self {
            flash: FlashStorage::new(flash),
            head: 0,
            count: 0,
        }
    }

    fn slot_offset(index: u8) -> u32 {
        SECTOR_ADDR + index as u32 * SLOT_SIZE
    }

    pub fn init(&mut self) {
        info!("flash capacity: {}B", self.flash.capacity());

        let mut first_empty = 255u8;
        let mut count = 0u8;

        for i in 0..SLOT_COUNT {
            let flags = self.slot_flags(i);
            if flags != FLAGS_EMPTY {
                if count == 0 {
                    info!("slot[{}] flags={:x}", i, flags);
                }
                count += 1;
            } else if first_empty == 255 {
                first_empty = i;
            }
        }

        self.head = if first_empty == 255 { 0 } else { first_empty };
        self.count = count;

        info!("loaded {} sessions from flash (head={})", count, self.head);
    }

    pub fn is_occupied(&mut self, index: u8) -> bool {
        self.slot_flags(index) != FLAGS_EMPTY
    }

    pub fn slot_at(&mut self, index: u8) -> (u32, u32, u32, u32) {
        let offset = Self::slot_offset(index);
        let mut buf = [0u8; 16];
        if let Err(e) = self.flash.read(offset, &mut buf) {
            error!("slot_at[{}] read failed: {}", index, defmt::Debug2Format(&e));
        }
        (
            u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            u32::from_le_bytes(buf[12..16].try_into().unwrap()),
        )
    }

    pub fn write_session(&mut self, start_epoch: u32, end_epoch: u32, steps: u32) {
        if self.head >= SLOT_COUNT || self.is_occupied(self.head) {
            if let Err(e) = self.flash.erase(SECTOR_ADDR, SECTOR_ADDR + SECTOR_SIZE) {
                info!("erase failed: {}", defmt::Debug2Format(&e));
            } else {
                info!("sector erased at 0x{:x}", SECTOR_ADDR);
            }
            self.head = 0;
            self.count = 0;
        }

        let index = self.head;
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&start_epoch.to_le_bytes());
        buf[4..8].copy_from_slice(&end_epoch.to_le_bytes());
        buf[8..12].copy_from_slice(&steps.to_le_bytes());
        buf[12..16].copy_from_slice(&FLAGS_UNSYNCED.to_le_bytes());

        let offset = Self::slot_offset(index);
        if let Err(e) = self.flash.write(offset, &buf) {
            info!("write failed: {}", defmt::Debug2Format(&e));
        } else {
            let mut verify = [0u8; 4];
            if let Err(e) = self.flash.read(offset + 12, &mut verify) {
                error!("write_session verify read failed: {}", defmt::Debug2Format(&e));
            }
            let written = u32::from_le_bytes(verify);
            info!(
                "stored at slot[{}] offset=0x{:x} flags=0x{:x}",
                index, offset, written
            );
        }

        self.head += 1;
        self.count += 1;
    }

    pub fn mark_synced(&mut self, index: u8) {
        let offset = Self::slot_offset(index) + 12;
        if let Err(e) = self.flash.write(offset, &FLAGS_SYNCED.to_le_bytes()) {
            info!("mark_synced failed: {}", defmt::Debug2Format(&e));
        } else {
            info!("session {} marked synced", index);
        }
    }

    pub fn is_synced(&mut self, index: u8) -> bool {
        self.slot_flags(index) == FLAGS_SYNCED
    }

    pub fn slot_flags(&mut self, index: u8) -> u32 {
        let offset = Self::slot_offset(index) + 12;
        let mut buf = [0u8; 4];
        if let Err(e) = self.flash.read(offset, &mut buf) {
            error!("slot_flags[{}] read failed: {}", index, defmt::Debug2Format(&e));
        }
        u32::from_le_bytes(buf)
    }

    pub fn head(&self) -> u8 {
        self.head
    }

    pub fn count(&self) -> u8 {
        self.count
    }

    pub fn unsynced_indices(&mut self) -> heapless::Vec<u8, { SLOT_COUNT as usize }> {
        let mut result = heapless::Vec::new();
        for i in 0..SLOT_COUNT {
            if self.slot_flags(i) == FLAGS_UNSYNCED {
                if result.push(i).is_err() {
                    error!("unsynced_indices push failed (vec full)");
                }
            }
        }
        result
    }
}
