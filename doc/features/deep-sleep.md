# Deep Sleep

## Requirements
* The device enters deep sleep after 120 seconds of inactivity.
* The device wakes from deep sleep on a pedal movement or history button press.

## User scenarios

### 1. Device wakes when the user moves a pedal
1. The device is in deep sleep.
2. The user moves a pedal.
3. The device wakes.

### 2. Device wakes when the user presses the history button
1. The device is in deep sleep.
2. The user presses the history button.
3. The device wakes.

### 3. Device sleeps after inactivity
1. The user stops interacting with the device.
2. After 120 seconds of inactivity, the device enters deep sleep.
3. The next pedal movement wakes the device, regardless of the pedal position when it went to sleep.

## Implementation
* Before sleep, the device samples the reed switch's current level and configures wake-up for the opposite level.
