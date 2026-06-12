include <mixin.scad>;

translate([0, 0, 0]) cylinder(h=3 + wall, d=36 - 0.3, center=false, $fn=128);
translate([0, 0, 0]) cylinder(h=wall, d=40, center=false, $fn=128);
