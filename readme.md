# Stepper

## Overview
* offline first: works without backend or wi-fi, stores permanently 1024 sessions
* detect steps via magnet trigger
* manages workout sessions (start on first trigger, stops after one minute from last trigger)
* tracks and displays total training time (today + current week)
* the history button displays 3 previous weeks

## Build & Flash
```fish
nix develop

SSID="WiFi" PASS="Password" UTC_OFFSET=180 cargo run --release
```
