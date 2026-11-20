# Habits Stepper
![](doc/stepper.jpg)

Automatically tracks workouts and sends them to a JSON backend over Wi-Fi. NTP synchronization handles day and week boundaries automatically, so there is no need to reset counts on Monday.

## Features
* automatic tracking of stepping sessions
* time-synchronized via NTP with a compile-time IANA timezone
* deep sleep after 120 seconds of inactivity and approximately two weeks of battery life with an 18650 battery
* firmware upload, logs, and battery charging over a single USB-C port
* persistent workout storage in NOR flash that survives deep sleep and reboots
* 3D-printed case with an 256x64 OLED display

## Documentation
* [Project boundaries](doc/boundaries.md)
* [Debugging](doc/debug.md)
* [Deep sleep](doc/features/deep-sleep.md)
* [Time synchronization](doc/features/time-sync.md)
* [Session tracking](doc/features/sessions/tracking.md)
* [Permanent session storage](doc/features/sessions/permanent-storage.md)
* [Backend sync](doc/features/sessions/backend-sync.md)
* [Mechanical maintenance](doc/hardware/mechanical-maintenance.md)
* [Power](doc/hardware/power.md)
* [Wiring](doc/hardware/wiring.md)
* [ESP32-C3 SuperMini datasheet](doc/hardware/ESP32-C3-SuperMini-datasheet.pdf)
* [MCP1700 datasheet](doc/hardware/MCP1700.pdf)

## Tests
Tests require a connected MCU.

```
nix develop
cargo test
```

## Build & Flash
```
nix develop

cargo run --release
    Finished `release` profile [optimized + debuginfo] target(s) in 0.10s
     Running `probe-rs run --chip=esp32c3 --preverify --always-print-stacktrace --no-location target/riscv32imc-unknown-none-elf/release/habits-stepper`
    Verifying ✔ 100% [####################] 360.25 KiB @  74.20 KiB/s (took 5s)           Finished in 5.09s
03:00:00 [INFO ] init: embassy initialized
03:00:00 [INFO ] storage: flash capacity: 4194304B
03:00:00 [INFO ] storage: loaded 18 sessions from flash (head=18)
13:57:03 [INFO ] time: rtc seeded: Europe/Moscow 2026-06-13 13:57:03
13:57:03 [INFO ] wifi: connecting to "fellowship-of-the-ring-2"
13:57:04 [INFO ] wifi: connected to "fellowship-of-the-ring-2"
13:57:18 [INFO ] time: synced (+4s)
13:57:18 [INFO ] time: Europe/Moscow 2026-06-13 13:57:18
13:57:18 [INFO ] time: waiting 90s for inactivity
13:57:26 [INFO ] input: reed closed
13:57:26 [INFO ] time: activity before timeout, resetting
13:57:26 [INFO ] time: waiting 90s for inactivity
13:57:26 [INFO ] display: session update: today=50min(1036steps) week=50min
13:57:29 [INFO ] input: reed closed
13:57:29 [INFO ] time: activity before timeout, resetting
13:57:29 [INFO ] time: waiting 90s for inactivity
13:57:29 [INFO ] display: session update: today=50min(1037steps) week=50min
```
