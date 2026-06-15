# Sessions
## Overview
Session task receives events from user_input, manages
session state, and sends session events to display.

## Input
- `StepDetected` — creates or prolongs current session
- `HistoryPressed` — switches display to SessionHistory view
- `HistoryReleased` — switches display back to SessionUpdate view

## Output
- `SessionEvent::Update(WeekTotals { minutes, steps })` — sent on step or history toggle
- `SessionEvent::History(WeekTotals { minutes, steps })` — sent on HistoryPressed

## Permanent storage
* Load sessions from permanent storage, log error on failure
* On load, log number of loaded sessions for current week and previous 3 weeks
* On session end, sync to backend, save to permanent storage, and log
* esp32c3 NOR flash wear should be minimized
