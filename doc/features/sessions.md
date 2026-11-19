# Sessions
## Overview
The session task receives events from user_input, manages session state, and sends session events to the display.

## Input
* `StepDetected`: creates or prolongs the current session
* `HistoryPressed`: switches the display to the SessionHistory view
* `HistoryReleased`: switches the display back to the SessionUpdate view

## Output
* `SessionEvent::Update(WeekTotals { minutes, steps })`: sent on a step or history toggle
* `SessionEvent::History(WeekTotals { minutes, steps })`: sent on HistoryPressed

## Permanent storage
* Load sessions from permanent storage and log an error on failure
* On load, log the number of loaded sessions
* On session end, save to permanent storage, signal today's and yesterday's totals for the backend, and log
* The backend replaces the value for a given date, so lost or failed posts self-heal when the next session ends
* Minimize ESP32-C3 NOR flash wear
* See [sessions/storage.md](sessions/storage.md) for the storage design
