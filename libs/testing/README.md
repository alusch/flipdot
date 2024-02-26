# flipdot-testing

Tools for testing and debugging Luminator sign communications.

For the basic task of sign communication, you likely want to use the high-level API
in the [`flipdot`] crate instead.

This crate isn't directly related to controlling a real sign, but provides some helpful diagnostic tools.
`VirtualSignBus` is a general-purpose mock implementation of one or more signs attached to the bus,
and `Odk` allows connecting a real ODK over serial to a `SignBus`.

Intended only for hobbyist and educational purposes. Not affiliated with Luminator in any way.

## Usage

```rust
use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};

// Populate bus with signs from addresses 2 to 126
// (which seems to be the possible range for actual signs).
let signs = (2..127).map(Address).map(|addr| VirtualSign::new(addr, PageFlipStyle::Manual));
let bus = VirtualSignBus::new(signs);

// Hook up ODK to virtual bus.
let port = serial::open("COM3")?;
let mut odk = Odk::try_new(port, bus)?;
loop {
    // ODK communications are forwarded to/from the virtual bus.
    odk.process_message()?;
}
```

## License

Distributed under the [MIT license].

[`flipdot`]: /
[MIT license]: /LICENSE
