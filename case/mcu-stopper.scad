include <mixin.scad>;

height = 15;

module display_cutout() {
  translate([-67 / 2, 0, 0.4]) cube([67, display_width, 100]);
  translate([-56 / 2, 0.7, -0.1]) cube([56, 17.8, 100]);
}

module battery_mounts() {
  translate([12.7, -8.8, 0]) leg(height - wall);
  translate([-12.7, -8.8, 0]) leg(height - wall);

  translate([12.7, 27.4, 0]) leg(height - wall);
  translate([-12.7, 27.4, 0]) leg(height - wall);
}

module stopper() {
  translate([0, 0, wall]) {

    difference() {
      union() {
        hull() {
          translate([11, display_width - 13, 7]) leg(wall);

          translate([display_length / 2 - 15, display_width + 3.2, 7]) leg(wall);
        }

        hull() {
          translate([-11, display_width - 13, 11 - wall * 2]) leg(wall);
          translate([-display_length / 2 + 15, display_width + 3.2, 11 - wall * 2]) leg(wall);
        }
      }

      translate([display_length / 2 - 15, display_width + 3.2, 6]) cylinder(10, d=3.12, $fn=32);

      translate([-display_length / 2 + 15, display_width + 3.2, 6]) cylinder(10, d=3.12, $fn=32);
    }

    difference() {
      length = 33;
      translate([-length / 2, display_width - 10.8, height - wall * 4]) cube([length, 6, wall]);
      translate([-length / 2, display_width - 5, height - wall * 4 - 0.2]) rotate([45, 0, 0]) cube([length, wall, wall]);
    }
  }
}

stopper();
