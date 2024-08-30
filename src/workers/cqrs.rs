



use std::{error::Error, fmt::Display};
use thiserror::Error;
pub mod mutators;
pub mod accessors;

// the Error, Debug, From and Display traits must be implemented for the CqrsError and its subtypes

#[derive(Error, Debug)]
pub struct CqrsError{
    kind: CqrsErrorKind,
    time: i64
}

#[derive(Error, Debug)]
pub enum CqrsErrorKind{
    Mutator(String),
    Accessor(String)
}

impl Display for CqrsError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        if let Some(source) = self.source(){ // source is of type trait which is used for dynamic dispatching
            writeln!(f, "caused by the source: {:?}", source)?;
        }
        Ok(())
    }
}

impl Display for CqrsErrorKind{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        if let Some(source) = self.source(){ // source is of type trait which is used for dynamic dispatching
            writeln!(f, "caused by the source: {:?}", source)?;
        }
        Ok(())
    }
}