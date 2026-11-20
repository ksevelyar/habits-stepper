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
1. The user has recorded 511 sessions over several months. The oldest 256 sessions were recorded before last week. Last week's total is 30 minutes and 300 steps, and this week's total is 20 minutes and 200 steps.
2. The user records another session of 5 minutes and 50 steps.
3. After the session ends, the user checks the current week's totals: 25 minutes and 250 steps.
4. The user presses the history button and sees 30 minutes and 300 steps.

### 3. User sees recent activity after a restart and storage rotation
1. The user records 257 sessions and the device restarts.
2. Over the following months, the user records 254 more sessions. Last week's total is 30 minutes and 300 steps, and this week's total is 20 minutes and 200 steps.
3. The user records another session of 5 minutes and 50 steps.
4. After the session ends, the display shows 25 minutes and 250 steps for the current week.
5. The user presses the history button and sees 30 minutes and 300 steps.

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
