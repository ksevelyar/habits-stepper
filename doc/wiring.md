# Wiring

## charging (tp4057)
5v from esp32 to in+
gnd from esp32 to in-

bat- to 18650-
bat+ to 18650+

out- to gnd from esp32
out+ - ldo mcp1700 3.3v - esp32 3.3v

## display
gnd - gnd
vdd - 3v3
scl - gpio6
sda - gpio7
rst - gpio3
dc  - gpio4
cs  - gpio10

## buttons
reed - gpio2 + gnd
history - gpio1 + gnd
