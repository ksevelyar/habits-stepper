diameter = 71.5;
height = 85;

display_length = 76;
display_holes_x = 73;
display_holes_y = 73;

display_width = 19.1;

esp32c3_width = 18.2;
esp32c3_length = 24;
wall = 2;

module leg(leg_height) {
  difference() {
    cylinder(h=leg_height, d=6.24, $fn=32);
    cylinder(h=leg_height + 1, d=3.12, $fn=32);
  }
}

module battery() {
  difference() {
    union() {
      difference() {
        hull() {
          length = display_length + 0.5;
          width = display_width + 3.5;
          translate([-length / 2, 0, 0]) cube(size=[length, width, wall]);

          translate([0, 0, 0]) cylinder(h=wall, d=diameter - 2, $fn=128);
        }

        difference() {
          cylinder(h=height, d=diameter+0.1, $fn=128);
          translate([0, 0, -0.1]) cylinder(h=height + wall + 1, d=diameter - wall-0.1, $fn=128);
          translate([-display_length / 2, -3, -0.1]) cube(size=[display_length, display_width + 6, height]);
        }
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
