diameter = 71.5;
height = 20;

display_length = 76;
display_holes_x = 73;
display_holes_y = 73;

display_width = 19.1;

esp32c3_length = 24;
wall = 2;

module esp32c3_mini_rails() {
  esp32c3_width = 18.1;
  half_width = 18.1 / 2;

  translate([-half_width - 2, -33, 0]) cube([2, esp32c3_length, 11]);
  translate([-half_width - 1, -33, 8.5]) cube([1.5, esp32c3_length, 1]);
  translate([-half_width - 0.35, -33, 10.2]) rotate([0, -40, 0]) cube([1.3, esp32c3_length, 1]);

  mirror([1, 0, 0]) {
    translate([-half_width - 2, -33, 0]) cube([2, esp32c3_length, 11]);
    translate([-half_width - 1, -33, 8.5]) cube([1.5, esp32c3_length, 1]);
    translate([-half_width - 0.35, -33, 10.2]) rotate([0, -40, 0]) cube([1.3, esp32c3_length, 1]);
  }
}

module button_cutout() {
  translate([-7, -24.5, -1]) cube(size=14);
}

module display_cutout() {
  translate([-67.5 / 2, 0, 0.1]) cube(size=[67.5, display_width, wall + 0.2]);
}

module display() {
  difference() {
    hull() {
      translate([-display_length / 2, 0, 0]) cube(size=[display_length, display_width, wall]);

      cylinder(h=wall, d=diameter, $fn=128);
    }
  }
}

module walls() {
  difference() {
    cylinder(h=height, d=diameter, $fn=128);
    translate([0, 0, -0.1]) cylinder(h=height + 1, d=diameter - wall, $fn=128);

    translate([-display_length / 2, -3, wall]) cube(size=[display_length, display_width + 6, height]);
  }
}

module type_c_cutout() {
  translate([0, -diameter / 2 + 4, 12.2]) rotate([90, 0, 0])
      hull() {
        translate([-2.9, 0, 0]) cylinder(h=10, d=3.7, $fn=64);

        translate([2.9, 0, 0]) cylinder(h=10, d=3.7, $fn=64);
      }
}

esp32c3_mini_rails();
difference() {
  union() {
    display();
    walls();
  }
  type_c_cutout();
  button_cutout();
  display_cutout();

  translate([display_length / 2 - 2.5, 2, -0.1]) cylinder(h=wall * 2, d=2.8, $fn=64, center=false);
  translate([-display_length / 2 + 2.5, 2, -0.1]) cylinder(h=wall * 2, d=2.8, $fn=64, center=false);
  translate([display_length / 2 - 2.5, display_width - 2, -0.1]) cylinder(h=wall * 2, d=2.8, $fn=64, center=false);
  translate([-display_length / 2 + 2.5, display_width - 2, -0.1]) cylinder(h=wall * 2, d=2.8, $fn=64, center=false);
}
