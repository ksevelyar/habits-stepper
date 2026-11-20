# Tracking

## Requirements
* The device counts each movement of either pedal as a step and groups steps into sessions.
* A session ends after one minute without a step.
* The display shows the current week's minutes and steps.
* The display shows the previous week's minutes and steps while the history button is pressed.
* A week begins on Monday at midnight in the configured timezone.

## User scenarios

### 1. Device counts steps from both pedals
1. The displayed step count is 0.
2. The user presses the left pedal and then the right pedal.
3. The displayed step count is 2.

### 2. User records a session
1. The current week's totals are 0 minutes and 0 steps.
2. The user takes 10 steps over 2 minutes.
3. The display shows 2 minutes and 10 steps for the current week.
4. The user stops stepping.
5. After one minute without a step, the session ends.

### 3. User views session history
1. The current week's totals are 10 minutes and 100 steps, and the previous week's totals are 30 minutes and 300 steps.
2. The user presses the history button.
3. The display shows 30 minutes and 300 steps.
4. The user releases the button.
5. The display shows 10 minutes and 100 steps.

### 4. User sees activity move to history on Monday
1. The user records 60 minutes and 60 steps on Sunday.
2. On Monday after midnight, the display shows 0 minutes and 0 steps for the current week.
3. The user presses the history button.
4. The display shows 60 minutes and 60 steps.

## Implementation
* A single reed switch on the right side captures both left and right steps. The pedals work in opposition, so pressing the left pedal opens the reed and pressing the right pedal closes it. Both edges produce `StepDetected`.
* The session task receives `StepDetected`, `HistoryPressed` and `HistoryReleased` events from user input and sends session events to the display.
* The device logs each completed session.
