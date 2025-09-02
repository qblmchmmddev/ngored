use tui_logger::TuiLoggerError;

#[allow(dead_code)]
#[derive(Debug)]
pub enum NgoredError {
    Logger(String),
    IO(String),
}

impl From<TuiLoggerError> for NgoredError {
    fn from(value: TuiLoggerError) -> Self {
        NgoredError::Logger(match value {
            TuiLoggerError::SetLoggerError(set_logger_error) => set_logger_error.to_string(),
            TuiLoggerError::ThreadError(error) => error.to_string(),
        })
    }
}

impl From<std::io::Error> for NgoredError {
    fn from(value: std::io::Error) -> Self {
        NgoredError::IO(value.to_string())
    }
}
