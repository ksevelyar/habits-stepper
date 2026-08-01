include <mixin.scad>;

height = 82;

module walls() {
  difference() {
    union() {
      translate([0, 0, wall]) cylinder(h=height - 22, d1=diameter - 0.5, d2=diameter, $fn=256);
    }
    translate([0, 0, -0.1]) cylinder(h=height * 2, d=diameter - wall, $fn=256);

    translate([-40 / 2, -50, 0]) cube(size=[40, 100, height + 5], center=false);
  }

  difference() {
    union() {
      translate([15.5, -21.5, 0]) rotate([0, 0, -60]) walls_connector();
      translate([15.5, 21.5, 0]) rotate([0, 0, -120]) walls_connector();
      translate([-15.5, 21.5, 0]) rotate([0, 0, 120]) walls_connector();
      translate([-15.5, -21.5, 0]) rotate([0, 0, 60]) walls_connector();
    }

    translate([0, 0, wall]) cylinder(h=height + 1, d=36, center=false, $fn=128);
  }
}

module walls_connector() {
  difference() {
    translate([0, 0, wall]) cylinder(h=height - 22, d=18, $fn=256);
    translate([0, 0, -0.1]) cylinder(h=height * 2, d=16, $fn=256);
    translate([-18 / 2, -18, 0]) cube([18, 18, 115], center=false);
  }
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

        translate([0, -27, -1]) cylinder(h=wall * 2, d=12, $fn=128);
      }
      translate([0, 0, 0]) cylinder(h=height - wall, d=38, center=false, $fn=128);
    }
    translate([0, 0, wall]) cylinder(h=height + 1, d=36, center=false, $fn=128);
    translate([0, 0, -0.1]) cylinder(h=height + 1, d=18, center=false, $fn=128);

    translate([12.7, -23.4, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=32);
    translate([-12.7, -23.4, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=32);

    translate([12.7, 23.4, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=32);
    translate([-12.7, 23.4, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=32);
  }
}

battery();
walls();
