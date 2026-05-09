# Programming ESP32-C3 with Rust
Examples with wiring diagrams for using some sensors with ESP32-C3, programmed with Rust
![ESP32-C3 with Rust Logo](./ESP32-C3.png)

## Materials Needed
- ESP32-C3 (I bought [this one from AliExpress](https://www.aliexpress.us/item/3256805941172619.html), and I recommend buying it from "TENSTAR ROBOT")
- USB Cable to connect the ESP32-C3 to your computer. You can use a USB-A to USB-C cable or a USB-C to USB-C cable, they both work.
- Project-specific materials

## Try out the code
- Clone this repo
- Install Rust
- You may need to install some Rust stuff for ESP32-C3
- Plug in your ESP32-C3
- Make sure you have the permissions to access ESP32-C3 (`/dev/ttyACM0`)
- `cd` to the directory you want to run
- `cargo run`

## Supported Chips / Boards
This was originally only for the ESP32-C3, but it should be really easy to add support for other ESP32 chips / boards. Some examples already have support for the ESP32-S3 in addition to the C3, and there is one example for STM32, but it should be really easy to add support for other ESP32 chips / boards. Some examples already have support for the ESP32-S3 in addition to the C3, and there is one example for STM32. If you would like an example for a specific chip / board to be added, let me know. I have a ESP32-C3, ESP32-S3, and a stm32f103c8.

## `std` vs `no_std`
On ESP32 chips you can use Rust with `std` or `no_std`. Originally, every project in this repo used `std`. At some point I switched to using `no_std`. I don't recommend using `std` anymore. The old examples still use `std`. If you would like a `no_std` version of an example that is currently `std` only, let me know.

## On-demand Maintanance Mode
I want to maintain this repo to help people do stuff with their ESP32s. However, not all examples will be up to date. If there is an example that uses an out-of-date `esp-hal` version, is not working, or not working on your chip, or if you have a question, don't hesistate to create an issue. 

## Resources
This project is based off of these resources

### Rust on ESP32-C3
- [The Rust on ESP Book](https://docs.espressif.com/projects/rust/book/)
- [Embedded Rust (no_std) on Espressif](https://docs.espressif.com/projects/rust/no_std-training/)
- https://github.com/ivmarkov/rust-esp32-std-demo
- https://github.com/shanemmattner/ESP32-C3_Rust_Tutorials

### Project-specific resources
Can be found in the `README.md` files in folders
