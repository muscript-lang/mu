use crate::ast::Program;

pub const MAGIC: &[u8; 4] = b"MUB0";

pub fn compile(_program: &Program) -> Vec<u8> {
    MAGIC.to_vec()
}
