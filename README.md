# IR Emitter and Receiver with ESP32-C3 and Rust
Use an ESP32-C3 to check if an IR receiver is receiving light from an IR emitter
![Emitter and Receiver](https://github.com/ChocolateLoverRaj/rust-esp32c3-ir-led/blob/main/Picture.png?raw=true)

## Materials Needed
- ESP32-C3 (I bought [this one](https://www.aliexpress.us/item/3256805941172619.html) and [this one](https://www.aliexpress.us/item/3256805870348476.html))
- USB Cable to connect the ESP32-C3 to your computer. You can use a USB-A to USB-C cable or a USB-C to USB-C cable, they both work.
- Breadboard
- Jumper wires
- IR Emitter (I bought [this one](), the 3mm 940nm version)
- IR Receiver (I bought [this one](), the 3mm version)
- Resistors - 220Ω, 4.7kΩ, 10kΩ. They don't have to have the exact values as listed.

## Try out the code
- Clone this repo
- Install Rust
- You may need to install some Rust stuff for ESP32-C3
- `cd digital` or `cd analog`
- `cargo run`

## Resources
This project is based off of these resources

### Related to IR Emitter and Receiver
- https://www.instructables.com/DIY-Photogate/
- https://tutorials-raspberrypi.com/photoresistor-brightness-light-sensor-with-raspberry-pi/

### Rust on ESP32-C3
- https://github.com/ivmarkov/rust-esp32-std-demo
- https://github.com/shanemmattner/ESP32-C3_Rust_Tutorials
