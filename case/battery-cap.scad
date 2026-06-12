include <mixin.scad>;

difference() {
  union() {
    translate([0, 0, 0]) cylinder(h=4 + wall, d=36, center=false, $fn=128);
    translate([0, 0, 0]) cylinder(h=wall, d=38, center=false, $fn=128);
  }

  translate([0, 0, wall]) cylinder(h=100, d=36 - 2, center=false, $fn=128);
}
