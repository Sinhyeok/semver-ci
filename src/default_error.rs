use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub(crate) struct DefaultError {
    pub(crate) message: String,
    pub(crate) source: Option<Box<dyn Error>>,
}

impl fmt::Display for DefaultError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for DefaultError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }
}
