diameter = 70.5;
height = 28;

display_length = 76;
display_width = 19.1;

esp32c3_width = 18.5;
esp32c3_length = 24;
wall = 2.5;

module esp32c3_mini_rails() {
  half_width = esp32c3_width / 2;

  // TODO: refactor to rail()
  translate([-half_width - 2, -33, 0]) cube([2, esp32c3_length, 11]);
  hull() {
    translate([-half_width - 0.15, -33, 7.5]) rotate([0, -40, 0]) cube([1.5, esp32c3_length, 1]);

    translate([-half_width - 0.5, -33, 8.4]) cube([1.5, esp32c3_length, 1]);
  }
  translate([-half_width - 0.35, -33, 10]) rotate([0, -40, 0]) cube([1.3, esp32c3_length, 1]);

  mirror([1, 0, 0]) {
    translate([-half_width - 2, -33, 0]) cube([2, esp32c3_length, 11]);
    hull() {
      translate([-half_width - 0.15, -33, 7.5]) rotate([0, -40, 0]) cube([1.5, esp32c3_length, 1]);

      translate([-half_width - 0.5, -33, 8.4]) cube([1.5, esp32c3_length, 1]);
    }
    translate([-half_width - 0.35, -33, 10]) rotate([0, -40, 0]) cube([1.3, esp32c3_length, 1]);
  }
}

module tp4057_rails() {
  half_width = 13 / 2;
  tp4057_length = 14;

  translate([-half_width - 2, 20, 0]) cube([2, tp4057_length, 11]);
  hull() {
    translate([-half_width - 0.15, 20, 7.5]) rotate([0, -40, 0]) cube([1.5, tp4057_length, 1]);

    translate([-half_width - 0.5, 20, 8.4]) cube([1.5, tp4057_length, 1]);
  }
  translate([-half_width - 0.35, 20, 10]) rotate([0, -40, 0]) cube([1.3, tp4057_length, 1]);

  mirror([1, 0, 0]) {
    translate([-half_width - 2, 20, 0]) cube([2, tp4057_length, 11]);
    hull() {
      translate([-half_width - 0.15, 20, 7.5]) rotate([0, -40, 0]) cube([1.5, tp4057_length, 1]);

      translate([-half_width - 0.5, 20, 8.4]) cube([1.5, tp4057_length, 1]);
    }
    translate([-half_width - 0.35, 20, 10]) rotate([0, -40, 0]) cube([1.3, tp4057_length, 1]);
  }
}

module button_cutout() {
  translate([-7, -24.5, -1]) cube(size=14);
}

module display_cutout() {
  translate([-67 / 2, 0, 0.32]) cube([67, display_width, 100]);
}

module display() {
  difference() {
    hull() {
      length = display_length + wall * 2 + 0.5;
      width = display_width + wall * 2 + 0.5;
      translate([-length / 2, 0, 0]) cube(size=[length, width, height]);

      cylinder(h=height, d=diameter + wall * 2, $fn=128);
    }

    hull() {
      length = display_length + 0.5;
      width = display_width + 3.5;
      translate([-length / 2, 0, wall]) cube(size=[length, width, height + wall]);

      translate([0, 0, wall]) cylinder(h=height, d=diameter + 0.3, $fn=128);
    }
  }
}

module type_c_cutout() {
  translate([0, -diameter / 2 + 4, 9.5]) rotate([90, 0, 0])
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

module battery() {
  translate([12.7, -8.8, 0]) leg(height - wall);
  translate([-12.7, -8.8, 0]) leg(height - wall);

  translate([12.7, 27.4, 0]) leg(height - wall);
  translate([-12.7, 27.4, 0]) leg(height - wall);
}

esp32c3_mini_rails();
tp4057_rails();
difference() {
  union() {
    display();
    // walls();
  }
  type_c_cutout();
  button_cutout();
  display_cutout();

  translate([display_length / 2 - 2.3, 2, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=64, center=false);
  translate([-display_length / 2 + 2.3, 2, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=64, center=false);
  translate([display_length / 2 - 2.5, display_width - 2, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=64, center=false);
  translate([-display_length / 2 + 2.5, display_width - 2, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=64, center=false);
}

battery();
