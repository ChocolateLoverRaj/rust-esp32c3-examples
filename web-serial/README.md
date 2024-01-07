# Web Serial
Read and write to a serial port on a web page. You can use this to communicate with an ESP32-C3 without installing anything, just by visiting a web page. It even works on Chromebooks without any special setup. In fact, it works even better with ChromeOS than normal Linux distros.

## Instructions
- Flash a program that prints to output on your ESP32-C3
- Connect the ESP32-C3 to your computer
- Make sure you can access `/dev/ttyACM0` if you're using Linux
- Open the web page using a Chromium-based browser. A demo is available at https://serial-output.netlify.app/

## Resources
- https://developer.mozilla.org/en-US/docs/Web/API/Web_Serial_API
- https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/get-started/establish-serial-connection.html