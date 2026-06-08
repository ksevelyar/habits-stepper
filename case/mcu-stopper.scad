diameter = 71.4;
height = 15;

display_length = 77;
display_width = 19.1;

esp32c3_width = 18.5;
esp32c3_length = 24;
wall = 2;

module rail_side(half_width, length, y) {
  translate([-half_width - 2, y, 2]) cube([2, length, 9]);
  hull() {
    translate([-half_width - 0.15, y, 7.5]) rotate([0, -40, 0]) cube([1.5, length, 1]);
    translate([-half_width - 0.5, y, 8.4]) cube([1.5, length, 1]);
  }
  translate([-half_width - 0.35, y, 10]) rotate([0, -40, 0]) cube([1.3, length, 1]);
}

module rail(half_width, length, y) {
  rail_side(half_width, length, y);
  mirror([1, 0, 0]) rail_side(half_width, length, y);
}

module esp32c3_mini_rails() {
  rail(esp32c3_width / 2, esp32c3_length - 8, 19);
}

module tp4057_rails() {
  rail(13 / 2, 14, -36);
}

module button_cutout() {
  size = 14.1;
  translate([-size / 2, -24.5, -1]) cube(size=size);
}

module charger_led_cutout() {
  translate([0, -30.1, 0.2]) cylinder(h=wall, d=10);
}

module display_cutout() {
  translate([-67 / 2, 0, 0.4]) cube([67, display_width, 100]);
  translate([-56 / 2, 0.7, -0.1]) cube([56, 17.8, 100]);
}

module display() {
  difference() {
    hull() {
      length = display_length + wall * 2 + 0.5;
      width = display_width + wall * 2 + 0.5;
      translate([-length / 2, 0, 0]) cube(size=[length, width, height]);

      cylinder(h=height, d=diameter + wall * 2, $fn=256);
    }

    hull() {
      length = display_length + 0.5;
      width = display_width + 3.5;
      translate([-length / 2, 0, wall]) cube(size=[length, width, height + wall]);

      translate([0, 0, wall]) cylinder(h=height, d=diameter + 0.3, $fn=256);
    }
  }
}

module type_c_cutout() {
  translate([0, diameter / 2 + 4, 8.5]) rotate([90, 0, 0])
      hull() {
        translate([-2.9, 0, 0]) cylinder(h=10, d=3.7, $fn=64);

        translate([2.9, 0, 0]) cylinder(h=10, d=3.7, $fn=64);
      }
}

module leg(leg_height) {
  difference() {
    cylinder(h=leg_height, d=6.24, $fn=32);
    cylinder(h=leg_height + 1, d=3.12, $fn=32);
  }
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
