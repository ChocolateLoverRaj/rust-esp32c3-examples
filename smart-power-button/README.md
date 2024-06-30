# Smart Power Button for Desktop Computers
## Why
- My computer doesn't have Wake on LAN

## Features
- Remotely press the power and reset buttons of your computer, even when it's shut down or in suspend mode.
- Remotely view the status of the power LED and HDD LED, so you know if it's on / in suspend mode / off.

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

## Wiring Diagram and Pictures
Will be added soon
