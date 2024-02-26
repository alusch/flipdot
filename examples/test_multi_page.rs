use std::cell::RefCell;
use std::env;
use std::error::Error;
use std::rc::Rc;

use flipdot::{Address, PageFlipStyle, PageId, SerialSignBus, Sign, SignType};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: test_multi_page <serial_port> <sign_address>");
        println!();
        println!("serial_port should be a port name like /dev/ttyUSB0 or COM3");
        println!("sign_address is the decimal address of a MAX3000 90 x 7 sign to communicate with");
        return Ok(());
    }

    let port = serial::open(&args[1])?;
    let bus = SerialSignBus::try_new(port)?;

    let addr = args[2].parse::<u16>()?;
    let sign = Sign::new(Rc::new(RefCell::new(bus)), Address(addr), SignType::Max3000Side90x7);
    sign.configure()?;

    // Create some pages and fill them with stripe patterns.
    let mut page1 = sign.create_page(PageId(0));
    for x in 0..page1.width() {
        for y in 0..page1.height() {
            page1.set_pixel(x, y, x % 4 == y % 4);
        }
    }

    let mut page2 = sign.create_page(PageId(1));
    for x in 0..page2.width() {
        for y in 0..page2.height() {
            page2.set_pixel(x, y, (x + y) % 5 > 2);
        }
    }

    // Send the pages to the sign.
    if sign.send_pages(&[page1, page2])? == PageFlipStyle::Manual {
        // Show the first page, then load and show the second.
        println!("Manually flipping pages");
        sign.show_loaded_page()?;
        sign.load_next_page()?;
        sign.show_loaded_page()?;
    } else {
        println!("Sign should automatically flip pages");
    }

    Ok(())
}
