use typedb_driver::Promise;
pub mod tilde;
mod classification;
mod tree;

pub type TildeResult<T> = std::result::Result<T, typedb_driver::Error>;
