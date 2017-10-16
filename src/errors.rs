error_chain! {
    errors {
        /// Sign bus failed to process message.
        Bus {
            description("Sign bus failed to process message")
            display("Sign bus failed to process message")
        }

        /// Sign did not respond properly according to the protocol.
        UnexpectedResponse(expected: String, got: String) {
            description("Unexpected sign response")
            display("Expected {} but received {}", expected, got)
        }
    }
}

impl From<Error> for Box<::std::error::Error + Send> {
    fn from(e: Error) -> Self {
        Box::new(e)
    }
}
