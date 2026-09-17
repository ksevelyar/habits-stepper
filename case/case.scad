include <mixin.scad>;

height = 24;

module charger_led_cutout() {
  translate([0, -30.1, 0.2]) cylinder(h=wall, d=8);
}

module display_cutout() {
  display_slot_cutout();
  display_window_cutout();
}

module mounts() {
  translate([12.7, -23.4, 0]) leg(height - wall);
  translate([-12.7, -23.4, 0]) leg(height - wall);

  translate([12.7, 23.4, 0]) leg(height - wall);
  translate([-12.7, 23.4, 0]) leg(height - wall);

  translate([display_length / 2 - 15, display_pcb_width + 4, 0]) leg(10);
  translate([-display_length / 2 + 15, display_pcb_width + 4, 0]) leg(10);
}

difference() {
  union() {
    display();
    esp32c3_mini_rails();
    tp4057_rails();
    mounts();
  }

  type_c_cutout();
  button_cutout();
  charger_led_cutout();
  display_cutout();

  translate([display_length / 2 - 2.3, 2, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=64, center=false);
  translate([-display_length / 2 + 2.3, 2, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=64, center=false);
  translate([display_length / 2 - 2.5, display_pcb_width - 2, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=64, center=false);
  translate([-display_length / 2 + 2.5, display_pcb_width - 2, -0.1]) cylinder(h=wall * 2, d=3.12, $fn=64, center=false);
}
