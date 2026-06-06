use defmt::{error, info};
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use esp_hal::peripherals::FLASH as Flash;
use esp_storage::FlashStorage;

const SECTOR0_ADDR: u32 = 0x3F0000;
const SECTOR1_ADDR: u32 = 0x3F1000;
const SECTOR_SIZE: u32 = 4096;
const SLOT_SIZE: u32 = 16;
const SLOTS_PER_SECTOR: u16 = (SECTOR_SIZE / SLOT_SIZE) as u16; // 256
pub const SLOT_COUNT: u16 = SLOTS_PER_SECTOR * 2; // 512

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum SlotFlags {
    Empty = 0xFFFF_FFFF,
    Unsynced = 0xFFFF_FFFE,
    Synced = 0xFFFF_FFFC,
}

impl SlotFlags {
    pub fn from_u32(v: u32) -> Self {
        match v {
            x if x == Self::Empty as u32 => Self::Empty,
            x if x == Self::Unsynced as u32 => Self::Unsynced,
            x if x == Self::Synced as u32 => Self::Synced,
            _ => Self::Empty, // TODO: handle corruption
        }
    }

    pub fn to_bytes(self) -> [u8; 4] {
        (self as u32).to_le_bytes()
    }
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
        info!("flash capacity: {}B", self.flash.capacity());

        let mut first_empty = None;
        let mut count = 0u16;

        for i in 0..SLOT_COUNT {
            let flags = self.slot_flags(i);
            if flags != SlotFlags::Empty {
                count += 1;
            } else if first_empty.is_none() {
                first_empty = Some(i);
            }
        }

        self.head = first_empty.unwrap_or(0);
        self.count = count;

        info!(
            "loaded {} sessions from flash (head={})",
            self.count, self.head
        );
    }

    pub fn is_occupied(&mut self, index: u16) -> bool {
        self.slot_flags(index) != SlotFlags::Empty
    }

    pub fn slot_at(&mut self, index: u16) -> (u32, u32, u32, u32) {
        let offset = Self::slot_offset(index);
        let mut buf = [0u8; 16];
        if let Err(e) = self.flash.read(offset, &mut buf) {
            error!(
                "slot_at[{}] read failed: {}",
                index,
                defmt::Debug2Format(&e)
            );
        }
        (
            u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            u32::from_le_bytes(buf[12..16].try_into().unwrap()),
        )
    }

    pub fn write_session(&mut self, start_epoch: u32, end_epoch: u32, steps: u32) {
        if self.head >= SLOT_COUNT {
            // Both sectors full — erase the oldest (sector 0), keep sector 1 as archive
            if let Err(e) = self.flash.erase(SECTOR0_ADDR, SECTOR0_ADDR + SECTOR_SIZE) {
                error!(
                    "write_session: erase sector 0 failed: {}",
                    defmt::Debug2Format(&e)
                );
            } else {
                info!("write_session: sector 0 erased, wrapping head");
            }
            self.head = 0;
            self.count = SLOTS_PER_SECTOR;
        } else if self.head > 0 && self.head.is_multiple_of(SLOTS_PER_SECTOR) {
            // Crossing from sector 0 into sector 1 — erase sector 1 for clean slate
            if let Err(e) = self.flash.erase(SECTOR1_ADDR, SECTOR1_ADDR + SECTOR_SIZE) {
                error!(
                    "write_session: erase sector 1 failed: {}",
                    defmt::Debug2Format(&e)
                );
            } else {
                info!("write_session: sector 1 erased, crossing boundary");
            }
        }

        if self.is_occupied(self.head) {
            error!("head={} unexpectedly occupied, full erase", self.head);
            let _ = self.flash.erase(SECTOR0_ADDR, SECTOR0_ADDR + SECTOR_SIZE);
            let _ = self.flash.erase(SECTOR1_ADDR, SECTOR1_ADDR + SECTOR_SIZE);
            self.head = 0;
            self.count = 0;
        }

        let index = self.head;
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&start_epoch.to_le_bytes());
        buf[4..8].copy_from_slice(&end_epoch.to_le_bytes());
        buf[8..12].copy_from_slice(&steps.to_le_bytes());
        buf[12..16].copy_from_slice(&SlotFlags::Unsynced.to_bytes());

        let offset = Self::slot_offset(index);
        if let Err(e) = self.flash.write(offset, &buf) {
            info!("write failed: {}", defmt::Debug2Format(&e));
        }

        self.head += 1;
        self.count += 1;
    }

    pub fn mark_synced(&mut self, index: u16) {
        let offset = Self::slot_offset(index) + 12;
        if let Err(e) = self.flash.write(offset, &SlotFlags::Synced.to_bytes()) {
            info!("mark_synced failed: {}", defmt::Debug2Format(&e));
        } else {
            info!("session {} marked synced", index);
        }
    }

    pub fn is_synced(&mut self, index: u16) -> bool {
        self.slot_flags(index) == SlotFlags::Synced
    }

    fn slot_flags(&mut self, index: u16) -> SlotFlags {
        let offset = Self::slot_offset(index) + 12;
        let mut buf = [0u8; 4];
        if let Err(e) = self.flash.read(offset, &mut buf) {
            error!(
                "slot_flags[{}] read failed: {:?}",
                index,
                defmt::Debug2Format(&e)
            );
            return SlotFlags::Empty; // defensive
        }
        SlotFlags::from_u32(u32::from_le_bytes(buf))
    }

    pub fn count(&self) -> u16 {
        self.count
    }

    pub fn head(&self) -> u16 {
        self.head
    }

    pub fn erase_all(&mut self) {
        if let Err(e) = self.flash.erase(SECTOR0_ADDR, SECTOR0_ADDR + SECTOR_SIZE) {
            error!("erase_all sector 0 failed: {}", defmt::Debug2Format(&e));
        }
        if let Err(e) = self.flash.erase(SECTOR1_ADDR, SECTOR1_ADDR + SECTOR_SIZE) {
            error!("erase_all sector 1 failed: {}", defmt::Debug2Format(&e));
        }
        self.head = 0;
        self.count = 0;
    }

    pub fn unsynced_indices(&mut self) -> heapless::Vec<u16, { SLOT_COUNT as usize }> {
        let mut result = heapless::Vec::new();
        for i in 0..SLOT_COUNT {
            if self.slot_flags(i) == SlotFlags::Unsynced && result.push(i).is_err() {
                error!("unsynced_indices push failed (vec full)");
            }
        }
        result
    }
}
