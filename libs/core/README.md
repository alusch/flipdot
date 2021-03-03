# flipdot-core

Core types for describing communication with Luminator flip-dot and LED signs.

For the basic task of sign communication, you likely want to use the high-level API
in the [`flipdot`] crate instead.

However, `flipdot_core` is useful for crates that want to interact with the sign protocol
at a lower level than the `flipdot` crate, or who want to provide their own `SignBus`
implementations for use by `flipdot`.

Tested with a MAX3000 90 × 7 side sign. Should work with any flip-dot or LED sign that uses the 7-pin circular
connector, but no guarantees.

Intended only for hobbyist and educational purposes. Not affiliated with Luminator in any way.

## Usage

Here's an example of directly interacting with a `SignBus` at the `Message` level instead of using `Sign`:

```rust
use flipdot_core::{Address, Message, Operation, SignBus, SignType, State};

// Assume we have a helper function to obtain a SignBus.
let mut bus: Box<dyn SignBus> = get_bus();

// Discover the sign and verify that is has not yet been configured.
let message = Message::Hello(Address(3));
let response = bus.process_message(message)?;
assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);

// Request that the sign receive the configuration data and verify that it acknowledges.
let message = Message::RequestOperation(Address(3), Operation::ReceiveConfig);
let response = bus.process_message(message)?;
assert_eq!(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig)), response);
```

## License

Distributed under the [MIT license].

[`flipdot`]: /
[MIT license]: /LICENSE
