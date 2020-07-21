# shared-bus-rtic
Provides macros and type definitions for using a shared peripheral bus in an RTIC application

## Description

Note that all of the drivers that use the same underlying bus **must** be stored within a single
resource (e.g. as one larger `struct`) within the RTIC resources. This ensures that RTIC will
prevent one driver from interrupting another while they are using the same underlying bus.

This crate also provides convenience types for working with `shared-bus` RTIC resources.

## Features

This crate is compatible with thumbv6 architectures. To enable support for thumbv6
devices, enable the `thumbv6` feature in your `Cargo.toml`:
```
[dependencies.shared-bus-rtic]
features = ["thumbv6"]
```

## Usage Example
```rust

use shared_bus_rtic::SharedBus;

struct SharedBusResources<T> {
    device: Device<SharedBus<T>>,
    other_device: OtherDevice<SharedBus<T>>,
}

// ...

// Replace this type with the type of your bus (e.g. hal::i2c::I2c<...>).
type BusType = ();

struct Resources {
    shared_bus_resources: SharedBusResources<BusType>,
}

#[init] fn init(c: init::Context) -> init::LateResources {
    let manager = shared_bus_rtic::new!(bus, BusType);
    let device = Device::new(manager.acquire());
    let other_device = OtherDevice::new(manager.acquire());

    init::LateResources {
        shared_bus_resources: SharedBusResources { device, other_device },
    }
}
```

### Valid Example

```rust
struct SharedBusResources<Bus> {
    device_on_shared_bus: Device<Bus>,
    other_device_on_shared_bus: OtherDevice<Bus>,
}

// ...

struct Resources {
    shared_bus_resources: SharedBusResources<Bus>,
}

#[task(resources=[shared_bus_resources], priority=5)
pub fn high_priority_task(c: high_priority_task::Context) {
    // Good - This task cannot interrupt the lower priority task that is using the bus because of a
    // resource lock.
    c.resources.shared_bus_resources.device_on_shared_bus.read();
}

#[task(resources=[shared_bus_resources], priority=0)
pub fn low_priority_task(c: low_priority_task::Context) {
    // Good - RTIC properly locks the entire shared bus from concurrent access.
    c.resources.shared_bus_resources.lock(|bus| bus.other_device_on_shared_bus.read());
}
```

In the above example, it can be seen that both devices on the bus are stored as a single resource
(in a shared `struct`). Because of this, RTIC properly locks the resource when either the high or
low priority task is using the bus.

### BAD EXAMPLE

```rust
struct Resources {
    // INVALID - DO NOT DO THIS.
    device_on_shared_bus: Device<Bus>,
    other_device_on_shared_bus: OtherDevice<Bus>,
}

#[task(resources=[device_on_shared_bus], priority=5)
pub fn high_priority_task(c: high_priority_task::Context) {
    // ERROR: This task might interrupt the read on the other device!!!
    c.resources.device_on_shared_bus.read();
}

#[task(resources=[other_device_on_shared_bus], priority=0)
pub fn low_priority_task(c: low_priority_task::Context) {
    // Attempt to read data from the device.
    c.resources.other_device_on_shared_bus.read();
}
```

In the above incorrect example, RTIC may interrupt the low priority task to complete the high
priority task. However, the low priority task may be using the shared bus. In this case, the
communication may be corrupted by multiple devices using the bus at the same time.
