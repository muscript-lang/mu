use std::fmt;

use crate::bytecode::{MAGIC, OpCode};

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Int(i64),
    Bool(bool),
    String(String),
    Adt { tag: String, fields: Vec<Value> },
    Unit,
}

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

pub fn run_bytecode(bytecode: &[u8], _args: &[String]) -> Result<(), VmError> {
    let mut cursor = 0usize;
    if bytecode.len() < 4 || &bytecode[0..4] != MAGIC {
        return Err(VmError {
            message: "invalid bytecode header".to_string(),
        });
    }
    cursor += 4;
    let nstrings = read_u32(bytecode, &mut cursor)? as usize;
    let mut strings = Vec::with_capacity(nstrings);
    for _ in 0..nstrings {
        let len = read_u32(bytecode, &mut cursor)? as usize;
        if cursor + len > bytecode.len() {
            return Err(VmError {
                message: "corrupt bytecode string table".to_string(),
            });
        }
        let s = std::str::from_utf8(&bytecode[cursor..cursor + len]).map_err(|_| VmError {
            message: "bytecode string table contains invalid utf-8".to_string(),
        })?;
        strings.push(s.to_string());
        cursor += len;
    }
    let code_len = read_u32(bytecode, &mut cursor)? as usize;
    if cursor + code_len != bytecode.len() {
        return Err(VmError {
            message: "corrupt bytecode code section length".to_string(),
        });
    }
    let code = &bytecode[cursor..];

    let mut ip = 0usize;
    let mut stack: Vec<Value> = Vec::new();
    let mut locals: Vec<Value> = Vec::new();

    while ip < code.len() {
        let op = code[ip];
        ip += 1;
        match op {
            x if x == OpCode::PushInt as u8 => {
                let v = read_i64(code, &mut ip)?;
                stack.push(Value::Int(v));
            }
            x if x == OpCode::PushBool as u8 => {
                let b = read_u8(code, &mut ip)? != 0;
                stack.push(Value::Bool(b));
            }
            x if x == OpCode::PushString as u8 => {
                let idx = read_u32(code, &mut ip)? as usize;
                let s = strings.get(idx).ok_or_else(|| VmError {
                    message: "string index out of bounds".to_string(),
                })?;
                stack.push(Value::String(s.clone()));
            }
            x if x == OpCode::PushUnit as u8 => stack.push(Value::Unit),
            x if x == OpCode::LoadLocal as u8 => {
                let idx = read_u32(code, &mut ip)? as usize;
                let v = locals.get(idx).ok_or_else(|| VmError {
                    message: "local index out of bounds".to_string(),
                })?;
                stack.push(v.clone());
            }
            x if x == OpCode::StoreLocal as u8 => {
                let idx = read_u32(code, &mut ip)? as usize;
                let v = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in STORE_LOCAL".to_string(),
                })?;
                if locals.len() <= idx {
                    locals.resize(idx + 1, Value::Unit);
                }
                locals[idx] = v;
            }
            x if x == OpCode::Pop as u8 => {
                stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in POP".to_string(),
                })?;
            }
            x if x == OpCode::Jump as u8 => {
                let target = read_u32(code, &mut ip)? as usize;
                if target > code.len() {
                    return Err(VmError {
                        message: "jump target out of bounds".to_string(),
                    });
                }
                ip = target;
            }
            x if x == OpCode::JumpIfFalse as u8 => {
                let target = read_u32(code, &mut ip)? as usize;
                let cond = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in JMP_IF_FALSE".to_string(),
                })?;
                let is_false = match cond {
                    Value::Bool(v) => !v,
                    _ => {
                        return Err(VmError {
                            message: "JMP_IF_FALSE expects a bool on the stack".to_string(),
                        });
                    }
                };
                if is_false {
                    if target > code.len() {
                        return Err(VmError {
                            message: "jump target out of bounds".to_string(),
                        });
                    }
                    ip = target;
                }
            }
            x if x == OpCode::CallBuiltin as u8 => {
                let id = read_u8(code, &mut ip)?;
                let argc = read_u8(code, &mut ip)? as usize;
                if stack.len() < argc {
                    return Err(VmError {
                        message: "stack underflow in CALL_BUILTIN".to_string(),
                    });
                }
                let args = stack.split_off(stack.len() - argc);
                let result = call_builtin(id, &args)?;
                stack.push(result);
            }
            x if x == OpCode::MkAdt as u8 => {
                let tag_idx = read_u32(code, &mut ip)? as usize;
                let argc = read_u8(code, &mut ip)? as usize;
                if stack.len() < argc {
                    return Err(VmError {
                        message: "stack underflow in MK_ADT".to_string(),
                    });
                }
                let tag = strings.get(tag_idx).ok_or_else(|| VmError {
                    message: "adt tag index out of bounds".to_string(),
                })?;
                let fields = stack.split_off(stack.len() - argc);
                stack.push(Value::Adt {
                    tag: tag.clone(),
                    fields,
                });
            }
            x if x == OpCode::JumpIfTag as u8 => {
                let tag_idx = read_u32(code, &mut ip)? as usize;
                let target = read_u32(code, &mut ip)? as usize;
                let tag = strings.get(tag_idx).ok_or_else(|| VmError {
                    message: "adt tag index out of bounds".to_string(),
                })?;
                let value = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in JMP_IF_TAG".to_string(),
                })?;
                let matches = match value {
                    Value::Adt { tag: value_tag, .. } => value_tag == *tag,
                    _ => false,
                };
                if matches {
                    if target > code.len() {
                        return Err(VmError {
                            message: "jump target out of bounds".to_string(),
                        });
                    }
                    ip = target;
                }
            }
            x if x == OpCode::Return as u8 => {
                let ret = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in RET".to_string(),
                })?;
                let code = match ret {
                    Value::Int(v) => v as i32,
                    _ => {
                        return Err(VmError {
                            message: "main must return an integer exit code".to_string(),
                        });
                    }
                };
                if code != 0 {
                    return Err(VmError {
                        message: format!("program exited with status {code}"),
                    });
                }
                return Ok(());
            }
            _ => {
                return Err(VmError {
                    message: format!("unknown opcode {op}"),
                });
            }
        }
    }

    Err(VmError {
        message: "program terminated without RET".to_string(),
    })
}

fn call_builtin(id: u8, args: &[Value]) -> Result<Value, VmError> {
    match id {
        1 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "print expects one argument".to_string(),
                });
            }
            let Value::String(s) = &args[0] else {
                return Err(VmError {
                    message: "print expects a string".to_string(),
                });
            };
            print!("{s}");
            Ok(Value::Unit)
        }
        2 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "println expects one argument".to_string(),
                });
            }
            let Value::String(s) = &args[0] else {
                return Err(VmError {
                    message: "println expects a string".to_string(),
                });
            };
            println!("{s}");
            Ok(Value::Unit)
        }
        _ => Err(VmError {
            message: format!("unknown builtin id {id}"),
        }),
    }
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, VmError> {
    if *cursor >= bytes.len() {
        return Err(VmError {
            message: "truncated bytecode".to_string(),
        });
    }
    let v = bytes[*cursor];
    *cursor += 1;
    Ok(v)
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, VmError> {
    if *cursor + 4 > bytes.len() {
        return Err(VmError {
            message: "truncated bytecode".to_string(),
        });
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[*cursor..*cursor + 4]);
    *cursor += 4;
    Ok(u32::from_le_bytes(buf))
}

fn read_i64(bytes: &[u8], cursor: &mut usize) -> Result<i64, VmError> {
    if *cursor + 8 > bytes.len() {
        return Err(VmError {
            message: "truncated bytecode".to_string(),
        });
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[*cursor..*cursor + 8]);
    *cursor += 8;
    Ok(i64::from_le_bytes(buf))
}
