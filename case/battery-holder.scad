diameter = 70.5;
height = 85;

display_length = 76;
display_width = 19.1;

wall = 2.5;

module leg(leg_height) {
  difference() {
    cylinder(h=leg_height, d=6.24, $fn=32);
    cylinder(h=leg_height + 1, d=3.12, $fn=32);
  }
}

module walls() {
  difference() {
    cylinder(h=height, d=diameter - 0.2, $fn=128);
    translate([0, 0, -0.1]) cylinder(h=height + 1, d=diameter - wall - 0.5, $fn=128);
  }

  translate([-diameter/2+1.3,-9,0]) cube(size = [4,18,height+4], center = false);
  translate([diameter/2-5.3,-9,0]) cube(size = [4,18,height+4], center = false);
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

        translate([0, -20, -1]) cylinder(h=wall * 2, d=10, $fn=128);
      }

      translate([0, 9.5, 0]) cylinder(h=height - wall, d=38, center=false, $fn=128);
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
