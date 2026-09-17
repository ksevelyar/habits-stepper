include <mixin.scad>;

height = 15;

module display_cutout() {
  display_slot_cutout();
  display_window_cutout(clearance = 0.4);
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
          translate([11, display_pcb_width - 13, 7]) leg(wall);

          translate([display_length / 2 - 15, display_pcb_width + 4, 7]) leg(wall);
        }

        hull() {
          translate([-11, display_pcb_width - 13, 11 - wall * 2]) leg(wall);
          translate([-display_length / 2 + 15, display_pcb_width + 4, 11 - wall * 2]) leg(wall);
        }
      }

      translate([display_length / 2 - 15, display_pcb_width + 4, 6]) cylinder(10, d=3.12, $fn=32);

      translate([-display_length / 2 + 15, display_pcb_width + 4, 6]) cylinder(10, d=3.12, $fn=32);
    }

    difference() {
      length = 30;
      translate([-length / 2, display_pcb_width - 11.8, height - wall * 4]) cube([length, 6, wall]);
      translate([-length / 2, display_pcb_width - 6, height - wall * 4 - 0.2]) rotate([45, 0, 0]) cube([length, wall, wall]);
    }
  }
}

stopper();
