use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin, Output, PinDriver};
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver, config::DriverConfig};
use esp_idf_svc::sys::EspError;
use sh1122::{Framebuffer, Sh1122Device, Sh1122Interface};

pub const DISPLAY_WIDTH: usize = 256;
pub const DISPLAY_HEIGHT: usize = 64;
pub const BRIGHTNESS: u8 = 0x60;
pub const DIGIT_SEGMENTS: [u8; 10] = [
    0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111,
    0b1111111, 0b1101111,
];

pub struct HardSpi<'d> {
    spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
    cs: PinDriver<'d, Output>,
    dc: PinDriver<'d, Output>,
}

impl<'d> HardSpi<'d> {
    pub fn new(
        spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
        pin_cs: impl esp_idf_svc::hal::gpio::OutputPin + 'd,
        pin_dc: impl esp_idf_svc::hal::gpio::OutputPin + 'd,
    ) -> Result<Self, EspError> {
        Ok(Self {
            spi,
            cs: PinDriver::output(pin_cs)?,
            dc: PinDriver::output(pin_dc)?,
        })
    }
}

impl<'d> Sh1122Interface for HardSpi<'d> {
    fn write_cmd(&mut self, command: u8, data: &[u8]) -> anyhow::Result<()> {
        self.cs.set_low()?;
        self.dc.set_low()?;
        self.spi.write(&[command])?;
        if !data.is_empty() {
            self.spi.write(data)?;
        }
        self.cs.set_high()?;
        Ok(())
    }

    fn write_data(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.cs.set_low()?;
        self.dc.set_high()?;
        self.spi.write(data)?;
        self.cs.set_high()?;
        Ok(())
    }
}

pub fn init_spi<'d, SCLK, MOSI, SPI>(
    spi2: SPI,
    sclk: SCLK,
    mosi: MOSI,
    miso: Option<AnyIOPin<'d>>,
) -> Result<SpiDeviceDriver<'d, SpiDriver<'d>>, EspError>
where
    SCLK: esp_idf_svc::hal::gpio::OutputPin + 'd,
    MOSI: esp_idf_svc::hal::gpio::OutputPin + 'd,
    SPI: esp_idf_svc::hal::spi::Spi + esp_idf_svc::hal::spi::SpiAnyPins + 'd,
{
    let spi_driver = SpiDriver::new(spi2, sclk, mosi, miso, &DriverConfig::default())?;
    SpiDeviceDriver::new(spi_driver, None::<AnyOutputPin>, &Default::default())
}

pub fn create_display<'d>(
    spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
    pin_cs: impl esp_idf_svc::hal::gpio::OutputPin + 'd,
    pin_dc: impl esp_idf_svc::hal::gpio::OutputPin + 'd,
) -> Result<Sh1122Device<HardSpi<'d>>, EspError> {
    let spi_interface = HardSpi::new(spi, pin_cs, pin_dc)?;
    let mut display = Sh1122Device::with_interface(spi_interface, DISPLAY_WIDTH, DISPLAY_HEIGHT);
    display.init_display().ok();
    Ok(display)
}

pub fn draw_digit<D: Sh1122Interface>(
    display: &mut Sh1122Device<D>,
    bits: u8,
    x: usize,
    color: u8,
) {
    if bits & 1 != 0 {
        draw_rect(display, x + 10, 0, 14, 4, color);
    }
    if bits & 2 != 0 {
        draw_rect(display, x + 24, 4, 4, 26, color);
    }
    if bits & 4 != 0 {
        draw_rect(display, x + 24, 34, 4, 26, color);
    }
    if bits & 8 != 0 {
        draw_rect(display, x + 10, 60, 14, 4, color);
    }
    if bits & 16 != 0 {
        draw_rect(display, x + 8, 34, 4, 26, color);
    }
    if bits & 32 != 0 {
        draw_rect(display, x + 8, 4, 4, 26, color);
    }
    if bits & 64 != 0 {
        draw_rect(display, x + 10, 30, 14, 4, color);
    }
}

pub fn draw_colon<D: Sh1122Interface>(display: &mut Sh1122Device<D>, x: usize, color: u8) {
    draw_rect(display, x + 10, 26, 4, 4, color);
    draw_rect(display, x + 10, 34, 4, 4, color);
}

pub fn draw_rect<D: Sh1122Interface>(
    display: &mut Sh1122Device<D>,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u8,
) {
    for xi in x..x + width {
        for yi in y..y + height {
            display.set_pixel(xi, yi, color);
        }
    }
}

pub fn render_time<D: Sh1122Interface>(display: &mut Sh1122Device<D>, minutes: u32, color: u8) {
    let tens = (minutes / 10) as usize;
    let ones = (minutes % 10) as usize;
    draw_digit(display, DIGIT_SEGMENTS[tens.min(9)], 0, color);
    draw_colon(display, 26, color);
    draw_digit(display, DIGIT_SEGMENTS[ones.min(9)], 40, color);
}
