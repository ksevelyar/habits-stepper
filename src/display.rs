use defmt::info;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;
use static_cell::ConstStaticCell;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

use crate::sessions::WeekTotals;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Size};
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_10X20};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{Drawable, Pixel, Point};
use embedded_graphics::text::{Baseline, Text};

pub const DISPLAY_WIDTH: usize = 256;
pub const DISPLAY_HEIGHT: usize = 64;
pub const PIXEL_COLOR: u8 = 0x60;

const PIXEL_BITS: usize = 4;
const PIXEL_SHIFT: usize = 8 - PIXEL_BITS;
const PIXEL_MASK: u8 = 0xf;
const FB_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * PIXEL_BITS / 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupStage {
    Wifi,
    Dhcp,
    Dns,
    Ntp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisplayEvent {
    Startup(StartupStage),
    Update(WeekTotals),
    History(WeekTotals),
}

pub static DISPLAY_CHANNEL: Channel<CriticalSectionRawMutex, DisplayEvent, 3> = Channel::new();
static STARTUP_DISPLAYED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub async fn show_startup(stage: StartupStage) {
    DISPLAY_CHANNEL.send(DisplayEvent::Startup(stage)).await;
    STARTUP_DISPLAYED.wait().await;
}

mod symbol;

use symbol::Symbol;

mod cmd {
    pub const SET_COL_ADR_LSB: u8 = 0x00;
    pub const SET_COL_ADR_MSB: u8 = 0x10;
    pub const SET_DISP_START_LINE: u8 = 0x40;
    pub const SET_CONTRAST: u8 = 0x81;
    pub const SET_ENTIRE_ON: u8 = 0xA4;
    pub const SET_NORM_INV: u8 = 0xA6;
    pub const SET_MUX_RATIO: u8 = 0xA8;
    pub const SET_DISP: u8 = 0xAE;
    pub const SET_ROW_ADR: u8 = 0xB0;
    pub const SET_COM_OUT_DIR: u8 = 0xC0;
    pub const SET_DISP_OFFSET: u8 = 0xD3;
}

#[derive(Debug, defmt::Format)]
pub enum DisplayError {
    Spi,
}

pub struct HardSpi<'d> {
    spi: Spi<'d, esp_hal::Blocking>,
    cs: Output<'d>,
    dc: Output<'d>,
}

impl<'d> HardSpi<'d> {
    pub fn create(spi: Spi<'d, esp_hal::Blocking>, cs: Output<'d>, dc: Output<'d>) -> Self {
        Self { spi, cs, dc }
    }

    fn write_cmd(&mut self, command: u8, data: &[u8]) -> Result<(), DisplayError> {
        self.cs.set_low();
        self.dc.set_low();
        self.spi.write(&[command]).map_err(|_| DisplayError::Spi)?;
        if !data.is_empty() {
            self.spi.write(data).map_err(|_| DisplayError::Spi)?;
        }
        self.cs.set_high();
        Ok(())
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), DisplayError> {
        self.cs.set_low();
        self.dc.set_high();
        self.spi.write(data).map_err(|_| DisplayError::Spi)?;
        self.cs.set_high();
        Ok(())
    }
}

pub struct Sh1122Device<'d> {
    interface: HardSpi<'d>,
    buffer: &'d mut [u8; FB_SIZE],
}

impl<'d> Sh1122Device<'d> {
    pub fn create(interface: HardSpi<'d>, buffer: &'d mut [u8; FB_SIZE]) -> Self {
        Self { interface, buffer }
    }

    pub fn init_display(&mut self) -> Result<(), DisplayError> {
        self.turn_display_off()?;
        self.set_row_adr(0)?;
        self.set_col_adr(0)?;
        self.set_start_line(0)?;
        self.set_mux_ratio((DISPLAY_HEIGHT - 1) as u8)?;
        self.set_com_output_scan_dir(0)?;
        self.set_display_offset(0)?;
        self.set_contrast(0x80)?;
        self.set_entire_on()?;
        self.set_inverted(false)?;
        self.turn_display_on()?;
        self.flush()?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), DisplayError> {
        self.set_col_adr(0)?;
        self.set_row_adr(0)?;
        self.interface.write_data(self.buffer)?;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0);
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, pixel: u8) {
        if x >= DISPLAY_WIDTH || y >= DISPLAY_HEIGHT {
            return;
        }
        let index = x + y * DISPLAY_WIDTH;
        let byte_index = index * PIXEL_BITS / 8;
        let bit_index = 8 - PIXEL_BITS - PIXEL_BITS * (index - byte_index * 8 / PIXEL_BITS);
        let mask = PIXEL_MASK << bit_index;
        self.buffer[byte_index] =
            (self.buffer[byte_index] & !mask) | ((pixel >> PIXEL_SHIFT) << bit_index);
    }

    fn turn_display_off(&mut self) -> Result<(), DisplayError> {
        self.interface.write_cmd(cmd::SET_DISP, &[])
    }

    fn turn_display_on(&mut self) -> Result<(), DisplayError> {
        self.interface.write_cmd(cmd::SET_DISP | 0x01, &[])
    }

    fn set_col_adr(&mut self, column: u8) -> Result<(), DisplayError> {
        self.interface
            .write_cmd(cmd::SET_COL_ADR_LSB | (column & 0x0f), &[])?;
        self.interface
            .write_cmd(cmd::SET_COL_ADR_MSB | ((column >> 4) & 0x0f), &[])?;
        Ok(())
    }

    fn set_row_adr(&mut self, row: u8) -> Result<(), DisplayError> {
        self.interface.write_cmd(cmd::SET_ROW_ADR, &[row])
    }

    fn set_start_line(&mut self, line: u8) -> Result<(), DisplayError> {
        self.interface
            .write_cmd(cmd::SET_DISP_START_LINE | (line & 0x3f), &[])
    }

    fn set_mux_ratio(&mut self, ratio: u8) -> Result<(), DisplayError> {
        self.interface.write_cmd(cmd::SET_MUX_RATIO, &[ratio])
    }

    fn set_com_output_scan_dir(&mut self, direction: u8) -> Result<(), DisplayError> {
        self.interface
            .write_cmd(cmd::SET_COM_OUT_DIR | (direction & 0x01), &[])
    }

    fn set_display_offset(&mut self, offset: u8) -> Result<(), DisplayError> {
        self.interface.write_cmd(cmd::SET_DISP_OFFSET, &[offset])
    }

    pub fn set_contrast(&mut self, contrast: u8) -> Result<(), DisplayError> {
        self.interface.write_cmd(cmd::SET_CONTRAST, &[contrast])
    }

    fn set_entire_on(&mut self) -> Result<(), DisplayError> {
        self.interface.write_cmd(cmd::SET_ENTIRE_ON, &[])
    }

    pub fn set_inverted(&mut self, inverted: bool) -> Result<(), DisplayError> {
        self.interface
            .write_cmd(cmd::SET_NORM_INV | u8::from(inverted), &[])
    }
}

fn draw_symbol(display: &mut Sh1122Device, symbol: &Symbol) {
    for rectangle in symbol.iterate_rectangles() {
        draw_rect(
            display,
            symbol.x + rectangle.x,
            rectangle.y,
            rectangle.width,
            rectangle.height,
        );
    }
}

pub fn draw_rect(display: &mut Sh1122Device, x: usize, y: usize, width: usize, height: usize) {
    for xi in x..x + width {
        for yi in y..y + height {
            display.set_pixel(xi, yi, PIXEL_COLOR);
        }
    }
}

pub fn render_time(display: &mut Sh1122Device, total_minutes: u32) {
    for symbol in symbol::build_time(total_minutes, 0) {
        draw_symbol(display, &symbol);
    }
}

pub fn render_steps(display: &mut Sh1122Device, value: u32) {
    for symbol in symbol::build_number(value, DISPLAY_WIDTH) {
        draw_symbol(display, &symbol);
    }
}

struct ScaledDisplay<'display, 'device> {
    display: &'display mut Sh1122Device<'device>,
    scale: usize,
    left: usize,
    top: usize,
}

impl OriginDimensions for ScaledDisplay<'_, '_> {
    fn size(&self) -> Size {
        Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32)
    }
}

impl DrawTarget for ScaledDisplay<'_, '_> {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<Pixels>(&mut self, pixels: Pixels) -> Result<(), Self::Error>
    where
        Pixels: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if color == BinaryColor::On && point.x >= 0 && point.y >= 0 {
                draw_rect(
                    self.display,
                    self.left + point.x as usize * self.scale,
                    self.top + point.y as usize * self.scale,
                    self.scale,
                    self.scale,
                );
            }
        }
        Ok(())
    }
}

fn render_startup(display: &mut Sh1122Device<'_>, stage: StartupStage) {
    let text = match stage {
        StartupStage::Wifi => "WIFI",
        StartupStage::Dhcp => "DHCP",
        StartupStage::Dns => "DNS",
        StartupStage::Ntp => "NTP",
    };
    let character_width = FONT_10X20.character_size.width as usize;
    let character_height = FONT_10X20.character_size.height as usize;
    let text_width = text.len() * character_width;
    let scale = (DISPLAY_HEIGHT / character_height).min(DISPLAY_WIDTH / text_width);
    let left = (DISPLAY_WIDTH - text_width * scale) / 2;
    let top = (DISPLAY_HEIGHT - character_height * scale) / 2;
    let mut target = ScaledDisplay {
        display,
        scale,
        left,
        top,
    };
    let style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
    Text::with_baseline(text, Point::zero(), style, Baseline::Top)
        .draw(&mut target)
        .expect("drawing to the display framebuffer cannot fail");
}

pub trait Display {
    fn show(&mut self, event: &DisplayEvent) -> Result<(), DisplayError>;
}

impl Display for Sh1122Device<'_> {
    fn show(&mut self, event: &DisplayEvent) -> Result<(), DisplayError> {
        self.clear();
        match event {
            DisplayEvent::Startup(stage) => render_startup(self, *stage),
            DisplayEvent::Update(totals) => {
                info!(
                    "display: session update: {}min({}steps)",
                    totals.minutes, totals.steps
                );
                render_time(self, totals.minutes);
                render_steps(self, totals.steps);
            }
            DisplayEvent::History(totals) => {
                info!(
                    "display: session history: {}min({}steps)",
                    totals.minutes, totals.steps
                );
                render_time(self, totals.minutes);
                render_steps(self, totals.steps);
            }
        }
        self.flush()
    }
}

pub async fn run_display<DisplayDevice: Display>(display: &mut DisplayDevice) {
    loop {
        let event = DISPLAY_CHANNEL.receive().await;
        while display.show(&event).is_err() {
            info!("display: flush failed");
            Timer::after(Duration::from_millis(100)).await;
        }
        if matches!(event, DisplayEvent::Startup(_)) {
            STARTUP_DISPLAYED.signal(());
        }
    }
}

pub struct DisplayHardware {
    pub spi: Spi<'static, esp_hal::Blocking>,
    pub chip_select: Output<'static>,
    pub data_command: Output<'static>,
    pub reset: Output<'static>,
}

pub async fn initialize_display(mut hardware: DisplayHardware) -> Sh1122Device<'static> {
    static FRAMEBUFFER: ConstStaticCell<[u8; FB_SIZE]> = ConstStaticCell::new([0; FB_SIZE]);

    hardware.reset.set_low();
    Timer::after(Duration::from_millis(10)).await;
    hardware.reset.set_high();
    Timer::after(Duration::from_millis(10)).await;

    let mut device = Sh1122Device::create(
        HardSpi::create(hardware.spi, hardware.chip_select, hardware.data_command),
        FRAMEBUFFER.take(),
    );
    device.init_display().ok();

    device
}

#[embassy_executor::task]
pub async fn run_display_task(hardware: DisplayHardware) {
    let mut device = initialize_display(hardware).await;
    run_display(&mut device).await;
}
