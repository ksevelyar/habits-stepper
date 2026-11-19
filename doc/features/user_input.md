# User input

## Reed switch
A single reed switch on the right side captures both left and right steps. The pedals work in opposition: pressing the left pedal raises the right pedal, opening the reed and counting a left step. Pressing the right pedal closes the reed and counts a right step. Both edges are counted as `StepDetected`.

## History button
When pressed, the display shows the previous week's totals. When released, it returns to the current week.

## Boot
On boot, user_input sends `HistoryReleased` so the display shows data immediately.

## Sleep
* The MCU wakes from deep sleep via the reed switch or history button
* The MCU enters deep sleep after 90 seconds of inactivity
* The reed's current level is sampled before sleep. Wake-up is set for the opposite level, so the chip wakes on the first reed change whether the pedal was pressed or released at sleep time
