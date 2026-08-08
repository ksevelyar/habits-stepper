# Session Storage

## Overview

Permanent storage persists completed session records in NOR flash and serves them back after a reboot. It exposes a narrow storage contract:

* Persists completed session records.
* Reconstructs its write position after restart.
* Returns records in chronological storage order.
* Reports how many records were removed during rotation.

```
append record:
* write payload and flags into the next slot
* erase the oldest sector when the ring is nearly full
* return number of evicted records

startup:
* scan all slots
* count occupied slots
* place the write position at the first empty slot
* expose records from oldest to newest
```

## Layout

Storage uses two 4096-byte sectors in NOR flash:

| Region | Address |
| --- | --- |
| Sector 0 | `0x3F0000` |
| Sector 1 | `0x3F1000` |

Each sector holds 256 slots of 16 bytes, for a total of 512 slots forming a single ring buffer.

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 4 | start epoch |
| 4 | 4 | end epoch |
| 8 | 4 | steps |
| 12 | 4 | slot flags |

All fields are little-endian.

## Slot flags

| Flags | Encoding |
| --- | --- |
| Empty | `0xFFFF_FFFF` |
| Occupied | `0xFFFF_FFFE` |

Any other value is treated as Empty; corruption is not repaired.

## Ring buffer

Records are appended at the write position, which then advances and wraps from the last slot to the first. The write position always points to the first empty slot after the stored records.

When 511 records are stored and another append arrives:

* Erase the sector that does not contain the write position; it holds the oldest records.
* Drop the stored count by 256.
* Write the new record.

The erased sector is erased before the write, so both sectors are never full simultaneously. Retention stays between 256 and 511 records.

On startup the slot flags are scanned to reconstruct the state:

* The stored count is the number of Occupied slots.
* The write position is the first Empty slot.
* Records are exposed from oldest to newest.

## Commit

NOR flash can only turn bits from 1 to 0; erasing restores all bits to 1. A record is written as one 16-byte program that includes the slot flags, so a record only counts once its flags read as Occupied. An interrupted write leaves a value that is not a valid flag and is read as Empty; the record is skipped. Corruption is not distinguished from an interrupted write.

## Failures

Flash failures are logged and do not abort the operation:

* Read: a slot could not be read; it is treated as Empty.
* Write: a record could not be written.
* Erase: a sector could not be erased during rotation.
