# Time Sync
## Requirements
* Synchronize time after every wake from deep sleep.

## 1. Device synchronizes time on boot with backend
1. `METRICS_URL` and `JWT_TOKEN` are configured.
2. The user presses a button.
3. The device wakes from deep sleep.
4. The display shows `WIFI`.
5. The device initializes Wi-Fi and begins association.
6. The display shows `DHCP`.
7. The device receives network configuration.
8. The display shows `DNS`.
9. The device resolves the NTP server.
10. The display shows `NTP`.
11. The device synchronizes time.
12. The display shows current-week minutes and steps.
13. Wi-Fi remains connected.

## 2. Device synchronizes time on boot without backend
1. `METRICS_URL` and `JWT_TOKEN` are not configured.
2. The user presses a button.
3. The device wakes from deep sleep.
4. The display shows `WIFI`.
5. The device initializes Wi-Fi and begins association.
6. The display shows `DHCP`.
7. The device receives network configuration.
8. The display shows `DNS`.
9. The device resolves the NTP server.
10. The display shows `NTP`.
11. The device synchronizes time.
12. The display shows current-week minutes and steps.
13. Wi-Fi disconnects.
