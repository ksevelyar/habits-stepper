# Sessions
## Overview
Session task receives events from user_input, manages
session state, and sends session events to display.

## Input
- `StepDetected` — creates or prolongs current session
- `HistoryPressed` — switches display to SessionHistory view
- `HistoryReleased` — switches display back to SessionUpdate view

## Output
- `SessionUpdate{today_minutes, week_minutes, today_steps}` — sent on step or history toggle
- `SessionHistory{current_week_minutes, prev_week_minutes}` — sent on HistoryPressed

## Permanent storage
* Sessions should load sessions from permanent storage and log error on fail to load them
* on load should log amount of loaded sessions for current week adn previous 3 weeks.
* on session end it should be synced to backend and saved into permanent storage, also should be logged
* esp32c3 NOR flash wear should be minimased
