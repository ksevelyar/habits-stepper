# Wiring

## Charging (TP4057)
5 V from ESP32 to IN+
GND from ESP32 to IN-

BAT- to 18650-
BAT+ to 18650+

OUT- to GND from ESP32
OUT+ to MCP1700 pin 2

## MCP1700
1 - GND
2 - TP4057 OUT+
3 - ESP32 3.3 V

## SH1122 display, SPI
GND - GND
VDD - 3V3
SCL - GPIO5
SDA - GPIO6
RST - GPIO7
DC - GPIO9
CS - GPIO10

## Buttons
Reed - GPIO1 + GND
History - GPIO2 + GND
