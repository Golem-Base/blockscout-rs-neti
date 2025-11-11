#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    AlloyRLP(alloy_rlp::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IOError(e) => write!(f, "IOError({e})"),
            Self::AlloyRLP(e) => write!(f, "AlloyRLP({e})"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(v: std::io::Error) -> Self {
        Self::IOError(v)
    }
}

impl From<alloy_rlp::Error> for Error {
    fn from(v: alloy_rlp::Error) -> Self {
        Self::AlloyRLP(v)
    }
}
