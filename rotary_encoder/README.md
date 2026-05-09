# Rotary Encoder
Receive the rotation of a knob input using a rotary encoder. This example uses a KY-040 rotary encoder specifically. You can't get the position / angle of the knob, but you can process how much the knob rotates. In this example, an LED is dimmed using PWM, and you can adjust the brightness by turning the knob.

## Microcontrollers in this example
- stm32f103c8

## Wiring Diagram
![Fritzing Wiring Diagram](./Sketch_bb.svg)

## Materials needed
- KY-040
- Jumper wires

## Resources
- https://lastminuteengineers.com/rotary-encoder-arduino-tutorial/
- https://dev.to/theembeddedrustacean/embedded-rust-embassy-gpio-button-controlled-blinking-3ee6
- https://blog.theembeddedrustacean.com/embedded-rust-embassy-pwm-generation
