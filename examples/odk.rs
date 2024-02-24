use std::{env, error::Error};

use flipdot_core::PageFlipStyle;
use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: odk <serial_port> <flip_mode> [sign_address]");
        println!();
        println!("serial_port should be a port name like /dev/ttyUSB0 or COM3");
        println!("flip_mode should be either auto or manual");
        println!("If sign_address is omitted, all possible sign addresses will be used");
        return Ok(());
    }

    let port = serial::open(&args[1])?;
    let flip_style = if args[2].eq_ignore_ascii_case("auto") {
        PageFlipStyle::Automatic
    } else {
        PageFlipStyle::Manual
    };

    let bus: VirtualSignBus<'_>;
    if args.len() > 3 {
        let addr = args[3].parse::<u16>()?;
        println!("Providing virtual sign {}", addr);
        bus = VirtualSignBus::new(vec![VirtualSign::new(Address(addr), flip_style)]);
    } else {
        // Populate bus with signs from addresses 2 to 126
        // (which seems to be the possible range for actual signs).
        println!("Providing all virtual signs 2-126");
        let signs = (2..127).map(Address).map(|addr| { VirtualSign::new(addr, PageFlipStyle::Manual) });
        bus = VirtualSignBus::new(signs);
    }

    // Hook up ODK to virtual bus.
    let mut odk = Odk::try_new(port, bus)?;
    loop {
        // ODK communications are forwarded to/from the virtual bus.
        odk.process_message()?;
    }
}
