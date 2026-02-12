use std::fmt;

use crate::ast::Program;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    String,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Pure,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TypeError {}

pub fn check_program(_program: &Program) -> Result<(), TypeError> {
    Ok(())
}
