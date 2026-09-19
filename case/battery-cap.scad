include <mixin.scad>;

battery_cap_height = 8;

difference() {
  union() {
    translate([0, 0, 0]) cylinder(h=battery_cap_height + wall, d=battery_slot_diameter, center=false, $fn=128);
    translate([0, 0, 0]) cylinder(h=wall, d=battery_slot_diameter + 2, center=false, $fn=128);
  }

  translate([0, 0, wall]) cylinder(h=100, d=battery_slot_diameter - 2, center=false, $fn=128);
}
