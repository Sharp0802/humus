use std::fmt::{Debug, Display, Formatter};

pub(crate) struct Error {
    message: String
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn new(message: &str) -> Self {
        Self{ message: message.to_string() }
    }

    pub fn from<T: ToString>(value: T) -> Self {
        Self{ message: value.to_string() }
    }
}
