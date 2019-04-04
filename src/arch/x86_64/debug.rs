use core::fmt;
#[cfg(feature = "qemu_debug")]
use spin::Mutex;
use spin::MutexGuard;

use log::{LOG, Log};
#[cfg(feature = "qemu_debug")]
use syscall::io::Io;
use syscall::io::Pio;
#[cfg(feature = "serial_debug")]
use devices::uart_16550::SerialPort;

#[cfg(feature = "graphical_debug")]
use super::graphical_debug::{DEBUG_DISPLAY, DebugDisplay};
#[cfg(feature = "serial_debug")]
use super::device::serial::COM1;

#[cfg(feature = "qemu_debug")]
pub static QEMU: Mutex<Pio<u8>> = Mutex::new(Pio::<u8>::new(0x402));

pub struct Writer;

impl Writer {
    pub fn new() -> Self {
        Writer
    }

    pub fn write(&mut self, buf: &[u8]) {
        {
            if let Some(ref mut log) = *LOG.lock() {
                log.write(buf);
            }
        }

        #[cfg(feature = "graphical_debug")]
        {
            if let Some(ref mut display) = *DEBUG_DISPLAY.lock() {
                let _ = display.write(buf);
            }
        }

        #[cfg(feature = "qemu_debug")]
        {
            let qemu = QEMU.lock();
            for &b in buf {
                qemu.write(b);
            }
        }

        #[cfg(feature = "serial_debug")]
        {
            COM1.lock().write(buf);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        self.write(s.as_bytes());
        Ok(())
    }
}
