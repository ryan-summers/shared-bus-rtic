#![no_std]
//! # Introduction
//! This crate provides a means of sharing an I2C or SPI bus between multiple drivers.
//!
//! ## Notice
//! Note that all of the drivers that use the same underlying bus **must** be stored within a single
//! resource (e.g. as one larger `struct`) within the RTIC resources. This ensures that RTIC will
//! prevent one driver from interrupting another while they are using the same underlying bus.
//!
//! This crate also provides convenience types for working with `shared-bus` RTIC resources.
//!
//! ## Usage Example
//! ```rust
//!
//! use shared_bus_rtic::SharedBus;
//!
//! struct SharedBusResources<T> {
//!     device: Device<SharedBus<T>>,
//!     other_device: OtherDevice<SharedBus<T>>,
//! }
//!
//! // ...
//!
//! // Replace this type with the type of your bus (e.g. hal::i2c::I2c<...>).
//! type BusType = ();
//!
//! struct Resources {
//!     shared_bus_resources: SharedBusResources<BusType>,
//! }
//!
//! #[init]
//! fn init(c: init::Context) -> init::LateResources {
//!     // TODO: Define your custom bus here.
//!     let bus: BusType = ();
//!
//!     // Construct the bus manager.
//!     let manager = shared_bus_rtic::new!(bus, BusType);
//!
//!     // Construct all of your devices that use the shared bus.
//!     let device = Device::new(manager.acquire());
//!     let other_device = OtherDevice::new(manager.acquire());
//!
//!     init::LateResources {
//!         shared_bus_resources: SharedBusResources { device, other_device },
//!     }
//! }
//! ```

use core::sync::atomic::{AtomicBool, Ordering};
use embedded_hal::blocking::{i2c, spi};

pub struct SharedBus<BUS> {
    bus: core::cell::UnsafeCell<BUS>,
    busy: AtomicBool,
}

impl<BUS> SharedBus<BUS> {
    pub fn new(bus: BUS) -> Self {
        SharedBus {
            bus: core::cell::UnsafeCell::new(bus),
            busy: AtomicBool::from(false),
        }
    }

    fn lock<R, F: FnOnce(&mut BUS) -> R>(&self, f: F) -> R {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .expect("Bus conflict");
        let result = f(unsafe { &mut *self.bus.get() });

        self.busy.store(false, Ordering::SeqCst);

        result
    }

    pub fn acquire(&self) -> &Self {
        self
    }
}

unsafe impl<BUS> Sync for SharedBus<BUS> {}

impl<BUS: i2c::Read> i2c::Read for &SharedBus<BUS> {
    type Error = BUS::Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.lock(|bus| bus.read(address, buffer))
    }
}

impl<BUS: i2c::Write> i2c::Write for &SharedBus<BUS> {
    type Error = BUS::Error;

    fn write(&mut self, address: u8, buffer: &[u8]) -> Result<(), Self::Error> {
        self.lock(|bus| bus.write(address, buffer))
    }
}

impl<BUS: i2c::WriteRead> i2c::WriteRead for &SharedBus<BUS> {
    type Error = BUS::Error;

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.lock(|bus| bus.write_read(address, bytes, buffer))
    }
}

impl<BUS: spi::Transfer<u8>> spi::Transfer<u8> for &SharedBus<BUS> {
    type Error = BUS::Error;

    fn transfer<'w>(&mut self, words: &'w mut [u8]) -> Result<&'w [u8], Self::Error> {
        self.lock(move |bus| bus.transfer(words))
    }
}

impl<BUS: spi::Write<u8>> spi::Write<u8> for &SharedBus<BUS> {
    type Error = BUS::Error;

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.lock(|bus| bus.write(words))
    }
}

/// Provides a method of generating a shared bus.
///
/// ## Args:
/// * `bus` - The actual bus that should be shared
/// * `T` - The full type of the bus that is being shared.
///
/// ## Example:
/// ```rust
/// let bus: I2C = ();
/// let manager = shared_bus_rtic::new!(bus, I2C);
///
/// let device = Device::new(manager.acquire());
/// ```
#[macro_export]
macro_rules! new {
    ($bus:ident, $T:ty) => {
        unsafe {
            static mut _MANAGER: core::mem::MaybeUninit<shared_bus_rtic::SharedBus<$T>> =
                core::mem::MaybeUninit::uninit();
            _MANAGER = core::mem::MaybeUninit::new(shared_bus_rtic::SharedBus::new($bus));
            &*_MANAGER.as_ptr()
        };
    };
}
