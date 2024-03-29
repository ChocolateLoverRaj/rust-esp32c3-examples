use esp_idf_sys as _;

fn main() {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    let mut buf = [0u8; 1024];
    let buf_ptr = buf.as_mut_ptr() as *mut libc::c_void;

    loop {
        let len = unsafe { libc::read(libc::STDIN_FILENO, buf_ptr, buf.len()) };
        if len > 0 {
            unsafe { libc::write(libc::STDOUT_FILENO, buf_ptr, len as usize) };
        }
    }
}
