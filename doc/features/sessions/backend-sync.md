# Backend Sync

## Requirements
* When backend synchronization is configured, the device sends today's and yesterday's total minutes to the backend after a session ends.
* Daily totals use dates in the configured timezone and include the portion of each session that falls on that date.

## User scenarios

### 1. Device sends today's and yesterday's totals after a session
1. `METRICS_URL`, `JWT_TOKEN` and `CHAIN_ID` are configured, and yesterday's total is 20 minutes.
2. The user completes a session, bringing today's total to 10 minutes.
3. The backend receives 10 minutes for today and 20 minutes for yesterday.

### 2. Device sends no totals without a metrics URL
1. `METRICS_URL` is not configured.
2. The user completes a session.
3. The device sends no totals to the backend.

### 3. Device sends the sum of sessions on the same day
1. The user completes a 1 minute session.
2. Later that day, the user completes another 1 minute session.
3. The backend receives a total of 2 minutes for that date.

### 4. Device sends a session across midnight on both dates
1. The user steps for 20 minutes before midnight and 20 minutes after midnight.
2. The session ends.
3. The backend receives 20 minutes for each date.
