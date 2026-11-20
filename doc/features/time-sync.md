# Time Sync
## Requirements
* Synchronize time after every wake from deep sleep.

## 1. Device synchronizes time on boot with backend
1. `METRICS_URL`, `JWT_TOKEN` and `CHAIN_ID` are configured.
2. The user presses a button.
3. The device wakes from deep sleep.
4. The display shows `WIFI` while the device connects to Wi-Fi.
5. The display shows `DHCP` while the device obtains an IP address.
6. The display shows `DNS` while the device resolves the NTP server.
7. The display shows `NTP` while the device synchronizes its time.
8. The display shows the current week's minutes and steps.
9. Wi-Fi remains connected.

## 2. Device synchronizes time on boot without backend
1. At least one of `METRICS_URL`, `JWT_TOKEN` and `CHAIN_ID` is not configured.
2. The user presses a button.
3. The device wakes from deep sleep.
4. The display shows `WIFI` while the device connects to Wi-Fi.
5. The display shows `DHCP` while the device obtains an IP address.
6. The display shows `DNS` while the device resolves the NTP server.
7. The display shows `NTP` while the device synchronizes its time.
8. The display shows the current week's minutes and steps.
9. Wi-Fi disconnects.
