use esp_idf_hal::{
    gpio::{InterruptType, PinDriver, Pull},
    peripherals::Peripherals,
    task::block_on,
};
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;

fn main() {
    block_on(main_async());
}

async fn main_async() {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    let nvs_default_partition = EspNvsPartition::<NvsDefault>::take().unwrap();

    let namespace = "led_namespace";
    let nvs = match EspNvs::new(nvs_default_partition, namespace, true) {
        Ok(nvs) => {
            println!("Got namespace {:?} from default partition", namespace);
            nvs
        }
        Err(e) => panic!("Could't get namespace {:?}", e),
    };
    let tag = "is_on";

    let peripherals = Peripherals::take().unwrap();
    let mut led = PinDriver::input_output(peripherals.pins.gpio8).unwrap();
    let mut button = PinDriver::input(peripherals.pins.gpio9).unwrap();
    button.set_pull(Pull::Down).unwrap();
    button.set_interrupt_type(InterruptType::PosEdge).unwrap();
    button.enable_interrupt().unwrap();

    match nvs.get_u8(tag).unwrap() {
        Some(is_on) => {
            let is_on = is_on == 1;
            println!("Got stored value for is_on: {}", is_on);
            led.set_level((!is_on).into()).unwrap();
        }
        None => {
            println!("No stored value for is_on. Storing default value.");
            nvs.set_u8(tag, 0).unwrap();
            led.set_high().unwrap();
        }
    }

    loop {
        button.wait_for_rising_edge().await.unwrap();
        led.toggle().unwrap();
        let is_on = led.is_low();
        println!("Storing new value for is_on: {}", is_on);
        nvs.set_u8(tag, is_on as u8).unwrap();
    }
}
