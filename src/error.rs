#[derive(Debug, Clone, Copy)]
pub enum CreateError {
    TypeNotFound,
    InvalidArgs,
}

impl std::fmt::Display for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for CreateError {}
