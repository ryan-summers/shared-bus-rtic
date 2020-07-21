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
use embedded_hal::{
    blocking::{self, i2c},
    spi,
};

/// A convenience type to use for declaring the underlying bus type.
pub type SharedBus<T> = &'static CommonBus<T>;

pub struct CommonBus<BUS> {
    bus: core::cell::UnsafeCell<BUS>,
    busy: AtomicBool,
}

impl<BUS> CommonBus<BUS> {
    pub fn new(bus: BUS) -> Self {
        CommonBus {
            bus: core::cell::UnsafeCell::new(bus),
            busy: AtomicBool::from(false),
        }
    }

    fn lock<R, F: FnOnce(&mut BUS) -> R>(&self, f: F) -> R {
        atomic::compare_exchange(&self.busy, false, true, Ordering::SeqCst, Ordering::SeqCst)
            .expect("Bus conflict");
        let result = f(unsafe { &mut *self.bus.get() });

        self.busy.store(false, Ordering::SeqCst);

        result
    }

    pub fn acquire(&self) -> &Self {
        self
    }
}

unsafe impl<BUS> Sync for CommonBus<BUS> {}

impl<BUS: i2c::Read> i2c::Read for &CommonBus<BUS> {
    type Error = BUS::Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.lock(|bus| bus.read(address, buffer))
    }
}

impl<BUS: i2c::Write> i2c::Write for &CommonBus<BUS> {
    type Error = BUS::Error;

    fn write(&mut self, address: u8, buffer: &[u8]) -> Result<(), Self::Error> {
        self.lock(|bus| bus.write(address, buffer))
    }
}

impl<BUS: i2c::WriteRead> i2c::WriteRead for &CommonBus<BUS> {
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

macro_rules! spi {
    ($($T:ty),*) => {
        $(
        impl<BUS: blocking::spi::Write<$T>> blocking::spi::Write<$T> for &CommonBus<BUS> {
            type Error = BUS::Error;

            fn write(&mut self, words: &[$T]) -> Result<(), Self::Error> {
                self.lock(|bus| bus.write(words))
            }
        }

        impl<BUS: blocking::spi::Transfer<$T>> blocking::spi::Transfer<$T> for &CommonBus<BUS> {
            type Error = BUS::Error;

            fn transfer<'w>(&mut self, words: &'w mut [$T]) -> Result<&'w [$T], Self::Error> {
                self.lock(move |bus| bus.transfer(words))
            }
        }

        impl<BUS: spi::FullDuplex<$T>> spi::FullDuplex<$T> for &CommonBus<BUS> {
            type Error = BUS::Error;

            fn read(&mut self) -> nb::Result<$T, Self::Error> {
                self.lock(|bus| bus.read())
            }

            fn send(&mut self, word: $T) -> nb::Result<(), Self::Error> {
                self.lock(|bus| bus.send(word))
            }
        }
        )*
    }
}

spi!(u8, u16, u32, u64);

#[cfg(feature = "thumbv6")]
mod atomic {
    use core::sync::atomic::{AtomicBool, Ordering};

    #[inline(always)]
    pub fn compare_exchange(
        atomic: &AtomicBool,
        current: bool,
        new: bool,
        _success: Ordering,
        _failure: Ordering,
    ) -> Result<bool, bool> {
        cortex_m::interrupt::free(|_cs| {
            let prev = atomic.load(Ordering::Acquire);
            if prev == current {
                atomic.store(new, Ordering::Release);
                Ok(prev)
            } else {
                Err(false)
            }
        })
    }
}

#[cfg(not(feature = "thumbv6"))]
mod atomic {
    use core::sync::atomic::{AtomicBool, Ordering};

    #[inline(always)]
    pub fn compare_exchange(
        atomic: &AtomicBool,
        current: bool,
        new: bool,
        success: Ordering,
        failure: Ordering,
    ) -> Result<bool, bool> {
        atomic.compare_exchange(current, new, success, failure)
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
            static mut _MANAGER: core::mem::MaybeUninit<shared_bus_rtic::CommonBus<$T>> =
                core::mem::MaybeUninit::uninit();
            _MANAGER = core::mem::MaybeUninit::new(shared_bus_rtic::CommonBus::new($bus));
            &*_MANAGER.as_ptr()
        };
    };
}
