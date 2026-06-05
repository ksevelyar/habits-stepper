use defmt::info;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;

use crate::DISPLAY_CHANNEL;
use crate::sessions::SessionEvent;

pub const DISPLAY_WIDTH: usize = 256;
pub const DISPLAY_HEIGHT: usize = 64;
pub const BRIGHTNESS: u8 = 0x60;

const PIXEL_BITS: usize = 4;
const PIXEL_SHIFT: usize = 8 - PIXEL_BITS;
const PIXEL_MASK: u8 = 0xf;
const FB_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * PIXEL_BITS / 8;

pub const DIGIT_SEGMENTS: [u8; 10] = [
    0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111,
    0b1111111, 0b1101111,
];

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
    iface: HardSpi<'d>,
    buf: [u8; FB_SIZE],
}

impl<'d> Sh1122Device<'d> {
    pub fn new(iface: HardSpi<'d>) -> Self {
        Self {
            iface,
            buf: [0; FB_SIZE],
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
        self.iface.write_data(&self.buf)?;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.buf.fill(0);
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, pixel: u8) {
        if x >= DISPLAY_WIDTH || y >= DISPLAY_HEIGHT {
            return;
        }
        let idx = x + y * DISPLAY_WIDTH;
        let byte_idx = idx * PIXEL_BITS / 8;
        let bit_idx = 8 - PIXEL_BITS - PIXEL_BITS * (idx - byte_idx * 8 / PIXEL_BITS);
        let mask = PIXEL_MASK << bit_idx;
        self.buf[byte_idx] = (self.buf[byte_idx] & !mask) | ((pixel >> PIXEL_SHIFT) << bit_idx);
    }

    fn display_off(&mut self) -> Result<(), DisplayError> {
        self.iface.write_cmd(cmd::SET_DISP, &[])
    }

    fn display_on(&mut self) -> Result<(), DisplayError> {
        self.iface.write_cmd(cmd::SET_DISP | 0x01, &[])
    }

    fn set_col_adr(&mut self, col: u8) -> Result<(), DisplayError> {
        self.iface
            .write_cmd(cmd::SET_COL_ADR_LSB | (col & 0x0f), &[])?;
        self.iface
            .write_cmd(cmd::SET_COL_ADR_MSB | ((col >> 4) & 0x0f), &[])?;
        Ok(())
    }

    fn set_row_adr(&mut self, row: u8) -> Result<(), DisplayError> {
        self.iface.write_cmd(cmd::SET_ROW_ADR, &[row])
    }

    fn set_start_line(&mut self, line: u8) -> Result<(), DisplayError> {
        self.iface
            .write_cmd(cmd::SET_DISP_START_LINE | (line & 0x3f), &[])
    }

    fn set_mux_ratio(&mut self, ratio: u8) -> Result<(), DisplayError> {
        self.iface.write_cmd(cmd::SET_MUX_RATIO, &[ratio])
    }

    fn set_com_output_scan_dir(&mut self, dir: u8) -> Result<(), DisplayError> {
        self.iface
            .write_cmd(cmd::SET_COM_OUT_DIR | (dir & 0x01), &[])
    }

    fn set_display_offset(&mut self, offset: u8) -> Result<(), DisplayError> {
        self.iface.write_cmd(cmd::SET_DISP_OFFSET, &[offset])
    }

    pub fn set_contrast(&mut self, contrast: u8) -> Result<(), DisplayError> {
        self.iface.write_cmd(cmd::SET_CONTRAST, &[contrast])
    }

    fn set_entire_on(&mut self) -> Result<(), DisplayError> {
        self.iface.write_cmd(cmd::SET_ENTIRE_ON, &[])
    }

    pub fn set_inverted(&mut self, inverted: bool) -> Result<(), DisplayError> {
        self.iface
            .write_cmd(cmd::SET_NORM_INV | u8::from(inverted), &[])
    }
}

pub fn draw_digit(display: &mut Sh1122Device, bits: u8, x: usize) {
    if bits & 1 != 0 {
        draw_rect(display, x + 10, 0, 14, 4);
    }
    if bits & 2 != 0 {
        draw_rect(display, x + 24, 4, 4, 26);
    }
    if bits & 4 != 0 {
        draw_rect(display, x + 24, 34, 4, 26);
    }
    if bits & 8 != 0 {
        draw_rect(display, x + 10, 60, 14, 4);
    }
    if bits & 16 != 0 {
        draw_rect(display, x + 8, 34, 4, 26);
    }
    if bits & 32 != 0 {
        draw_rect(display, x + 8, 4, 4, 26);
    }
    if bits & 64 != 0 {
        draw_rect(display, x + 10, 30, 14, 4);
    }
}

pub fn draw_colon(display: &mut Sh1122Device, x: usize) {
    draw_rect(display, x + 10, 26, 4, 4);
    draw_rect(display, x + 10, 34, 4, 4);
}

pub fn draw_rect(display: &mut Sh1122Device, x: usize, y: usize, width: usize, height: usize) {
    for xi in x..x + width {
        for yi in y..y + height {
            display.set_pixel(xi, yi, BRIGHTNESS);
        }
    }
}

pub fn render_time(display: &mut Sh1122Device, minutes: u32, x: usize) {
    let hours = (minutes / 60) as usize;
    let minutes = minutes % 60;
    let tens = (minutes / 10) as usize;
    let ones = (minutes % 10) as usize;

    draw_digit(display, DIGIT_SEGMENTS[hours], x);
    draw_colon(display, x + 26);
    draw_digit(display, DIGIT_SEGMENTS[tens], x + 40);
    draw_digit(display, DIGIT_SEGMENTS[ones], x + 70);
}

#[embassy_executor::task]
pub async fn display_task(
    spi: Spi<'static, esp_hal::Blocking>,
    cs: Output<'static>,
    dc: Output<'static>,
    mut rst: Output<'static>,
) {
    rst.set_low();
    Timer::after(Duration::from_millis(10)).await;
    rst.set_high();
    Timer::after(Duration::from_millis(10)).await;

    let mut device = Sh1122Device::new(HardSpi::new(spi, cs, dc));
    device.init_display().ok();

    loop {
        let event = DISPLAY_CHANNEL.receive().await;
        device.clear();
        match event {
            SessionEvent::Update(update) => {
                info!(
                    "SessionUpdate: today={}min({}steps) week={}min",
                    update.today_minutes, update.today_steps, update.week_minutes
                );
                render_time(&mut device, update.today_minutes, 80);
            }
            SessionEvent::History(history) => {
                info!(
                    "SessionHistory: w1={}min w2={}min w3={}min",
                    history.week1_minutes, history.week2_minutes, history.week3_minutes
                );
                render_time(&mut device, history.week1_minutes, 10);
                render_time(&mut device, history.week2_minutes, 90);
                render_time(&mut device, history.week3_minutes, 170);
            }
        }
        device.flush().ok();
    }
}
