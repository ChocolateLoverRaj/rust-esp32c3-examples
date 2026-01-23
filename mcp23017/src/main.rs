#![no_std]
#![no_main]

use core::{mem, slice};

use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Instant, Timer};
use embedded_hal_async::i2c::I2c;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    i2c::{self},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use pure_rotary_encoder::{Direction, RotaryEncoder, RotaryPinsState};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let p = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(p.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    info!("Hello from a Rust no_std environment with esp_rtos (basically embassy for ESP32).");

    let config = i2c::master::Config::default().with_frequency(Rate::from_khz(400));
    let i2c = Mutex::<CriticalSectionRawMutex, _>::new(
        i2c::master::I2c::new(p.I2C0, Default::default())
            .unwrap()
            .with_scl(p.GPIO5)
            .with_sda(p.GPIO6)
            .into_async(),
    );

    let mut reset_pin = Output::new(p.GPIO8, Level::High, OutputConfig::default());
    reset_pin.set_low();
    Timer::after_micros(1).await;
    reset_pin.set_high();

    let mut interrupt_pin = Input::new(p.GPIO1, InputConfig::default().with_pull(Pull::Up));

    let mut mcp23017 = I2cDeviceWithConfig::new(&i2c, config);
    let address = 0x20;

    // Enable interrupt mirroring, active low, and open-drain
    mcp23017.write(address, &[0x0A, 0b01000100]).await.unwrap();
    // Write to IOCON (Address: 0x0A):
    //  - MIRROR = 1 (interrupt mirroring enabled)
    //  - INTPOL = 0 (interrupts are active low)
    //  - ODR = 1 (interrupt pins are open-drain)

    // Set pin B1 as input
    mcp23017.write(address, &[0x01, 0b00001110]).await.unwrap();
    // Write to IODIRB (Address: 0x11):
    //  - Set B1 as input (1 = input).

    // Enable the pull-up resistor for pin B1
    mcp23017.write(address, &[0x0D, 0b00001110]).await.unwrap();
    // Write to GPPUB (Address: 0x17):
    //  - Enable pull-up resistor for B1.

    // Set the interrupt compare value for pin B1 to HIGH
    // mcp23017.write(address, &[0x07, 0b00000010]).await.unwrap();
    // Write to DEFVALB (Address: 0x15):
    //  - Set DEFVAL for B1 to HIGH (1).

    // Set the interrupt control to compare with DEFVAL
    // mcp23017.write(address, &[0x09, 0b00000010]).await.unwrap();
    // Write to INTCONB (Address: 0x13):
    //  - Compare B1 value against DEFVAL (not previous state).

    // Enable interrupts for pin B1
    mcp23017.write(address, &[0x05, 0b00001110]).await.unwrap();
    // Write to GPINTENB (Address: 0x19):
    //  - Enable interrupt-on-change for B1.

    let mut last_read = Instant::now();
    let mut rotary_encoder = None;
    let mut position = 0_i64;
    loop {
        // Read the value
        let mut byte = Default::default();
        mcp23017
            .write_read(address, &[0x13], slice::from_mut(&mut byte))
            .await
            .unwrap();
        let now = Instant::now();
        let last_read = mem::replace(&mut last_read, now);
        let sw = (byte & 1 << 1) != 0;
        let dt = (byte & 1 << 2) != 0;
        let clk = (byte & 1 << 3) != 0;
        let rotary_pins = RotaryPinsState { dt, clk };
        let direction = rotary_encoder
            .get_or_insert(RotaryEncoder::new(rotary_pins))
            .process_data(rotary_pins);
        if let Some(direction) = direction {
            position += match direction {
                Direction::Clockwise => 1,
                Direction::CounterClockwise => -1,
            };
        }
        info!(
            "sw = {}, dt = {}, clk = {}, direction = {}, position = {}, duration = {} us",
            u8::from(sw),
            u8::from(dt),
            u8::from(clk),
            direction,
            position,
            (now - last_read).as_micros(),
        );
        interrupt_pin.wait_for_low().await;
    }
}
