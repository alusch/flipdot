use flipdot_core;

error_chain! {
    links {
        Core(flipdot_core::Error, flipdot_core::ErrorKind) #[doc = "Error propagated from `flipdot_core`."];
    }

    errors {
        /// An error occurred trying to configure the serial port.
        Serial(message: String) {
            description("Sign communication failure")
            display("I/O error: {}", message)
        }
    }
}

impl From<Error> for Box<::std::error::Error + Send> {
    fn from(e: Error) -> Self {
        Box::new(e)
    }
}
