#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

extern crate alloc;

use alloc::boxed::Box;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::main;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::time::Rate;
use mipidsi::{
    Builder,
    interface::SpiInterface,
    models::ILI9342CRgb565,
    options::{Orientation, Rotation},
};
use mousefood::EmbeddedBackend;
use ratatui::Terminal;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    loop {
        log::error!("{panic_info:?}");
    }
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 128 * 1024);

    // Turn on display backlight
    let _backlight = Output::new(peripherals.GPIO4, Level::High, OutputConfig::default());

    // Configure SPI
    log::info!("Initializing SPI display");
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(80))
            .with_mode(Mode::_3),
    )
    .unwrap()
    .with_sck(peripherals.GPIO18)
    .with_mosi(peripherals.GPIO19);

    let cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());
    let spi_device = embedded_hal_bus::spi::ExclusiveDevice::new(spi, cs, Delay::new()).unwrap();

    let dc = Output::new(peripherals.GPIO16, Level::High, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO23, Level::High, OutputConfig::default());
    let buffer = Box::leak(Box::new([0_u8; 4096]));
    let spi_interface = SpiInterface::new(spi_device, dc, buffer);

    // Configure display
    let mut delay = Delay::new();
    let mut display = Builder::new(ILI9342CRgb565, spi_interface)
        .reset_pin(rst)
        .init(&mut delay)
        .expect("Failed to init display");

    display
        .set_orientation(Orientation {
            rotation: Rotation::Deg270,
            mirrored: true,
        })
        .unwrap();

    log::info!("Starting render");

    let backend = EmbeddedBackend::new(&mut display, Default::default());
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let p = Paragraph::new("Hello").style(Style::default());
            f.render_widget(p, f.area());
        })
        .ok();

    loop {
        delay.delay_millis(1000);
    }
}
