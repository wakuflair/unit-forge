mod parser;
mod unit;
mod unit_table;

use thiserror::Error;
pub use unit::*;
pub use unit_table::*;

#[derive(Debug, Error)]
pub enum DefinitionError {
    #[error("Duplicated unit found. Unit '{0}' of category '{1}'")]
    DuplicatedUnit(String, String),
    #[error("Derived unit not defined. Unit '{0}' in expression '{1}' of category '{2}'")]
    UnitNotFound(String, String, String),
    #[error("Invalid derived expression format: '{0}'")]
    InvalidDerivedExpression(String),
    #[error("No units defined in category '{0}'")]
    NoUnitDefined(String),
}
