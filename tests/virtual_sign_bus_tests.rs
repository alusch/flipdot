use std::cell::RefCell;
use std::rc::Rc;

use flipdot::core::State;
use flipdot::{Address, PageFlipStyle, PageId, Sign, SignType};
use flipdot_testing::{VirtualSign, VirtualSignBus};

#[test]
fn sign_virtual_sign_interaction() {
    let bus = VirtualSignBus::new(vec![
        VirtualSign::new(Address(3), PageFlipStyle::Automatic),
        VirtualSign::new(Address(6), PageFlipStyle::Manual),
    ]);
    let bus = Rc::new(RefCell::new(bus));

    let sign = Sign::new(bus.clone(), Address(6), SignType::HorizonFront160x16);

    // Verify both virtual signs initially have unknown type and are in the Unconfigured state.
    assert_eq!(None, bus.borrow().sign(0).sign_type());
    assert_eq!(State::Unconfigured, bus.borrow().sign(0).state());
    assert_eq!(None, bus.borrow().sign(1).sign_type());
    assert_eq!(State::Unconfigured, bus.borrow().sign(1).state());

    sign.configure().unwrap();

    // After configuring sign 6, the corresponding virtual sign should be in the ConfigReceived
    // state with the appropriate sign type, and the other should be unaffected.
    assert_eq!(None, bus.borrow().sign(0).sign_type());
    assert_eq!(State::Unconfigured, bus.borrow().sign(0).state());
    assert_eq!(Some(SignType::HorizonFront160x16), bus.borrow().sign(1).sign_type());
    assert_eq!(State::ConfigReceived, bus.borrow().sign(1).state());

    let mut pages = [sign.create_page(PageId(1))];
    pages[0].set_pixel(5, 5, true);
    sign.send_pages(&pages).unwrap();

    // After sending pages to sign 6, the corresponding virtual sign should be in the PageLoaded
    // state with the appropriate page list, and the other should be unaffected.
    assert!(bus.borrow().sign(0).pages().is_empty());
    assert_eq!(State::Unconfigured, bus.borrow().sign(0).state());
    assert_eq!(&pages, bus.borrow().sign(1).pages());
    assert_eq!(State::PageLoaded, bus.borrow().sign(1).state());

    sign.show_loaded_page().unwrap();

    // After showing a page, the corresponding virtual sign should be in the PageShown
    // state, and the other should be unaffected.
    assert_eq!(State::Unconfigured, bus.borrow().sign(0).state());
    assert_eq!(State::PageShown, bus.borrow().sign(1).state());

    sign.load_next_page().unwrap();

    // After loading a page, the corresponding virtual sign should be in the PageLoaded
    // state, and the other should be unaffected.
    assert_eq!(State::Unconfigured, bus.borrow().sign(0).state());
    assert_eq!(State::PageLoaded, bus.borrow().sign(1).state());

    let mut pages2 = [sign.create_page(PageId(1)), sign.create_page(PageId(2))];
    pages2[0].set_pixel(1, 1, true);
    sign.send_pages(&pages2).unwrap();

    // After sending pages to sign 6, the corresponding virtual sign should be in the PageLoaded
    // state with the appropriate page list, and the other should be unaffected.
    assert!(bus.borrow().sign(0).pages().is_empty());
    assert_eq!(State::Unconfigured, bus.borrow().sign(0).state());
    assert_eq!(&pages2, bus.borrow().sign(1).pages());
    assert_eq!(State::PageLoaded, bus.borrow().sign(1).state());

    let sign2 = Sign::new(bus.clone(), Address(3), SignType::Max3000Dash30x7);
    sign2.configure().unwrap();

    // Configuring sign 3 should not have affected sign 6.
    assert_eq!(State::ConfigReceived, bus.borrow().sign(0).state());
    assert_eq!(State::PageLoaded, bus.borrow().sign(1).state());

    let mut pages3 = [sign2.create_page(PageId(1))];
    pages3[0].set_pixel(2, 3, true);
    sign2.send_pages(&pages3).unwrap();

    // After sending pages to sign 3, the corresponding virtual sign should be in the PageLoaded
    // state with the appropriate page list, and the other should be unaffected.
    assert_eq!(&pages3, bus.borrow().sign(0).pages());
    assert_eq!(State::ShowingPages, bus.borrow().sign(0).state());
    assert_eq!(&pages2, bus.borrow().sign(1).pages());
    assert_eq!(State::PageLoaded, bus.borrow().sign(1).state());

    sign2.show_loaded_page().unwrap();

    // These functions should be no-ops for automatic-flip signs.
    assert_eq!(State::ShowingPages, bus.borrow().sign(0).state());
    assert_eq!(State::PageLoaded, bus.borrow().sign(1).state());

    sign2.load_next_page().unwrap();

    // These functions should be no-ops for automatic flip signs.
    assert_eq!(State::ShowingPages, bus.borrow().sign(0).state());
    assert_eq!(State::PageLoaded, bus.borrow().sign(1).state());

    sign.shut_down().unwrap();

    // After shutdown, all state should be cleared from the virtual sign.
    assert!(bus.borrow().sign(1).pages().is_empty());
    assert_eq!(State::Unconfigured, bus.borrow().sign(1).state());
    assert_eq!(None, bus.borrow().sign(1).sign_type());
}
