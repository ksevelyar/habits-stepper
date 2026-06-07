# Habits Display

## Overview
* async runtime with embassy
* deep sleep 
* 18650 battery + tp4057 charger
* upload firmware and charge from one usb type c port
* ring buffer with nor flash
* embedded_test to run tests directly on the mcu

## Build & Flash
```
nix develop

cargo run --release
    Finished `release` profile [optimized + debuginfo] target(s) in 0.10s
     Running `probe-rs run --chip=esp32c3 --preverify --always-print-stacktrace --no-location target/riscv32imc-unknown-none-elf/release/habits-stepper`
    Verifying ✔ 100% [####################] 321.10 KiB @  74.58 KiB/s (took 4s)           Finished in 4.54s
[INFO ] Embassy initialized!
[INFO ] starting wifi
[INFO ] IPv4: DOWN
[INFO ] flash capacity: 4194304B
[INFO ] loaded 3 sessions from flash (head=3)
[INFO ] RTC seeded: Europe/Moscow 2026-06-07 10:30:06
[INFO ] start connection task
[INFO ] About to connect...
[INFO ] link_up = true
[INFO ] IPv4: DOWN
[INFO ] Connected to "fellowship-of-the-ring-2" (channel: 4)
[INFO ] time: synced (-1s)
[INFO ] Europe/Moscow 2026-06-07 10:30:09
[INFO ] time: NTP sync done
[INFO ] time: waiting 90s for inactivity
```

## udev setup for ESP32-C3 with probe-rs

```nix
services.udev.extraRules = ''
  # NOTE: esp32c3
  SUBSYSTEM=="usb", ATTR{idVendor}=="303a", ATTR{idProduct}=="1001", MODE="0660", GROUP="dialout"
'';
```
