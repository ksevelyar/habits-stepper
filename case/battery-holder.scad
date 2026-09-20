include <mixin.scad>;

height = 79;
connector_inner_diameter = 17.4;
mounting_hole_diameter = 3.12;

module walls() {
  difference() {
    translate([0, 0, wall]) cylinder(h=height - 22, d1=diameter - 0.5, d2=diameter, $fn=256);
    translate([0, 0, -overcut]) cylinder(h=height * 2, d=diameter - wall, $fn=256);

    translate([-20, -40, 0]) cube(size=[40, 100, height + overcut], center=false);
  }

  difference() {
    union() {
      translate([15.1, -20.7, 0]) rotate([0, 0, -60]) walls_connector();

      translate([15.1, 20.7, 0]) rotate([0, 0, -120]) walls_connector();
      translate([-15.1, 20.7, 0]) rotate([0, 0, 120]) walls_connector();
      translate([-15.1, -20.7, 0]) rotate([0, 0, 60]) walls_connector();
    }


  }

}

module walls_connector() {
  difference() {
    translate([0, 0, wall]) cylinder(h=height - 22, d=connector_inner_diameter+wall, $fn=256);
    translate([0, 0, -overcut]) cylinder(h=height * 2, d=connector_inner_diameter, $fn=256);

    translate([-(connector_inner_diameter+wall) / 2, -connector_inner_diameter, 0])
        cube([connector_inner_diameter+wall, connector_inner_diameter, height+overcut], center=false);
  }

}

module battery() {
  difference() {
    union() {
      difference() {
        hull() {
          length = display_length + 0.2;
          width = display_pcb_width + 3.1;
          translate([-length / 2, 0, 0]) cube(size=[length, width, wall]);

          translate([0, 0, 0]) cylinder(h=wall, d=diameter - 0.5, $fn=128);
        }

        translate([0, -27, -1]) cylinder(h=wall * 2, d=12, $fn=128);
      }
      translate([0, 0, 0]) cylinder(h=height - wall, d=battery_slot_diameter + 2, center=false, $fn=128);
    }

    translate([0, 0, wall]) cylinder(h=height + overcut, d=battery_slot_diameter, center=false, $fn=128);
    translate([0, 0, -overcut]) cylinder(h=height + overcut, d=18, center=false, $fn=128);

    mounting_holes();
  }
}

module mounting_holes() {
  translate([12.7, -23.4, -overcut]) cylinder(h=wall * 2, d=mounting_hole_diameter, $fn=32);
  translate([-12.7, -23.4, -overcut]) cylinder(h=wall * 2, d=mounting_hole_diameter, $fn=32);
  translate([12.7, 23.4, -overcut]) cylinder(h=wall * 2, d=mounting_hole_diameter, $fn=32);
  translate([-12.7, 23.4, -overcut]) cylinder(h=wall * 2, d=mounting_hole_diameter, $fn=32);
}

module cross() {
translate([-wall/2,-10,0]) cube([wall,20,wall]);
translate([-10,-wall/2,0]) cube([20,wall,wall]);
}

cross();

battery();
walls();

