use esp_idf_hal::{
    gpio::{InterruptType, Level, PinDriver, Pull},
    peripherals::Peripherals,
    task::block_on,
};
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use futures::{prelude::*, stream::select_all};
use std::{io::Error, pin::Pin, thread, time::Duration};
use stdin::get_stdin_stream;

mod async_timeout;
mod stdin;
mod timer_stream;

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

    fn on_to_level(on: bool) -> Level {
        match on {
            true => Level::Low,
            false => Level::High,
        }
    }

    match nvs.get_u8(tag).unwrap() {
        Some(is_on) => {
            let is_on = is_on == 1;
            println!("Got stored value for is_on: {}", is_on);
            led.set_level(on_to_level(is_on)).unwrap();
        }
        None => {
            println!("No stored value for is_on. Storing default value.");
            nvs.set_u8(tag, 0).unwrap();
            led.set_high().unwrap();
        }
    }

    // loop {
    //     button.wait_for_rising_edge().await.unwrap();
    //     led.toggle().unwrap();
    //     let is_on = led.is_low();
    //     println!("Storing new value for is_on: {}", is_on);
    //     nvs.set_u8(tag, is_on as u8).unwrap();
    // }

    // struct Chunk {
    //     buf: [u8; 8],
    //     size: usize,
    // }
    // let (mut tx, rx) = channel::<Chunk>(1);

    // let _handle = spawn(move || {
    //     block_on(async {
    //         let mut usb = Usb::new();
    //         loop {
    //             let mut buf = [0u8; 8];
    //             let size = usb.read(&mut buf).unwrap();
    //             if size > 0 {
    //                 tx.send(Chunk { buf, size }).await.unwrap();
    //             }
    //             sleep(Duration::from_millis(10));
    //         }
    //     })
    // });

    // let mut stream = unfold(rx, move |mut rx| async move {
    //     let chunk = rx.next().await.unwrap();
    //     if chunk.size > 0 {
    //         Some((Ok(chunk.buf[..chunk.size].to_owned()), rx))
    //     } else {
    //         None
    //     }
    // })
    // .boxed()
    // .into_async_read()
    // .lines();

    // let mut usb_stream = UsbStream::new(usb, 1024);

    struct M;

    impl Iterator for M {
        type Item = Result<[u8; 1], Error>;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                let byte = unsafe { libc::getchar() };
                if byte != -1 {
                    return Some(Ok([byte as u8]));
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    }

    // let mut stream = TryStreamExt::into_async_read(stream::iter(M {})).lines();

    let (line_stream, _stop_reading_stdin) = get_stdin_stream(Duration::from_millis(10));
    let stream = line_stream
        .map(|byte| Ok::<[u8; 1], Error>([byte]))
        .into_async_read()
        .lines()
        .filter_map(|line| async {
            if let Ok(line) = line {
                match line.as_str() {
                    "on" => Some(true),
                    "off" => Some(false),
                    _ => None,
                }
            } else {
                None
            }
        });

    let event_streams: Vec<Pin<Box<dyn Stream<Item = bool>>>> = vec![Box::pin(stream)];
    let mut event_stream = select_all(event_streams);

    loop {
        let on = event_stream.next().await.unwrap();
        led.set_level(on_to_level(on)).unwrap();
        println!("Storing new value for is_on: {}", on);
        nvs.set_u8(tag, on as u8).unwrap();
    }
}
