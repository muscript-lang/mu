use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;

use crate::bytecode::{self, OpCode};

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
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

pub const DEFAULT_FUEL: u64 = 10_000_000;

pub trait VmHost {
    fn io_print(&mut self, text: &str) -> Result<(), VmError>;
    fn io_println(&mut self, text: &str) -> Result<(), VmError>;
    fn io_readln(&mut self) -> Result<String, VmError>;
    fn fs_read_to_string(&mut self, path: &str) -> Result<String, String>;
    fn fs_write_string(&mut self, path: &str, data: &str) -> Result<(), String>;
    fn proc_run(&mut self, cmd: &str, args: &[String]) -> Result<i32, String>;
    fn http_get(&mut self, url: &str) -> Result<String, String>;
}

#[derive(Default)]
pub struct RealHost;

pub struct FuzzHost;

impl VmHost for RealHost {
    fn io_print(&mut self, text: &str) -> Result<(), VmError> {
        print!("{text}");
        Ok(())
    }

    fn io_println(&mut self, text: &str) -> Result<(), VmError> {
        println!("{text}");
        Ok(())
    }

    fn io_readln(&mut self) -> Result<String, VmError> {
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
        Ok(line)
    }

    fn fs_read_to_string(&mut self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("read failed: {e}"))
    }

    fn fs_write_string(&mut self, path: &str, data: &str) -> Result<(), String> {
        std::fs::write(path, data).map_err(|e| format!("write failed: {e}"))
    }

    fn proc_run(&mut self, cmd: &str, args: &[String]) -> Result<i32, String> {
        let mut child = std::process::Command::new(cmd);
        for arg in args {
            child.arg(arg);
        }
        child
            .status()
            .map(|status| status.code().unwrap_or(-1))
            .map_err(|e| format!("run failed: {e}"))
    }

    fn http_get(&mut self, url: &str) -> Result<String, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            match ureq::get(url).call() {
                Ok(mut response) => {
                    let mut body = String::new();
                    response
                        .body_mut()
                        .as_reader()
                        .read_to_string(&mut body)
                        .map_err(|e| format!("get body read failed: {e}"))?;
                    Ok(body)
                }
                Err(e) => Err(format!("get failed: {e}")),
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = url;
            Err("http get disabled on wasm host".to_string())
        }
    }
}

impl VmHost for FuzzHost {
    fn io_print(&mut self, _text: &str) -> Result<(), VmError> {
        Ok(())
    }

    fn io_println(&mut self, _text: &str) -> Result<(), VmError> {
        Ok(())
    }

    fn io_readln(&mut self) -> Result<String, VmError> {
        Err(VmError {
            message: "fuzz host: readln disabled".to_string(),
        })
    }

    fn fs_read_to_string(&mut self, _path: &str) -> Result<String, String> {
        Err("fuzz host: fs read disabled".to_string())
    }

    fn fs_write_string(&mut self, _path: &str, _data: &str) -> Result<(), String> {
        Err("fuzz host: fs write disabled".to_string())
    }

    fn proc_run(&mut self, _cmd: &str, _args: &[String]) -> Result<i32, String> {
        Err("fuzz host: proc run disabled".to_string())
    }

    fn http_get(&mut self, _url: &str) -> Result<String, String> {
        Err("fuzz host: http get disabled".to_string())
    }
}

pub fn run_bytecode(bytecode: &[u8], args: &[String]) -> Result<(), VmError> {
    let mut host = RealHost;
    run_bytecode_with_fuel_and_host(bytecode, args, DEFAULT_FUEL, &mut host)
}

pub fn run_bytecode_with_fuel(bytecode: &[u8], args: &[String], fuel: u64) -> Result<(), VmError> {
    let mut host = RealHost;
    run_bytecode_with_fuel_and_host(bytecode, args, fuel, &mut host)
}

pub fn run_bytecode_with_fuel_and_host<H: VmHost>(
    bytecode: &[u8],
    _args: &[String],
    mut fuel: u64,
    host: &mut H,
) -> Result<(), VmError> {
    let decoded = bytecode::decode(bytecode).map_err(|e| VmError {
        message: e.to_string(),
    })?;
    let strings = decoded.strings;
    let functions = decoded.functions;
    let entry_fn = decoded.entry_fn;
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
        if fuel == 0 {
            return Err(VmError {
                message: with_code("E4007", "execution fuel exhausted"),
            });
        }
        fuel -= 1;
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
                let result = call_builtin(host, id, &args)?;
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
            x if x == OpCode::Trap as u8 => {
                let msg_idx = read_u32(code, &mut frame.ip)? as usize;
                let msg = strings.get(msg_idx).ok_or_else(|| VmError {
                    message: "trap message index out of bounds".to_string(),
                })?;
                return Err(VmError {
                    message: msg.clone(),
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
                        message: with_code("E4001", &format!("assert failure: {msg}")),
                    });
                }
                stack.push(Value::Unit);
            }
            x if x == OpCode::ContractConst as u8 => {
                let msg_idx = read_u32(code, &mut frame.ip)? as usize;
                let msg = strings.get(msg_idx).ok_or_else(|| VmError {
                    message: "contract message index out of bounds".to_string(),
                })?;
                let cond = stack.pop().ok_or_else(|| VmError {
                    message: "stack underflow in CONTRACT_CONST".to_string(),
                })?;
                let is_true = as_bool(cond)?;
                if !is_true {
                    return Err(VmError {
                        message: with_code("E4002", &format!("contract failure: {msg}")),
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
                        message: with_code("E4001", &format!("assert failure: {message}")),
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
                    message: with_code("E4004", "adt field index out of bounds"),
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
                            message: with_code(
                                "E4006",
                                &format!("program exited with status {code}"),
                            ),
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

fn as_bool(value: Value) -> Result<bool, VmError> {
    match value {
        Value::Bool(v) => Ok(v),
        _ => Err(VmError {
            message: "assert expects bool condition".to_string(),
        }),
    }
}

fn call_builtin<H: VmHost>(host: &mut H, id: u8, args: &[Value]) -> Result<Value, VmError> {
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
            host.io_print(s)?;
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
            host.io_println(s)?;
            Ok(Value::Unit)
        }
        3 => {
            if !args.is_empty() {
                return Err(VmError {
                    message: "readln expects zero arguments".to_string(),
                });
            }
            host.io_readln().map(Value::String)
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
            match host.fs_read_to_string(path) {
                Ok(data) => Ok(ok_value(Value::String(data))),
                Err(e) => Ok(err_value(e)),
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
            match host.fs_write_string(path, data) {
                Ok(()) => Ok(ok_value(Value::Unit)),
                Err(e) => Ok(err_value(e)),
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
                Ok(v) => Ok(ok_value(json_to_value(v))),
                Err(e) => Ok(err_value(format!("json parse failed: {e}"))),
            }
        }
        7 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "stringify expects one argument".to_string(),
                });
            }
            if let Some(v) = value_to_json(&args[0]) {
                return serde_json::to_string(&v)
                    .map(Value::String)
                    .map_err(|e| VmError {
                        message: format!("json stringify failed: {e}"),
                    });
            }
            match &args[0] {
                Value::String(s) => Ok(Value::String(s.clone())),
                Value::Adt { tag, fields } => Ok(Value::String(format!("{tag}({})", fields.len()))),
                Value::Closure { .. } => Ok(Value::String("<closure>".to_string())),
                Value::Int(v) => Ok(Value::String(v.to_string())),
                Value::Float(v) => Ok(Value::String(v.to_string())),
                Value::Bool(v) => Ok(Value::String(v.to_string())),
                Value::Array(items) => Ok(Value::String(format!("<array:{}>", items.len()))),
                Value::Map(entries) => Ok(Value::String(format!("<map:{}>", entries.len()))),
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
            let Value::Array(arg_values) = &args[1] else {
                return Err(VmError {
                    message: "run expects second argument as string array".to_string(),
                });
            };
            let mut proc_args = Vec::with_capacity(arg_values.len());
            for arg in arg_values {
                let Value::String(arg) = arg else {
                    return Err(VmError {
                        message: "run expects second argument as string array".to_string(),
                    });
                };
                proc_args.push(arg.clone());
            }
            match host.proc_run(cmd, &proc_args) {
                Ok(code) => Ok(ok_value(Value::Int(i64::from(code)))),
                Err(e) => Ok(err_value(e)),
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
            match host.http_get(url) {
                Ok(body) => Ok(ok_value(Value::String(body))),
                Err(e) => Ok(err_value(e)),
            }
        }
        20 => {
            let (a, b) = int2(args, "+")?;
            a.checked_add(b)
                .map(Value::Int)
                .ok_or_else(|| arithmetic_error("integer overflow"))
        }
        21 => {
            let (a, b) = int2(args, "-")?;
            a.checked_sub(b)
                .map(Value::Int)
                .ok_or_else(|| arithmetic_error("integer overflow"))
        }
        22 => {
            let (a, b) = int2(args, "*")?;
            a.checked_mul(b)
                .map(Value::Int)
                .ok_or_else(|| arithmetic_error("integer overflow"))
        }
        23 => {
            let (a, b) = int2(args, "/")?;
            if b == 0 {
                return Err(VmError {
                    message: with_code("E4003", "division by zero"),
                });
            }
            a.checked_div(b)
                .map(Value::Int)
                .ok_or_else(|| arithmetic_error("integer overflow"))
        }
        24 => {
            let (a, b) = int2(args, "%")?;
            if b == 0 {
                return Err(VmError {
                    message: with_code("E4003", "division by zero"),
                });
            }
            a.checked_rem(b)
                .map(Value::Int)
                .ok_or_else(|| arithmetic_error("integer overflow"))
        }
        25 => {
            if args.len() != 2 {
                return Err(VmError {
                    message: "== expects two arguments".to_string(),
                });
            }
            Ok(Value::Bool(args[0] == args[1]))
        }
        26 => {
            if args.len() != 2 {
                return Err(VmError {
                    message: "!= expects two arguments".to_string(),
                });
            }
            Ok(Value::Bool(args[0] != args[1]))
        }
        27 => {
            let (a, b) = int2(args, "<")?;
            Ok(Value::Bool(a < b))
        }
        28 => {
            let (a, b) = int2(args, "<=")?;
            Ok(Value::Bool(a <= b))
        }
        29 => {
            let (a, b) = int2(args, ">")?;
            Ok(Value::Bool(a > b))
        }
        30 => {
            let (a, b) = int2(args, ">=")?;
            Ok(Value::Bool(a >= b))
        }
        31 => {
            let (a, b) = bool2(args, "and")?;
            Ok(Value::Bool(a && b))
        }
        32 => {
            let (a, b) = bool2(args, "or")?;
            Ok(Value::Bool(a || b))
        }
        33 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "not expects one argument".to_string(),
                });
            }
            let Value::Bool(v) = args[0] else {
                return Err(VmError {
                    message: "not expects bool arguments".to_string(),
                });
            };
            Ok(Value::Bool(!v))
        }
        34 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "neg expects one argument".to_string(),
                });
            }
            let Value::Int(v) = args[0] else {
                return Err(VmError {
                    message: "neg expects integer arguments".to_string(),
                });
            };
            v.checked_neg()
                .map(Value::Int)
                .ok_or_else(|| arithmetic_error("integer overflow"))
        }
        35 => {
            if args.len() != 2 {
                return Err(VmError {
                    message: "str_cat expects two arguments".to_string(),
                });
            }
            let Value::String(a) = &args[0] else {
                return Err(VmError {
                    message: "str_cat expects string arguments".to_string(),
                });
            };
            let Value::String(b) = &args[1] else {
                return Err(VmError {
                    message: "str_cat expects string arguments".to_string(),
                });
            };
            Ok(Value::String(format!("{a}{b}")))
        }
        36 => {
            if args.len() != 1 {
                return Err(VmError {
                    message: "len expects one argument".to_string(),
                });
            }
            let Value::String(s) = &args[0] else {
                return Err(VmError {
                    message: "len expects string arguments".to_string(),
                });
            };
            Ok(Value::Int(s.chars().count() as i64))
        }
        _ => Err(VmError {
            message: format!("unknown builtin id {id}"),
        }),
    }
}

fn int2(args: &[Value], op: &str) -> Result<(i64, i64), VmError> {
    if args.len() != 2 {
        return Err(VmError {
            message: format!("{op} expects two arguments"),
        });
    }
    let Value::Int(a) = args[0] else {
        return Err(VmError {
            message: format!("{op} expects integer arguments"),
        });
    };
    let Value::Int(b) = args[1] else {
        return Err(VmError {
            message: format!("{op} expects integer arguments"),
        });
    };
    Ok((a, b))
}

fn bool2(args: &[Value], op: &str) -> Result<(bool, bool), VmError> {
    if args.len() != 2 {
        return Err(VmError {
            message: format!("{op} expects two arguments"),
        });
    }
    let Value::Bool(a) = args[0] else {
        return Err(VmError {
            message: format!("{op} expects bool arguments"),
        });
    };
    let Value::Bool(b) = args[1] else {
        return Err(VmError {
            message: format!("{op} expects bool arguments"),
        });
    };
    Ok((a, b))
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

fn json_to_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Adt {
            tag: "Null".to_string(),
            fields: Vec::new(),
        },
        serde_json::Value::Bool(b) => Value::Adt {
            tag: "Bool".to_string(),
            fields: vec![Value::Bool(b)],
        },
        serde_json::Value::Number(n) => Value::Adt {
            tag: "Num".to_string(),
            fields: vec![Value::Float(n.as_f64().unwrap_or(0.0))],
        },
        serde_json::Value::String(s) => Value::Adt {
            tag: "Str".to_string(),
            fields: vec![Value::String(s)],
        },
        serde_json::Value::Array(items) => Value::Adt {
            tag: "Arr".to_string(),
            fields: vec![Value::Array(items.into_iter().map(json_to_value).collect())],
        },
        serde_json::Value::Object(entries) => {
            let mut out = BTreeMap::new();
            for (k, v) in entries {
                out.insert(k, json_to_value(v));
            }
            Value::Adt {
                tag: "Obj".to_string(),
                fields: vec![Value::Map(out)],
            }
        }
    }
}

fn value_to_json(v: &Value) -> Option<serde_json::Value> {
    match v {
        Value::Adt { tag, fields } if tag == "Null" && fields.is_empty() => {
            Some(serde_json::Value::Null)
        }
        Value::Adt { tag, fields } if tag == "Bool" && fields.len() == 1 => match &fields[0] {
            Value::Bool(b) => Some(serde_json::Value::Bool(*b)),
            _ => None,
        },
        Value::Adt { tag, fields } if tag == "Num" && fields.len() == 1 => match &fields[0] {
            Value::Float(v) => serde_json::Number::from_f64(*v).map(serde_json::Value::Number),
            Value::String(s) => serde_json::from_str::<serde_json::Number>(s)
                .ok()
                .map(serde_json::Value::Number),
            Value::Int(i) => Some(serde_json::Value::Number(serde_json::Number::from(*i))),
            _ => None,
        },
        Value::Adt { tag, fields } if tag == "Str" && fields.len() == 1 => match &fields[0] {
            Value::String(s) => Some(serde_json::Value::String(s.clone())),
            _ => None,
        },
        Value::Adt { tag, fields } if tag == "Arr" && fields.len() == 1 => match &fields[0] {
            Value::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(value_to_json(item)?);
                }
                Some(serde_json::Value::Array(out))
            }
            _ => None,
        },
        Value::Adt { tag, fields } if tag == "Obj" && fields.len() == 1 => match &fields[0] {
            Value::Map(entries) => {
                let mut out = serde_json::Map::with_capacity(entries.len());
                for (k, v) in entries {
                    out.insert(k.clone(), value_to_json(v)?);
                }
                Some(serde_json::Value::Object(out))
            }
            _ => None,
        },
        _ => None,
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

fn with_code(code: &str, message: &str) -> String {
    format!("{code}: {message}")
}

fn arithmetic_error(message: &str) -> VmError {
    VmError {
        message: with_code("E4003", message),
    }
}

#[cfg(test)]
mod tests {
    use super::{FuzzHost, Value, VmError, VmHost, call_builtin, json_to_value, value_to_json};

    struct TestHost;

    impl VmHost for TestHost {
        fn io_print(&mut self, _text: &str) -> Result<(), VmError> {
            Ok(())
        }
        fn io_println(&mut self, _text: &str) -> Result<(), VmError> {
            Ok(())
        }
        fn io_readln(&mut self) -> Result<String, VmError> {
            Ok(String::new())
        }
        fn fs_read_to_string(&mut self, _path: &str) -> Result<String, String> {
            Err("disabled".to_string())
        }
        fn fs_write_string(&mut self, _path: &str, _data: &str) -> Result<(), String> {
            Err("disabled".to_string())
        }
        fn proc_run(&mut self, _cmd: &str, _args: &[String]) -> Result<i32, String> {
            Ok(0)
        }
        fn http_get(&mut self, _url: &str) -> Result<String, String> {
            Err("disabled".to_string())
        }
    }

    #[test]
    fn proc_run_builtin_rejects_non_array_args() {
        let mut host = FuzzHost;
        let err = call_builtin(
            &mut host,
            8,
            &[
                Value::String("echo".to_string()),
                Value::String("x".to_string()),
            ],
        )
        .expect_err("run should reject non-array second argument");
        assert!(
            err.to_string()
                .contains("run expects second argument as string array"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn proc_run_builtin_accepts_string_array_args() {
        let mut host = TestHost;
        let value = call_builtin(
            &mut host,
            8,
            &[
                Value::String("echo".to_string()),
                Value::Array(vec![Value::String("ok".to_string())]),
            ],
        )
        .expect("run should accept string array");
        assert!(matches!(value, Value::Adt { tag, .. } if tag == "Ok"));
    }

    #[test]
    fn json_num_payload_uses_float_value() {
        let value = json_to_value(serde_json::json!(1.25));
        let Value::Adt { tag, fields } = value else {
            panic!("expected ADT value");
        };
        assert_eq!(tag, "Num");
        assert!(matches!(fields.as_slice(), [Value::Float(v)] if (*v - 1.25).abs() < f64::EPSILON));
    }

    #[test]
    fn json_num_float_roundtrips_to_json_number() {
        let value = Value::Adt {
            tag: "Num".to_string(),
            fields: vec![Value::Float(2.5)],
        };
        let json = value_to_json(&value).expect("Num(Float) should convert to JSON");
        assert_eq!(json, serde_json::json!(2.5));
    }
}
