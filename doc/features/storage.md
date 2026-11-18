# Session Storage

`FlashRing` stores completed sessions in two 4096-byte NOR flash sectors.

| Sector | Address |
| --- | --- |
| 0 | `0x3F0000` |
| 1 | `0x3F1000` |

Each sector contains 256 slots. Each slot is 16 bytes:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 4 | Start epoch |
| 4 | 4 | End epoch |
| 8 | 4 | Steps |
| 12 | 4 | Unused |

Fields use little-endian encoding. An erased start epoch of `0xFFFF_FFFF` marks an empty slot.

On startup, storage scans the start epochs, counts occupied slots, and places the write position at the first empty slot. Sessions are returned from oldest to newest.

When 511 sessions are stored, storage erases the sector containing the oldest sessions before writing the next session. This removes 256 sessions.

Flash read, write, and erase failures are logged. Operations continue after failure.
