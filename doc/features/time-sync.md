# Time Sync

## Requirements
* The device synchronizes time after every wake from deep sleep via NTP.
* The display shows synchronization progress.
* Wi-Fi disconnects after synchronization when backend synchronization is not configured.

## User scenarios

### 1. Device synchronizes time on boot with backend
1. Backend synchronization is configured.
2. The user presses a button and the device wakes from deep sleep.
3. The display shows `WIFI` while the device connects to Wi-Fi.
4. The display shows `DHCP` while the device obtains an IP address.
5. The display shows `DNS` while the device resolves the time server.
6. The display shows `NTP` while the device synchronizes its time.
7. The display shows the current week's minutes and steps.
8. Wi-Fi remains connected.

### 2. Device synchronizes time on boot without backend
1. Backend synchronization is not fully configured.
2. The user presses a button and the device wakes from deep sleep.
3. The display shows `WIFI` while the device connects to Wi-Fi.
4. The display shows `DHCP` while the device obtains an IP address.
5. The display shows `DNS` while the device resolves the time server.
6. The display shows `NTP` while the device synchronizes its time.
7. After synchronization, the display shows the current week's minutes and steps and Wi-Fi disconnects.
