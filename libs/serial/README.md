# flipdot-serial

Tools for communicating with Luminator signs over serial.

For the basic task of sign communication, you likely want to use the high-level API
in the [`flipdot`] crate instead.

However, you can use the `configure_port` function to configure serial port appropriately
if you're doing custom lower-level communication.

Intended only for hobbyist and educational purposes. Not affiliated with Luminator in any way.

## Usage

```rust
use std::time::Duration;

let mut port = serial::open("COM3")?;
flipdot_serial::configure_port(&mut port, Duration::from_secs(5))?;
// Now ready for communication with a sign (8N1 19200 baud).
```

## License

Distributed under the [MIT license].

[`flipdot`]: /
[MIT license]: /LICENSE
