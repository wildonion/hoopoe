



use std::{error::Error, fmt::{Display, Write}};
use thiserror::Error;
pub mod mutators;
pub mod accessors;

// the Error, Debug, From and Display traits must be implemented for the CqrsError and its subtypes
// the Error trait will be implemented by the thiserror Error macro for each type annotated with this macro

#[derive(Error)]
pub struct CqrsError{
    kind: CqrsErrorKind,
    time: i64
}

#[derive(Error)]
pub enum CqrsErrorKind{
    Mutator(String), // the cause of the mutator actor error is of type string
    Accessor(String) // the cause of the accessor actor error is of type string
}


// Debug trait is used to extend and change the way of 
// debugging an instance of a type to the console
impl std::fmt::Debug for CqrsError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self)?;
        if let Some(source) = self.source(){
            writeln!(f, "Debug Error: {}", source)?;
        }
        Ok(())
    }   
}

impl std::fmt::Debug for CqrsErrorKind{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self)?;
        if let Some(source) = self.source(){
            writeln!(f, "Debug Error: {}", source)?;
        }
        Ok(())
    }   
}


// Debug trait is used to extend and change the way of 
// logging and printing an instance of a type to the console
impl Display for CqrsError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        if let Some(source) = self.source(){ // source is of type trait which is used for dynamic dispatching
            writeln!(f, "Display Error: {:?}", source)?;
        }
        Ok(())
    }
}

impl Display for CqrsErrorKind{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        if let Some(source) = self.source(){ // source is of type trait which is used for dynamic dispatching
            writeln!(f, "Display Error: {:?}", source)?;
        }
        Ok(())
    }
}