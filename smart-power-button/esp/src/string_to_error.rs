use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
/// A string error, which can optionally wrap any other error
pub struct StringError {
    source: Option<anyhow::Error>,
    string: String
}

impl Error for StringError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|cause| cause.as_ref())
    }
}

impl Display for StringError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string)
    }
}

impl From<String> for StringError {
    fn from(value: String) -> Self {
        Self {
            string: value,
            source: None
        }
    }
}

pub trait ErrorToStringError {
    fn wrap_message(self, message: String) -> StringError;
}

impl<E: Into<anyhow::Error>> ErrorToStringError for E {
    fn wrap_message(self, message: String) -> StringError {
        StringError {
            source: Some(self.into()),
            string: message,
        }
    }
}

pub trait ResultWrapErrorMessageExt<T> {
    fn wrap_err_message(self, message: String) -> Result<T, StringError>;
}

impl<T, E: Into<anyhow::Error>> ResultWrapErrorMessageExt<T> for Result<T, E> {
    fn wrap_err_message(self, message: String) -> Result<T, StringError> {
        self.map_err(|e| e.wrap_message(message))
    }
}

