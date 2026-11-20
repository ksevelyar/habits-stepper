# Permanent Storage

## Requirements
* Completed sessions survive device restarts.
* When storage needs space for a new session, the device discards the oldest stored sessions and retains the newer sessions.
* The device minimizes NOR flash wear when storing sessions.

## User scenarios

### 1. User sees saved activity after a restart
1. The user recorded 30 minutes and 300 steps last week and 20 minutes and 200 steps this week.
2. The device restarts.
3. The display shows 20 minutes and 200 steps.
4. The user presses the history button.
5. The display shows 30 minutes and 300 steps.

### 2. User sees recent activity after recording 512 sessions
1. The user has recorded 511 sessions last week. The oldest 256 sessions contain 256 steps and the newer 255 contain 510 steps.
2. The user checks history and sees 766 steps.
3. The user records another session of 5 minutes and 50 steps.
4. The display shows 5 minutes and 50 steps. History shows 510 steps.
5. The device restarts. The same totals are displayed.
6. The user records another session of 1 minute and 10 steps.
7. The display shows 6 minutes and 60 steps. History still shows 510 steps.

### 3. User sees recent activity after a restart and storage rotation
1. The user records 257 sessions and the device restarts.
2. The user records 254 more sessions last week. The oldest 256 sessions contain 256 steps and the newer 255 contain 510 steps.
3. The user checks history and sees 766 steps.
4. The user records another session of 5 minutes and 50 steps.
5. The display shows 5 minutes and 50 steps. History shows 510 steps.
6. The device restarts. The same totals are displayed.
7. The user records another session of 1 minute and 10 steps.
8. The display shows 6 minutes and 60 steps. History still shows 510 steps.

## Implementation

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

On startup, storage scans the start epochs, counts occupied slots and places the write position at the first empty slot. The number of loaded sessions is logged. Sessions are returned from oldest to newest.

When 511 sessions are stored, storage erases the sector containing the oldest sessions before writing the next session. This removes 256 sessions.

Flash read, write and erase failures are logged.
