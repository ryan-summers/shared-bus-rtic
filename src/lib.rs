#![no_std]

use core::sync::atomic::{AtomicBool, Ordering};
pub use shared_bus::BusManager;

// This is a dummy "mutex" implementation for `shared_bus`. Note that it is not a mutex in any way,
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

pub type Mutex<T> = DummyMutex<core::cell::RefCell<T>>;
pub type BusProxy<T> = shared_bus::proxy::BusProxy<'static, Mutex<T>, T>;

#[macro_export]
macro_rules! new {
    ($bus:ident, $T:ty) => {
        unsafe {
            use shared_bus_rtic::{BusManager, Mutex};
            static mut _MANAGER: core::mem::MaybeUninit<BusManager<Mutex<$T>, $T>> =
                core::mem::MaybeUninit::uninit();
            _MANAGER = core::mem::MaybeUninit::new(BusManager::new($bus));
            &*_MANAGER.as_ptr()
        };
    };
}
