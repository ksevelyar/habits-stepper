diameter = 71.4;
height = 85;

display_length = 76;
display_width = 19.1;

wall = 2.3;

module leg(leg_height) {
  difference() {
    cylinder(h=leg_height, d=6.24, $fn=32);
    cylinder(h=leg_height + 1, d=3.12, $fn=32);
  }
}

module walls() {
  difference() {
    translate([0, 0, wall]) cylinder(h=height, d1=diameter - 0.5, d2=diameter, $fn=256);
    translate([0, 0, -0.1]) cylinder(h=height * 2, d=diameter - wall - 0.5, $fn=256);

    translate([-30 / 2, -50, 0]) cube(size=[30, 100, height + 5], center=false);
  }

  translate([-diameter / 2 + 1.9, -20.5/2, 0]) cube(size=[8.2, 20.5, height + 8], center=false);
  translate([diameter / 2 - 10.3, -20.5/2, 0]) cube(size=[8.2, 20.5, height + 8], center=false);
}

module battery() {
  difference() {
    union() {
      difference() {
        hull() {
          length = display_length + 0.2;
          width = display_width + 3.1;
          translate([-length / 2, 0, 0]) cube(size=[length, width, wall]);

          translate([0, 0, 0]) cylinder(h=wall, d=diameter - 0.5, $fn=128);
        }

        translate([0, -21, -1]) cylinder(h=wall * 2, d=15, $fn=128);
      }
      translate([0, 9.5, 0]) cylinder(h=height - wall, d=39, center=false, $fn=128);
    }
    translate([0, 9.5, wall]) cylinder(h=height + 1, d=36, center=false, $fn=128);
    translate([0, 9.5, -0.1]) cylinder(h=height + 1, d=18, center=false, $fn=128);

    translate([12.7, -8.8, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=32);
    translate([-12.7, -8.8, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=32);

    translate([12.7, 27.4, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=32);
    translate([-12.7, 27.4, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=32);
  }
}

battery();
walls();
