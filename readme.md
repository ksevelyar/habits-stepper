# Stepper

## Overview
* detect steps via magnet trigger
* manages workout sessions (start on first trigger, stops after 2 minutes from last trigger)
* tracks and displays total training time (today + current week).
* sends single session record to backend on session end via json POST

## Session
* mcu wakes up from deep sleep via reed switch
* ended_at updates after 2 minutes of inactivity
* json data sends to backend via POST
* deep sleep
* use nvs to store last 10 sessions?

## Battery
* 18650: 4.2V-3V
* mcu + oled + 2-4 http post requests per day

## Session structure?
```json
{
  "steps": u32,
  "started_at": i64,
  "ended_at": i64,
  "synced": false
}
```

## Build & Flash

```fish
nix develop

SSID="WiFi" PASS="Password" UTC_OFFSET=180 cargo run --release
```

## Update deps
The simplest way is to regenerate a dummy project and copy the dependencies from it:

```
cargo generate esp-rs/esp-idf-template cargo
```
