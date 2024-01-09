// use std::io::{Read, Write};

use std::{
    thread::{sleep, spawn},
    time::Duration,
};

use esp_idf_hal::task::block_on;
use futures::channel::oneshot::{self};
use futures::stream::StreamExt;
use futures::Stream;
use futures::{channel::mpsc::channel, stream::unfold, SinkExt};

// pub struct Usb;

// impl Usb {
//     pub fn new() -> Self {
//         Self {}
//     }
// }

// impl Read for Usb {
//     fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
//         let mut byte_buf = [0u8; 1];
//         let byte_buf_ptr = byte_buf.as_mut_ptr() as *mut libc::c_void;
//         let mut len = 0;
//         while len < buf.len() {
//             let len_up_to_1 =
//                 unsafe { libc::read(libc::STDIN_FILENO, byte_buf_ptr, byte_buf.len()) };
//             if len_up_to_1 == 1 {
//                 buf[len] = byte_buf[0];
//                 len += 1;
//             } else {
//                 break;
//             }
//         }
//         Ok(len)
//     }
// }

// impl Write for Usb {
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         let buf_ptr = buf.as_ptr() as *mut libc::c_void;
//         let bytes_written = unsafe { libc::write(libc::STDOUT_FILENO, buf_ptr, buf.len()) };
//         Ok(bytes_written as usize)
//     }

//     fn flush(&mut self) -> std::io::Result<()> {
//         // Do nothing
//         Ok(())
//     }
// }

pub fn get_stdin_stream(
    poll_frequency: Duration,
) -> (
    std::pin::Pin<Box<dyn Stream<Item = u8> + std::marker::Send>>,
    oneshot::Sender<()>,
) {
    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
    let (mut tx, rx) = channel::<u8>(1);
    let _handle = spawn(move || {
        block_on(async {
            loop {
                let byte = unsafe { libc::getchar() };
                if byte != -1 {
                    tx.send(byte as u8).await.unwrap();
                }
                if stop_rx.try_recv().is_ok_and(|v| v.is_some()) {
                    break;
                };
                sleep(poll_frequency);
            }
        })
    });

    let stream = unfold(rx, move |mut rx| async move {
        let chunk = rx.next().await.unwrap();
        Some((chunk, rx))
    })
    .boxed();
    (stream, stop_tx)
}
