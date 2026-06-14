use defmt::info;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;

use crate::DISPLAY_CHANNEL;
use crate::sessions::SessionEvent;

pub const DISPLAY_WIDTH: usize = 256;
pub const DISPLAY_HEIGHT: usize = 64;
pub const PIXEL_COLOR: u8 = 0x60;

const PIXEL_BITS: usize = 4;
const PIXEL_SHIFT: usize = 8 - PIXEL_BITS;
const PIXEL_MASK: u8 = 0xf;
const FB_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * PIXEL_BITS / 8;

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
    pub fn new(spi: Spi<'d, esp_hal::Blocking>, cs: Output<'d>, dc: Output<'d>) -> Self {
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
    buffer: [u8; FB_SIZE],
}

impl<'d> Sh1122Device<'d> {
    pub fn new(interface: HardSpi<'d>) -> Self {
        Self {
            interface,
            buffer: [0; FB_SIZE],
        }
    }

    pub fn init_display(&mut self) -> Result<(), DisplayError> {
        self.display_off()?;
        self.set_row_adr(0)?;
        self.set_col_adr(0)?;
        self.set_start_line(0)?;
        self.set_mux_ratio((DISPLAY_HEIGHT - 1) as u8)?;
        self.set_com_output_scan_dir(0)?;
        self.set_display_offset(0)?;
        self.set_contrast(0x80)?;
        self.set_entire_on()?;
        self.set_inverted(false)?;
        self.display_on()?;
        self.flush()?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), DisplayError> {
        self.set_col_adr(0)?;
        self.set_row_adr(0)?;
        self.interface.write_data(&self.buffer)?;
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

    fn display_off(&mut self) -> Result<(), DisplayError> {
        self.interface.write_cmd(cmd::SET_DISP, &[])
    }

    fn display_on(&mut self) -> Result<(), DisplayError> {
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
    for rectangle in symbol.rects() {
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
    let word = symbol::build_time_word(total_minutes, 0);

    for i in 0..word.count {
        draw_symbol(display, &word.symbols[i]);
    }
}

pub fn render_steps(display: &mut Sh1122Device, value: u32) {
    let word = symbol::build_number_word(value, DISPLAY_WIDTH);

    for i in 0..word.count {
        draw_symbol(display, &word.symbols[i]);
    }
}

#[embassy_executor::task]
pub async fn display_task(
    spi: Spi<'static, esp_hal::Blocking>,
    cs: Output<'static>,
    dc: Output<'static>,
    mut reset: Output<'static>,
) {
    reset.set_low();
    Timer::after(Duration::from_millis(10)).await;
    reset.set_high();
    Timer::after(Duration::from_millis(10)).await;

    let mut device = Sh1122Device::new(HardSpi::new(spi, cs, dc));
    device.init_display().ok();

    loop {
        let event = DISPLAY_CHANNEL.receive().await;
        device.clear();
        match event {
            SessionEvent::Update(update) => {
                info!(
                    "display: session update: week={}min({}steps)",
                    update.week_minutes, update.week_steps
                );
                render_time(&mut device, update.week_minutes);
                render_steps(&mut device, update.week_steps);
            }
            SessionEvent::History(history) => {
                info!(
                    "display: session history: prev={}min({}steps)",
                    history.prev_week_minutes, history.prev_week_steps
                );
                render_time(&mut device, history.prev_week_minutes);
                render_steps(&mut device, history.prev_week_steps);
            }
        }
        device.flush().ok();
    }
}
