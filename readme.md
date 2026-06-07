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
    Verifying ✔ 100% [####################] 359.17 KiB @ 772.58 KiB/s (took 0s)            Erasing ✔ 100% [####################] 704.00 KiB @ 682.35 KiB/s (took 1s)
  Programming ✔ 100% [####################] 359.17 KiB @  84.37 KiB/s (took 4s)           Finished in 6.11s
03:00:00 [INFO ] init: embassy initialized
03:00:00 [INFO ] storage: flash capacity: 4194304B
03:00:00 [INFO ] storage: loaded 0 sessions from flash (head=0)
03:00:00 [INFO ] wifi: connecting to "fellowship-of-the-ring-2"
03:00:00 [WARN ] wifi: connection attempt 1 failed
03:00:00 [ERROR] wifi: disconnected: SSID: "fellowship-of-the-ring-2", reason: NoAccessPointFound, RSSI: -128
03:00:00 [INFO ] wifi: connecting to "fellowship-of-the-ring-2"
03:00:00 [INFO ] wifi: connected to "fellowship-of-the-ring-2"
17:49:12 [INFO ] time: synced (+0s)
17:49:12 [INFO ] time: Europe/Moscow 2026-06-07 17:49:12
17:49:12 [INFO ] time: waiting 90s for inactivity
```

## udev setup for ESP32-C3 with probe-rs

```nix
services.udev.extraRules = ''
  SUBSYSTEM=="usb", ATTR{idVendor}=="303a", ATTR{idProduct}=="1001", MODE="0660", GROUP="dialout"
'';
```
