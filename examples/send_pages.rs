//! Test program to demonstrate how to send multiple pages of data to a sign and flip between them.
//! Run with `RUST_LOG=debug` environment variable to watch the bus messages go by.

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use flipdot::{Address, PageFlipStyle, PageId, Sign, SignType};
use flipdot_testing::{VirtualSign, VirtualSignBus};

fn main() -> Result<(), Box<dyn Error>> {
    // Create a virtual sign bus for testing purposes.
    // To control a real sign you would use SerialSignBus instead.
    let virtual_signs = vec![VirtualSign::new(Address(3), PageFlipStyle::Manual)];
    let bus = Rc::new(RefCell::new(VirtualSignBus::new(virtual_signs)));

    // Create a sign for the type and address we want to control.
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
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

    // For testing purposes, print the virtual sign's configuration and pages.
    println!("Sign configured as {:?}", bus.borrow().sign(0).sign_type());
    for page in bus.borrow().sign(0).pages() {
        println!("Page {}:\n{}", page.id(), page);
    }

    Ok(())
}
