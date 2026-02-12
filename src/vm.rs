use std::fmt;
use std::io::Read;

use crate::bytecode::{MAGIC, OpCode};

#[derive(Debug, Clone)]
struct FunctionBlob {
    arity: u8,
    captures: u8,
    code: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Int(i64),
    Bool(bool),
    String(String),
    Adt { tag: String, fields: Vec<Value> },
    Closure { fn_id: u32, captures: Vec<Value> },
    Unit,
}

#[derive(Debug)]
struct Frame {
    fn_id: usize,
    ip: usize,
    locals: Vec<Value>,
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
    let (strings, functions, entry_fn) = decode(bytecode)?;
    let entry_idx = entry_fn as usize;
    if entry_idx >= functions.len() {
        return Err(VmError {
            message: "entry function index out of bounds".to_string(),
        });
    }
    if functions[entry_idx].arity != 0 {
        return Err(VmError {
            message: "main function must have arity 0".to_string(),
        });
    }

    let mut stack: Vec<Value> = Vec::new();
    let mut frames = vec![Frame {
        fn_id: entry_idx,
        ip: 0,
        locals: Vec::new(),
    }];

    while !frames.is_empty() {
        let frame = frames.last_mut().expect("checked non-empty");
        let func = &functions[frame.fn_id];
        let code = &func.code;
        if frame.ip >= code.len() {
            return Err(VmError {
                message: "program terminated without RET".to_string(),
            });
        }

        let op = code[frame.ip];
        frame.ip += 1;
        match op {
            x if x == OpCode::PushInt as u8 => {
                let v = read_i64(code, &mut frame.ip)?;
                stack.push(Value::Int(v));
            }
            x if x == OpCode::PushBool as u8 => {
                let b = read_u8(code, &mut frame.ip)? != 0;
                stack.push(Value::Bool(b));
            }
            x if x == OpCode::PushString as u8 => {
                let idx = read_u32(code, &mut frame.ip)? as usize;
                let s = strings.get(idx).ok_or_else(|| VmError {
                    message: "string index out of bounds".to_string(),
                })?;
                stack.push(Value::String(s.clone()));
            }
            x if x == OpCode::PushUnit as u8 => stack.push(Value::Unit),
            x if x == OpCode::LoadLocal as u8 => {
                let idx = read_u32(code, &mut frame.ip)? as usize;
                let v = frame.locals.get(idx).ok_or_else(|| VmError {
                    message: "local index out of bounds".to_string(),
                })?;
                stack.push(v.clone());
            }
            x if x == OpCode::StoreLocal as u8 => {
                let idx = read_u32(code, &mut frame.ip)? as usize;
                let v = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in STORE_LOCAL".to_string(),
                })?;
                if frame.locals.len() <= idx {
                    frame.locals.resize(idx + 1, Value::Unit);
                }
                frame.locals[idx] = v;
            }
            x if x == OpCode::Pop as u8 => {
                stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in POP".to_string(),
                })?;
            }
            x if x == OpCode::Jump as u8 => {
                let target = read_u32(code, &mut frame.ip)? as usize;
                if target > code.len() {
                    return Err(VmError {
                        message: "jump target out of bounds".to_string(),
                    });
                }
                frame.ip = target;
            }
            x if x == OpCode::JumpIfFalse as u8 => {
                let target = read_u32(code, &mut frame.ip)? as usize;
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
                    frame.ip = target;
                }
            }
            x if x == OpCode::CallBuiltin as u8 => {
                let id = read_u8(code, &mut frame.ip)?;
                let argc = read_u8(code, &mut frame.ip)? as usize;
                if stack.len() < argc {
                    return Err(VmError {
                        message: "stack underflow in CALL_BUILTIN".to_string(),
                    });
                }
                let args = stack.split_off(stack.len() - argc);
                let result = call_builtin(id, &args)?;
                stack.push(result);
            }
            x if x == OpCode::CallFn as u8 => {
                let fn_id = read_u32(code, &mut frame.ip)? as usize;
                let argc = read_u8(code, &mut frame.ip)? as usize;
                let target = functions.get(fn_id).ok_or_else(|| VmError {
                    message: "function id out of bounds".to_string(),
                })?;
                if target.arity as usize != argc {
                    return Err(VmError {
                        message: format!(
                            "function arity mismatch: expected {}, got {}",
                            target.arity, argc
                        ),
                    });
                }
                if stack.len() < argc {
                    return Err(VmError {
                        message: "stack underflow in CALL_FN".to_string(),
                    });
                }
                if target.captures != 0 {
                    return Err(VmError {
                        message: "CALL_FN cannot target closure-compiled function".to_string(),
                    });
                }
                let args = stack.split_off(stack.len() - argc);
                frames.push(Frame {
                    fn_id,
                    ip: 0,
                    locals: args,
                });
            }
            x if x == OpCode::MkClosure as u8 => {
                let fn_id = read_u32(code, &mut frame.ip)?;
                let ncap = read_u8(code, &mut frame.ip)? as usize;
                if stack.len() < ncap {
                    return Err(VmError {
                        message: "stack underflow in MK_CLOSURE".to_string(),
                    });
                }
                let captures = stack.split_off(stack.len() - ncap);
                stack.push(Value::Closure { fn_id, captures });
            }
            x if x == OpCode::CallClosure as u8 => {
                let argc = read_u8(code, &mut frame.ip)? as usize;
                if stack.len() < argc + 1 {
                    return Err(VmError {
                        message: "stack underflow in CALL_CLOSURE".to_string(),
                    });
                }
                let args = stack.split_off(stack.len() - argc);
                let closure = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in CALL_CLOSURE".to_string(),
                })?;
                let Value::Closure { fn_id, captures } = closure else {
                    return Err(VmError {
                        message: "CALL_CLOSURE expects a closure value".to_string(),
                    });
                };
                let target = functions.get(fn_id as usize).ok_or_else(|| VmError {
                    message: "closure function id out of bounds".to_string(),
                })?;
                if target.arity as usize != argc {
                    return Err(VmError {
                        message: format!(
                            "closure arity mismatch: expected {}, got {}",
                            target.arity, argc
                        ),
                    });
                }
                if target.captures as usize != captures.len() {
                    return Err(VmError {
                        message: "closure capture count mismatch".to_string(),
                    });
                }
                let mut locals = captures;
                locals.extend(args);
                frames.push(Frame {
                    fn_id: fn_id as usize,
                    ip: 0,
                    locals,
                });
            }
            x if x == OpCode::MkAdt as u8 => {
                let tag_idx = read_u32(code, &mut frame.ip)? as usize;
                let argc = read_u8(code, &mut frame.ip)? as usize;
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
                let tag_idx = read_u32(code, &mut frame.ip)? as usize;
                let target = read_u32(code, &mut frame.ip)? as usize;
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
                    frame.ip = target;
                }
            }
            x if x == OpCode::AssertConst as u8 => {
                let msg_idx = read_u32(code, &mut frame.ip)? as usize;
                let msg = strings.get(msg_idx).ok_or_else(|| VmError {
                    message: "assert message index out of bounds".to_string(),
                })?;
                let cond = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in ASSERT_CONST".to_string(),
                })?;
                let is_true = as_bool(cond)?;
                if !is_true {
                    return Err(VmError {
                        message: format!("assert failure: {msg}"),
                    });
                }
                stack.push(Value::Unit);
            }
            x if x == OpCode::AssertDyn as u8 => {
                let msg = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in ASSERT_DYN".to_string(),
                })?;
                let cond = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in ASSERT_DYN".to_string(),
                })?;
                let is_true = as_bool(cond)?;
                if !is_true {
                    let message = match msg {
                        Value::String(s) => s,
                        _ => "assert failure".to_string(),
                    };
                    return Err(VmError {
                        message: format!("assert failure: {message}"),
                    });
                }
                stack.push(Value::Unit);
            }
            x if x == OpCode::GetAdtField as u8 => {
                let idx = read_u8(code, &mut frame.ip)? as usize;
                let value = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in GET_ADT_FIELD".to_string(),
                })?;
                let Value::Adt { fields, .. } = value else {
                    return Err(VmError {
                        message: "GET_ADT_FIELD expects an ADT value".to_string(),
                    });
                };
                let field = fields.get(idx).ok_or_else(|| VmError {
                    message: "adt field index out of bounds".to_string(),
                })?;
                stack.push(field.clone());
            }
            x if x == OpCode::Return as u8 => {
                let ret = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in RET".to_string(),
                })?;
                frames.pop();
                if frames.is_empty() {
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
                stack.push(ret);
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

fn decode(bytecode: &[u8]) -> Result<(Vec<String>, Vec<FunctionBlob>, u32), VmError> {
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

    let nfuncs = read_u32(bytecode, &mut cursor)? as usize;
    let mut functions = Vec::with_capacity(nfuncs);
    for _ in 0..nfuncs {
        let arity = read_u8(bytecode, &mut cursor)?;
        let captures = read_u8(bytecode, &mut cursor)?;
        let code_len = read_u32(bytecode, &mut cursor)? as usize;
        if cursor + code_len > bytecode.len() {
            return Err(VmError {
                message: "corrupt bytecode function section".to_string(),
            });
        }
        let code = bytecode[cursor..cursor + code_len].to_vec();
        cursor += code_len;
        functions.push(FunctionBlob {
            arity,
            captures,
            code,
        });
    }

    let entry = read_u32(bytecode, &mut cursor)?;
    if cursor != bytecode.len() {
        return Err(VmError {
            message: "trailing bytes in bytecode stream".to_string(),
        });
    }
    Ok((strings, functions, entry))
}

fn as_bool(value: Value) -> Result<bool, VmError> {
    match value {
        Value::Bool(v) => Ok(v),
        _ => Err(VmError {
            message: "assert expects bool condition".to_string(),
        }),
    }
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
        3 => {
            if !args.is_empty() {
                return Err(VmError {
                    message: "readln expects zero arguments".to_string(),
                });
            }
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).map_err(|e| VmError {
                message: format!("readln failed: {e}"),
            })?;
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            Ok(Value::String(line))
        }
        4 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "read expects one argument".to_string(),
                });
            }
            let Value::String(path) = &args[0] else {
                return Err(VmError {
                    message: "read expects a string path".to_string(),
                });
            };
            match std::fs::read_to_string(path) {
                Ok(data) => Ok(ok_value(Value::String(data))),
                Err(e) => Ok(err_value(format!("read failed: {e}"))),
            }
        }
        5 => {
            if args.len() != 2 {
                return Err(VmError {
                    message: "write expects two arguments".to_string(),
                });
            }
            let Value::String(path) = &args[0] else {
                return Err(VmError {
                    message: "write expects a string path".to_string(),
                });
            };
            let Value::String(data) = &args[1] else {
                return Err(VmError {
                    message: "write expects a string payload".to_string(),
                });
            };
            match std::fs::write(path, data) {
                Ok(()) => Ok(ok_value(Value::Unit)),
                Err(e) => Ok(err_value(format!("write failed: {e}"))),
            }
        }
        6 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "parse expects one argument".to_string(),
                });
            }
            let Value::String(text) = &args[0] else {
                return Err(VmError {
                    message: "parse expects a string".to_string(),
                });
            };
            match serde_json::from_str::<serde_json::Value>(text) {
                Ok(v) => match serde_json::to_string(&v) {
                    Ok(normalized) => Ok(ok_value(Value::String(normalized))),
                    Err(e) => Ok(err_value(format!("json stringify failed: {e}"))),
                },
                Err(e) => Ok(err_value(format!("json parse failed: {e}"))),
            }
        }
        7 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "stringify expects one argument".to_string(),
                });
            }
            match &args[0] {
                Value::String(s) => Ok(Value::String(s.clone())),
                Value::Adt { tag, fields } => Ok(Value::String(format!("{tag}({})", fields.len()))),
                Value::Closure { .. } => Ok(Value::String("<closure>".to_string())),
                Value::Int(v) => Ok(Value::String(v.to_string())),
                Value::Bool(v) => Ok(Value::String(v.to_string())),
                Value::Unit => Ok(Value::String("()".to_string())),
            }
        }
        8 => {
            if args.len() != 2 {
                return Err(VmError {
                    message: "run expects two arguments".to_string(),
                });
            }
            let Value::String(cmd) = &args[0] else {
                return Err(VmError {
                    message: "run expects a string command".to_string(),
                });
            };
            let Value::String(arg_text) = &args[1] else {
                return Err(VmError {
                    message: "run expects second argument as string (space-separated args)".to_string(),
                });
            };
            let mut child = std::process::Command::new(cmd);
            for arg in arg_text.split_whitespace() {
                child.arg(arg);
            }
            match child.status() {
                Ok(status) => Ok(ok_value(Value::Int(i64::from(status.code().unwrap_or(-1))))),
                Err(e) => Ok(err_value(format!("run failed: {e}"))),
            }
        }
        9 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "get expects one argument".to_string(),
                });
            }
            let Value::String(url) = &args[0] else {
                return Err(VmError {
                    message: "get expects a string url".to_string(),
                });
            };
            match ureq::get(url).call() {
                Ok(mut response) => {
                    let mut body = String::new();
                    response
                        .body_mut()
                        .as_reader()
                        .read_to_string(&mut body)
                        .map_err(|e| VmError {
                            message: format!("get body read failed: {e}"),
                        })?;
                    Ok(ok_value(Value::String(body)))
                }
                Err(e) => Ok(err_value(format!("get failed: {e}"))),
            }
        }
        _ => Err(VmError {
            message: format!("unknown builtin id {id}"),
        }),
    }
}

fn ok_value(value: Value) -> Value {
    Value::Adt {
        tag: "Ok".to_string(),
        fields: vec![value],
    }
}

fn err_value(message: String) -> Value {
    Value::Adt {
        tag: "Er".to_string(),
        fields: vec![Value::String(message)],
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
