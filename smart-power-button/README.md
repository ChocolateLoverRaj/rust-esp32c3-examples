# Smart Power Button for Desktop Computers
## Why
- I got a gaming computer that I will also remotely use for programming
    - I want to play Overwatch
    - I want to do CPU heavy coding stuff like compiling the Linux kernel from source and working on large Rust projects
    - My Xbox (Xbox One S) can't run Overwatch well
    - My laptop (jinlon (HP Elite c1030 Chromebook)), can't compile code fast
- I want to remotely wake up my computer for programming
    - My gaming computer is connected to the living room TV. It does not have a good spot for programming.
- My computer can't Wake on LAN

## Features
- Remotely press the power and reset buttons of your computer, even when it's shut down or in suspend mode.
- Remotely view the status of the power LED and HDD LED, so you know if it's on / in suspend mode / off.
- Isolated circuits. The ESP32 does not need to have the same power source as the computer. For example, you can power it through USB-C that's connected to a laptop running on battery.
- Turn on the computer automatically when a Bluetooth game controller or other Bluetooth device is on and within range.

## Code setup
Copy `esp/example.env` to `esp/.env`. Edit the file to include your Wi-Fi info and GPIO pin numbers.

## Developing
### Making changes to the web page without flashing web page to ESP
Flashing all the web assets to the ESP takes a long time and wears down the flash more. Instead, do the following:
- In the `esp`, run `cargo r --no-default-features --features std,embassy,esp-idf-svc/native`. This will run without the `static-files` feature, which means that the files built by Trunk will not be included in the ESP code.
- View the output from the ESP to get the IP address of the ESP
- Copy `web/example.env` to `web/.env`. Edit the `WS_HOST` variable to be the ip address of the ESP
- Run `trunk serve` in `web`
- Open the web page served by Trunk in your browser instead of the ESP's server

### Running ESP in release mode to reduce size
Running the `esp` code in with `--release` reduces size, which saves time.

## Flashing Code for Normal Use
- Configure the `.env` files. See [#developing](#developing).
- Run `trunk serve --release` in `web`
- Run `cargo r --release` in `esp`

## Wiring Diagram
![Fritzing Bread Board](./Sketch_bb.svg)

## Pictures of Breadboard
![Photo 0](./Photo%200.webp)
![Photo 1](./Photo%201.webp)
![Photo 2](./Photo%202.webp)
![Photo 3](./Photo%203.webp)
![Photo 4](./Photo%204.webp)
![Photo 5](./Photo%205.webp)
In my setup, I used an ATX power splitter so that I could access the 5VSB and GND pins to always power the ESP32-C3, even when the computer is off.

# PCB
To make the circuit compact and easier to use, I designed a PCB. I ordered the PCB with assembly on PCBWay, which sponsored the PCB prototyping for this project. As seen in the pictures, the quality of the PCB is great. The silkscreen is very clear including the QR Code in the bottom. The only issue is that some of the components were not perfectly aligned. I was stil able to plug in everything I needed to though. You can also order PCBs and PCBs with assembly for your own projects with [PCBWay](https://www.pcbway.com/).
![PCB In Antistatic Bag](./PCB%20In%20Antistatic%20Bag.jpg)
![PCB Top](./PCB%20Top.jpg)
![PCB Bottom](./PCB%20Bottom.jpg)

I made a few mistakes in the PCB design:
- I used GPIO2, which is a special GPIO pin, which did not work as an input. To fix this, I cut off the connection to GPIO2 and soldered the trace to GPIO0 instead.
- The optocopulers for detecting the power and HDD LEDs assume that a 330Ω resistor is connected. This is okay if you get the wiring right, but while I was testing the PCB I accidentally destroyed 2 optocouplers (probably by connecting them to a 5V source without a resistor for a brief moment).
- The LEDs on the PCB and the extra external LEDs were supposed to work even when a ESP32-C3 Super-Mini was not attached, but it doesn't, since it needs a 3.3V source to work, which is only provided by the ESP32-C3 Super-Mini.
- I used resistors which will send 20mA (the maximum current) through the LEDs on the PCB, but this is ***way* too bright**! My PC is in my bedroom, and I had to disconnect it to fall asleep.
![PCB Modification Top](./PCB%20Modification%20Top.jpg)
![PCB Bottom](./PCB%20Modification%20Bottom.jpg)
Here is the PCB in action:
![PCB In Action](./PCB%20In%20Action.jpg)

[Here is the link to the original PCB just for reference](https://easyeda.com/editor#project_id=c5dba1f1b9a34bec985b2a5e179ea9b3) (it has the mistakes). I designed it on EasyEDA.

# Redesigned PCB
After I learned from my mistakes with the original design, I realized a few things:
- Always test the circuit as close as possible as you can test to the actual PCB before ordering the PCB
- The buttons and LEDs on the PCB are unnecessary
- The OLED display is unnecessary
- I do not need to order PCB assembly, I an use through hole components, order a PCB, and solder it myself.
- I should use KiCad instead of EasyEDA.

So I designed a new PCB with KiCad. It is located in the "KiCad PCB" folder. **I have not tested it yet.**

## Materials Needed
(In addition to a ESP32-C3)
- 6x PC817 (or other optocoupler). I bought [this one](https://www.aliexpress.us/item/3256806236608107.html)
- ATX Splitter (optional, for always powering the ESP32-C3 with the PSU). I bought [this one](https://www.aliexpress.us/item/3256805387697490.html?spm=a2g0o.order_list.order_list_main.38.36df1802MgpdVl&gatewayAdapt=glo2usa)
- Breadboard and wires or some way of connecting the components together
