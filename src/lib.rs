#![no_std]
//! # Introduction
//! This crate provides convenience definitions for working with
//! [`shared-bus`](../shared_bus/index.html).
//!
//! This repository aids in using `shared-bus`, which is a tool to share a single peripheral bus
//! such as I2C or SPI, with multiple drivers.
//!
//! Generally, `shared-bus` creates a `BusManager` which hands out `BusProxy` structures to drivers.
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
//! use shared_bus_rtic::BusProxy;
//!
//! struct SharedBusResources<T> {
//!     device: Device<BusProxy<T>>,
//!     other_device: OtherDevice<BusProxy<T>>,
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

pub use shared_bus;

#[doc(hidden)]
// This is a dummy "mutex" implementation for [`shared_bus`. Note that it is not a mutex in any way,
// but is rather just a dummy API to provide to `shared_bus`. We do not require a mutex because RTIC
// will handle resource sharing to ensure we do not have conflicts.
pub struct DummyMutex<T> {
    item: T,
    busy: AtomicBool,
}

impl<T> shared_bus::mutex::BusMutex<T> for DummyMutex<T> {
    fn create(item: T) -> Self {
        DummyMutex {
            item: item,
            busy: AtomicBool::from(false),
        }
    }

    fn lock<R, F: FnOnce(&T) -> R>(&self, f: F) -> R {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .expect("Bus conflict");
        let result = f(&self.item);

        self.busy.store(false, Ordering::SeqCst);

        result
    }
}

#[doc(hidden)]
pub type Mutex<T> = DummyMutex<core::cell::RefCell<T>>;

/// A convenience type definition for a `shared-bus` BusProxy.
///
/// The generic parameter of this type is the type of the underlying bus that is shared.
pub type BusProxy<T> = shared_bus::proxy::BusProxy<'static, Mutex<T>, T>;

/// Provides a method of generating a `shared-bus`
/// [`BusManager`](../shared_bus/proxy/struct.BusManager.html) for use in RTIC.
///
/// ## Args:
/// * `bus` - The actual bus that should be shared
/// * `T` - The full type of the bus that is being shared.
///
/// ## Example:
/// ```rust
/// let bus: I2C = ();
/// let manager = rtic_shared_bus::new!(bus, I2C);
///
/// let device = Device::new(manager.acquire());
/// ```
#[macro_export]
macro_rules! new {
    ($bus:ident, $T:ty) => {
        unsafe {
            use shared_bus_rtic::{
                shared_bus::BusManager,
                Mutex,
            };
            static mut _MANAGER: core::mem::MaybeUninit<BusManager<Mutex<$T>, $T>> =
                core::mem::MaybeUninit::uninit();
            _MANAGER = core::mem::MaybeUninit::new(BusManager::new($bus));
            &*_MANAGER.as_ptr()
        };
    };
}
