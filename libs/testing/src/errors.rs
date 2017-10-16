use flipdot_core;
use flipdot_serial;

error_chain! {
    links {
        Core(flipdot_core::Error, flipdot_core::ErrorKind) #[doc = "Error propagated from `flipdot_core`."];
        Serial(flipdot_serial::Error, flipdot_serial::ErrorKind) #[doc = "Error propagated from `flipdot_serial`."];
    }

    errors {
        /// The sign bus failed to process a message.
        Bus {
            description("Sign bus failed to process message")
            display("Sign bus failed to process message")
        }
    }
}

impl From<Error> for Box<::std::error::Error + Send> {
    fn from(e: Error) -> Self {
        Box::new(e)
    }
}
