# User input

## Reed switch
A single reed switch on the right side captures both left and right steps. The pedals work in opposition: pressing the left pedal raises the right pedal, opening the reed and counting a left step; pressing the right pedal closes the reed and counts a right step. Both edges are counted as `StepDetected`.

## History button
When pressed the display shows the previous week's totals; when
released it returns to the current week.

## Boot
On boot, user_input sends `HistoryReleased` so the display shows data immediately.

## Sleep
* mcu wakes up from deep sleep via reed switch or history button
* deep sleep after 90s of inactivity
* the reed current level is sampled before sleep. Wake-up is set for the opposite level. This way the chip wakes on the first reed change whether the pedal was pressed or released at sleep time.
