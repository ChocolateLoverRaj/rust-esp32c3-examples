use std::io::{BufRead, Read};
use std::thread;
use std::time::Duration;
use esp_idf_svc::log::EspLogger;
use log::info;
use {
    esp_idf_sys::{esp, esp_vfs_dev_uart_use_driver, uart_driver_install, vTaskDelay},
    std::{
        io::{stdin, stdout, Write},
        ptr::null_mut,
        thread::spawn,
    },
};

fn echo() {
    let mut buffer = [0u8; 8];
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let stdout = stdout();
    let mut stdout_handle = stdout.lock();

    loop {
        match handle.read(&mut buffer) {
            Ok(length) => {
                stdout_handle.write_all(&mut buffer.split_at_mut(length).0).unwrap();
                // print!("{:#?}", buffer);
                // buffer.clear();
            }
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::WouldBlock
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::Interrupted => {
                        // info!("Error: {e}\r\n");
                        unsafe { vTaskDelay(20) };
                    }
                    _ => {
                        // info!("Error: {e}\r\n");
                    }
                }
            }
        }
    }
}

fn main() {
    EspLogger::initialize_default();

    echo();
}