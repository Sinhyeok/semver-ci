use super::messages;
use std::error::Error;
use std::fmt;

pub(crate) type Result<T> = std::result::Result<T, DefaultError>;

/// An application error with context and an intact chain of underlying causes.
#[derive(Debug)]
pub(crate) struct DefaultError {
    message: String,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl DefaultError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    pub(crate) fn with_source(mut self, source: impl Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Display the complete chain at the CLI or a deliberate recovery boundary.
    pub(crate) fn report(&self) -> String {
        let mut report = messages::error_report(self);
        let mut source = self.source();
        while let Some(cause) = source {
            report.push_str(&messages::caused_by(cause));
            source = cause.source();
        }
        report
    }
}

impl fmt::Display for DefaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for DefaultError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source.as_ref() as &(dyn Error + 'static))
    }
}

/// Add context where an external operation fails; propagate application errors with `?`.
pub(crate) trait ResultExt<T> {
    fn context(self, message: impl Into<String>) -> Result<T>;
}

impl<T, E: Error + Send + Sync + 'static> ResultExt<T> for std::result::Result<T, E> {
    fn context(self, message: impl Into<String>) -> Result<T> {
        self.map_err(|source| DefaultError::new(message).with_source(source))
    }
}
