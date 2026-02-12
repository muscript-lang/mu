use std::fmt;

#[derive(Debug)]
pub struct VmError {
    pub message: String,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VmError {}

pub fn run_bytecode(_bytecode: &[u8], _args: &[String]) -> Result<(), VmError> {
    Err(VmError {
        message: "`muc run` runtime is not implemented yet in this scaffold".to_string(),
    })
}
