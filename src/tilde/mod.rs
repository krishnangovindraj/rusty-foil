use typedb_driver::Promise;
mod classification;
pub mod tilde;
mod tree;

pub type TildeResult<T> = std::result::Result<T, typedb_driver::Error>;
