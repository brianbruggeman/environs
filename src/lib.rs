mod builder;
mod dotenv;
mod error;
mod macros;
mod parse;
mod resolve;

pub use crate::builder::{Var, VarOr, VarOrElse, VarOrStr};
pub use crate::dotenv::{DotenvLoader, load, load_override, load_override_path, load_path};
pub use crate::error::{Error, Location, Result};
#[cfg(feature = "chrono")]
pub use crate::parse::ChronoParseError;
pub use crate::parse::{BoolParseError, FromEnvStr, VecParseError};
pub use crate::resolve::{resolve, resolve_or, resolve_or_else, resolve_or_parse, resolve_with};
