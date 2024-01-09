# External LED Control
## Stores state
Remember if the LED was on or off after a reboot.

## Button
Press the boot button to toggle the LED.

## USB
Write "on", "off", or "toggle", then new line through USB to control the LED. You will get updates on if the LED is on or off.

## BLE
Connect to BLE to read, write, and subscribe to notifications of the LED state.

## Resources
- https://github.com/espressif/esp-idf/blob/5524b692ee5d04d7a1000eb0c41640746fc67f3c/examples/storage/nvs_rw_value/main/nvs_value_example_main.c
- https://github.com/esp-rs/esp-idf-svc/blob/master/examples/nvs_get_set_c_style.rs
- https://github.com/taks/esp32-nimble
