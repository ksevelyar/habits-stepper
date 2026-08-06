include <mixin.scad>;

battery_cap_height = 8;

difference() {
  union() {
    translate([0, 0, 0]) cylinder(h=battery_cap_height + wall, d=36, center=false, $fn=128);
    translate([0, 0, 0]) cylinder(h=wall, d=38, center=false, $fn=128);
  }

  translate([0, 0, wall]) cylinder(h=100, d=36 - 2, center=false, $fn=128);
}
