# Wiring

## charging (tp4057)
5v from esp32 to in+
gnd from esp32 to in-

bat- to 18650-
bat+ to 18650+

out- to gnd from esp32
out+ - ldo mcp1700 3.3v - esp32 3.3v

## display sh1122, spi
gnd - gnd
vdd - 3v3
scl - gpio5
sda - gpio6
rst - gpio7
dc  - gpio9
cs  - gpio10

## buttons
reed - gpio1 + gnd
history - gpio2 + gnd
