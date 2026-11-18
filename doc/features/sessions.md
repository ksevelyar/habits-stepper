# Sessions
## Overview
Session task receives events from user_input, manages
session state, and sends session events to display.

## Input
* `StepDetected`: creates or prolongs current session
* `HistoryPressed`: switches display to SessionHistory view
* `HistoryReleased`: switches display back to SessionUpdate view

## Output
* `SessionEvent::Update(WeekTotals { minutes, steps })`: sent on step or history toggle
* `SessionEvent::History(WeekTotals { minutes, steps })`: sent on HistoryPressed

## Permanent storage
* Load sessions from permanent storage, log error on failure
* On load, log the number of loaded sessions
* On session end, save to permanent storage, signal today and yesterday totals for the backend, and log
* Backend replaces the value for a given date, so lost or failed posts self-heal on the next session end
* esp32c3 NOR flash wear should be minimized
* See [sessions/storage.md](sessions/storage.md) for the storage design
