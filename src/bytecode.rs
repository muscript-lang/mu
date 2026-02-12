use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use crate::ast::{Decl, Expr, FunctionDecl, Literal, Param, Pattern, Program, ValueDecl};

pub const MAGIC: &[u8; 4] = b"MUB1";

#[derive(Debug, Clone)]
pub struct BytecodeError {
    pub message: String,
}

impl fmt::Display for BytecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BytecodeError {}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum OpCode {
    PushInt = 1,
    PushBool = 2,
    PushString = 3,
    PushUnit = 4,
    LoadLocal = 5,
    StoreLocal = 6,
    Pop = 7,
    Jump = 8,
    JumpIfFalse = 9,
    CallBuiltin = 10,
    Return = 11,
    MkAdt = 12,
    JumpIfTag = 13,
    AssertConst = 14,
    AssertDyn = 15,
    GetAdtField = 16,
    CallFn = 17,
    MkClosure = 18,
    CallClosure = 19,
    Trap = 20,
    ContractConst = 21,
}

#[derive(Debug, Clone)]
pub struct FunctionBytecode {
    pub arity: u8,
    pub captures: u8,
    pub code: Vec<u8>,
}

#[derive(Default)]
struct CompileCtx {
    strings: Vec<String>,
    string_ids: HashMap<String, u32>,
    ctor_names: HashSet<String>,
    fn_ids: HashMap<String, u32>,
    value_ids: HashMap<String, u32>,
    functions: Vec<FunctionBytecode>,
}

struct Lowerer<'a> {
    ctx: &'a mut CompileCtx,
    code: Vec<u8>,
    locals: BTreeMap<String, u32>,
    next_local: u32,
}

pub fn compile(program: &Program) -> Result<Vec<u8>, BytecodeError> {
    let mut top_functions = Vec::new();
    let mut top_values = Vec::new();
    for decl in &program.module.decls {
        match decl {
            Decl::Function(f) => top_functions.push(f),
            Decl::Value(v) => top_values.push(v),
            _ => {}
        }
    }
    if top_functions.is_empty() {
        return Err(BytecodeError {
            message: "missing `main` function".to_string(),
        });
    }

    let mut ctx = CompileCtx {
        ctor_names: collect_ctors(program),
        ..CompileCtx::default()
    };

    let mut next_id = 0u32;
    for v in &top_values {
        ctx.value_ids.insert(v.name.name.clone(), next_id);
        next_id += 1;
    }
    for f in &top_functions {
        ctx.fn_ids.insert(f.name.name.clone(), next_id);
        next_id += 1;
    }
    let top_len = top_values.len() + top_functions.len();
    ctx.functions = vec![
        FunctionBytecode {
            arity: 0,
            captures: 0,
            code: Vec::new()
        };
        top_len
    ];

    for (idx, v) in top_values.iter().enumerate() {
        let func = lower_top_value(&mut ctx, v)?;
        ctx.functions[idx] = func;
    }
    for (idx, f) in top_functions.iter().enumerate() {
        let func = lower_top_function(&mut ctx, f)?;
        ctx.functions[top_values.len() + idx] = func;
    }

    let entry_fn = *ctx.fn_ids.get("main").ok_or_else(|| BytecodeError {
        message: "missing `main` function".to_string(),
    })?;

    Ok(encode(&ctx.strings, &ctx.functions, entry_fn))
}

fn lower_top_function(ctx: &mut CompileCtx, f: &FunctionDecl) -> Result<FunctionBytecode, BytecodeError> {
    let mut locals = BTreeMap::new();
    for i in 0..f.sig.params.len() {
        locals.insert(format!("arg{i}"), i as u32);
    }
    let mut lowerer = Lowerer {
        ctx,
        code: Vec::new(),
        next_local: f.sig.params.len() as u32,
        locals,
    };
    lowerer.lower_expr(&f.expr)?;
    lowerer.code.push(OpCode::Return as u8);
    Ok(FunctionBytecode {
        arity: f.sig.params.len() as u8,
        captures: 0,
        code: lowerer.code,
    })
}

fn lower_top_value(ctx: &mut CompileCtx, v: &ValueDecl) -> Result<FunctionBytecode, BytecodeError> {
    let mut lowerer = Lowerer {
        ctx,
        code: Vec::new(),
        next_local: 0,
        locals: BTreeMap::new(),
    };
    lowerer.lower_expr(&v.expr)?;
    lowerer.code.push(OpCode::Return as u8);
    Ok(FunctionBytecode {
        arity: 0,
        captures: 0,
        code: lowerer.code,
    })
}

impl<'a> Lowerer<'a> {
    fn lower_expr(&mut self, expr: &Expr) -> Result<(), BytecodeError> {
        match expr {
            Expr::Literal(Literal::Int(v, _)) => {
                self.code.push(OpCode::PushInt as u8);
                self.code.extend_from_slice(&v.to_le_bytes());
            }
            Expr::Literal(Literal::Bool(v, _)) => {
                self.code.push(OpCode::PushBool as u8);
                self.code.push(if *v { 1 } else { 0 });
            }
            Expr::Literal(Literal::String(v, _)) => {
                let id = self.intern_string(v);
                self.code.push(OpCode::PushString as u8);
                self.code.extend_from_slice(&id.to_le_bytes());
            }
            Expr::Unit(_) => self.code.push(OpCode::PushUnit as u8),
            Expr::Name(id) => {
                if let Some(slot) = self.locals.get(&id.name) {
                    self.code.push(OpCode::LoadLocal as u8);
                    self.code.extend_from_slice(&slot.to_le_bytes());
                } else if let Some(value_id) = self.ctx.value_ids.get(&id.name).copied() {
                    self.code.push(OpCode::CallFn as u8);
                    self.code.extend_from_slice(&value_id.to_le_bytes());
                    self.code.push(0);
                } else {
                    return Err(BytecodeError {
                        message: format!("unsupported unresolved name `{}` in lowering", id.name),
                    });
                }
            }
            Expr::Let {
                name, value, body, ..
            } => {
                self.lower_expr(value)?;
                let slot = self.alloc_local();
                self.code.push(OpCode::StoreLocal as u8);
                self.code.extend_from_slice(&slot.to_le_bytes());
                let prev = self.locals.insert(name.name.clone(), slot);
                self.lower_expr(body)?;
                restore_local(&mut self.locals, &name.name, prev);
            }
            Expr::Block { prefix, tail, .. } => {
                for e in prefix {
                    self.lower_expr(e)?;
                    self.code.push(OpCode::Pop as u8);
                }
                self.lower_expr(tail)?;
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.lower_expr(cond)?;
                self.code.push(OpCode::JumpIfFalse as u8);
                let patch_false = self.code.len();
                self.code.extend_from_slice(&0u32.to_le_bytes());
                self.lower_expr(then_branch)?;
                self.code.push(OpCode::Jump as u8);
                let patch_end = self.code.len();
                self.code.extend_from_slice(&0u32.to_le_bytes());
                let false_ip = self.code.len() as u32;
                self.code[patch_false..patch_false + 4].copy_from_slice(&false_ip.to_le_bytes());
                self.lower_expr(else_branch)?;
                let end_ip = self.code.len() as u32;
                self.code[patch_end..patch_end + 4].copy_from_slice(&end_ip.to_le_bytes());
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Name(name) = &**callee {
                    if let Some(builtin_id) = builtin_id(&name.name) {
                        for arg in args {
                            self.lower_expr(arg)?;
                        }
                        self.code.push(OpCode::CallBuiltin as u8);
                        self.code.push(builtin_id);
                        self.code.push(args.len() as u8);
                        return Ok(());
                    }
                    if let Some(fn_id) = self.ctx.fn_ids.get(&name.name).copied() {
                        for arg in args {
                            self.lower_expr(arg)?;
                        }
                        self.code.push(OpCode::CallFn as u8);
                        self.code.extend_from_slice(&fn_id.to_le_bytes());
                        self.code.push(args.len() as u8);
                        return Ok(());
                    }
                    if let Some(slot) = self.locals.get(&name.name).copied() {
                        self.code.push(OpCode::LoadLocal as u8);
                        self.code.extend_from_slice(&slot.to_le_bytes());
                        for arg in args {
                            self.lower_expr(arg)?;
                        }
                        self.code.push(OpCode::CallClosure as u8);
                        self.code.push(args.len() as u8);
                        return Ok(());
                    }
                }

                self.lower_expr(callee)?;
                for arg in args {
                    self.lower_expr(arg)?;
                }
                self.code.push(OpCode::CallClosure as u8);
                self.code.push(args.len() as u8);
            }
            Expr::Lambda { params, body, .. } => {
                let captures = capture_plan(&self.locals, params);
                let lambda_id = self.compile_lambda(params, body, &captures)?;
                for cap in &captures {
                    let slot = self.locals.get(cap).ok_or_else(|| BytecodeError {
                        message: format!("missing capture `{cap}` during lambda lowering"),
                    })?;
                    self.code.push(OpCode::LoadLocal as u8);
                    self.code.extend_from_slice(&slot.to_le_bytes());
                }
                self.code.push(OpCode::MkClosure as u8);
                self.code.extend_from_slice(&lambda_id.to_le_bytes());
                self.code.push(captures.len() as u8);
            }
            Expr::Match { scrutinee, arms, .. } => {
                self.lower_expr(scrutinee)?;
                let scrut_slot = self.alloc_local();
                self.code.push(OpCode::StoreLocal as u8);
                self.code.extend_from_slice(&scrut_slot.to_le_bytes());
                let mut end_jumps = Vec::new();
                let mut has_fallback = false;
                for arm in arms {
                    match &arm.pattern {
                        Pattern::Wildcard(_) => {
                            has_fallback = true;
                            self.lower_expr(&arm.expr)?;
                            let end_patch = self.emit_jump_placeholder(OpCode::Jump);
                            end_jumps.push(end_patch);
                        }
                        Pattern::Literal(Literal::Bool(expected, _)) => {
                            self.code.push(OpCode::LoadLocal as u8);
                            self.code.extend_from_slice(&scrut_slot.to_le_bytes());
                            if *expected {
                                let next_patch = self.emit_jump_placeholder(OpCode::JumpIfFalse);
                                self.lower_expr(&arm.expr)?;
                                let end_patch = self.emit_jump_placeholder(OpCode::Jump);
                                end_jumps.push(end_patch);
                                self.patch_jump_to_current(next_patch);
                            } else {
                                let arm_patch = self.emit_jump_placeholder(OpCode::JumpIfFalse);
                                let next_patch = self.emit_jump_placeholder(OpCode::Jump);
                                self.patch_jump_to_current(arm_patch);
                                self.lower_expr(&arm.expr)?;
                                let end_patch = self.emit_jump_placeholder(OpCode::Jump);
                                end_jumps.push(end_patch);
                                self.patch_jump_to_current(next_patch);
                            }
                        }
                        Pattern::Ctor { name, args, .. } => {
                            let tag_id = self.intern_string(&name.name);
                            self.code.push(OpCode::LoadLocal as u8);
                            self.code.extend_from_slice(&scrut_slot.to_le_bytes());
                            let arm_patch = self.emit_jump_if_tag_placeholder(tag_id);
                            let next_patch = self.emit_jump_placeholder(OpCode::Jump);
                            self.patch_jump_to_current(arm_patch);
                            let mut bound: Vec<(String, Option<u32>)> = Vec::new();
                            for (idx, arg_pat) in args.iter().enumerate() {
                                match arg_pat {
                                    Pattern::Name(id) => {
                                        self.code.push(OpCode::LoadLocal as u8);
                                        self.code.extend_from_slice(&scrut_slot.to_le_bytes());
                                        self.code.push(OpCode::GetAdtField as u8);
                                        self.code.push(idx as u8);
                                        let slot = self.alloc_local();
                                        self.code.push(OpCode::StoreLocal as u8);
                                        self.code.extend_from_slice(&slot.to_le_bytes());
                                        let prev = self.locals.insert(id.name.clone(), slot);
                                        bound.push((id.name.clone(), prev));
                                    }
                                    Pattern::Wildcard(_) => {}
                                    _ => {
                                        return Err(BytecodeError {
                                            message:
                                                "only identifier and wildcard constructor field patterns are supported in bytecode lowering"
                                                    .to_string(),
                                        });
                                    }
                                }
                            }
                            self.lower_expr(&arm.expr)?;
                            for (name, prev) in bound.into_iter().rev() {
                                restore_local(&mut self.locals, &name, prev);
                            }
                            let end_patch = self.emit_jump_placeholder(OpCode::Jump);
                            end_jumps.push(end_patch);
                            self.patch_jump_to_current(next_patch);
                        }
                        Pattern::Name(id) => {
                            if self.ctx.ctor_names.contains(&id.name) {
                                let tag_id = self.intern_string(&id.name);
                                self.code.push(OpCode::LoadLocal as u8);
                                self.code.extend_from_slice(&scrut_slot.to_le_bytes());
                                let arm_patch = self.emit_jump_if_tag_placeholder(tag_id);
                                let next_patch = self.emit_jump_placeholder(OpCode::Jump);
                                self.patch_jump_to_current(arm_patch);
                                self.lower_expr(&arm.expr)?;
                                let end_patch = self.emit_jump_placeholder(OpCode::Jump);
                                end_jumps.push(end_patch);
                                self.patch_jump_to_current(next_patch);
                            } else {
                                has_fallback = true;
                                self.code.push(OpCode::LoadLocal as u8);
                                self.code.extend_from_slice(&scrut_slot.to_le_bytes());
                                let slot = self.alloc_local();
                                self.code.push(OpCode::StoreLocal as u8);
                                self.code.extend_from_slice(&slot.to_le_bytes());
                                let prev = self.locals.insert(id.name.clone(), slot);
                                self.lower_expr(&arm.expr)?;
                                restore_local(&mut self.locals, &id.name, prev);
                                let end_patch = self.emit_jump_placeholder(OpCode::Jump);
                                end_jumps.push(end_patch);
                            }
                        }
                        _ => {
                            return Err(BytecodeError {
                                message:
                                    "only boolean, constructor, name, and wildcard patterns are supported in bytecode lowering"
                                        .to_string(),
                            });
                        }
                    }
                }
                if !has_fallback {
                    let msg_id = self.intern_string("E4005: invalid match");
                    self.code.push(OpCode::Trap as u8);
                    self.code.extend_from_slice(&msg_id.to_le_bytes());
                }
                for patch in end_jumps {
                    self.patch_jump_to_current(patch);
                }
            }
            Expr::Paren { inner, .. } => self.lower_expr(inner)?,
            Expr::Assert { cond, msg, .. } => {
                self.lower_expr(cond)?;
                if let Some(msg_expr) = msg {
                    self.lower_expr(msg_expr)?;
                    self.code.push(OpCode::AssertDyn as u8);
                } else {
                    let msg_id = self.intern_string("assert failure");
                    self.code.push(OpCode::AssertConst as u8);
                    self.code.extend_from_slice(&msg_id.to_le_bytes());
                }
            }
            Expr::Require { expr, .. } => {
                self.lower_expr(expr)?;
                let msg_id = self.intern_string("contract require failure");
                self.code.push(OpCode::ContractConst as u8);
                self.code.extend_from_slice(&msg_id.to_le_bytes());
            }
            Expr::Ensure { expr, .. } => {
                self.lower_expr(expr)?;
                let msg_id = self.intern_string("contract ensure failure");
                self.code.push(OpCode::ContractConst as u8);
                self.code.extend_from_slice(&msg_id.to_le_bytes());
            }
            Expr::NameApp { name, args, .. } => {
                if !self.ctx.ctor_names.contains(&name.name) {
                    return Err(BytecodeError {
                        message: format!(
                            "name application `{}` is not a known constructor in this module",
                            name.name
                        ),
                    });
                }
                for arg in args {
                    self.lower_expr(arg)?;
                }
                let tag_id = self.intern_string(&name.name);
                self.code.push(OpCode::MkAdt as u8);
                self.code.extend_from_slice(&tag_id.to_le_bytes());
                self.code.push(args.len() as u8);
            }
        }
        Ok(())
    }

    fn compile_lambda(
        &mut self,
        params: &[Param],
        body: &Expr,
        captures: &[String],
    ) -> Result<u32, BytecodeError> {
        let lambda_id = self.ctx.functions.len() as u32;
        let mut locals = BTreeMap::new();
        let mut slot = 0u32;
        for cap in captures {
            locals.insert(cap.clone(), slot);
            slot += 1;
        }
        for p in params {
            locals.insert(p.name.name.clone(), slot);
            slot += 1;
        }
        let mut nested = Lowerer {
            ctx: self.ctx,
            code: Vec::new(),
            locals,
            next_local: slot,
        };
        nested.lower_expr(body)?;
        nested.code.push(OpCode::Return as u8);
        nested.ctx.functions.push(FunctionBytecode {
            arity: params.len() as u8,
            captures: captures.len() as u8,
            code: nested.code,
        });
        Ok(lambda_id)
    }

    fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(id) = self.ctx.string_ids.get(s) {
            return *id;
        }
        let id = self.ctx.strings.len() as u32;
        self.ctx.strings.push(s.to_string());
        self.ctx.string_ids.insert(s.to_string(), id);
        id
    }

    fn alloc_local(&mut self) -> u32 {
        let slot = self.next_local;
        self.next_local += 1;
        slot
    }

    fn emit_jump_placeholder(&mut self, op: OpCode) -> usize {
        self.code.push(op as u8);
        let patch = self.code.len();
        self.code.extend_from_slice(&0u32.to_le_bytes());
        patch
    }

    fn emit_jump_if_tag_placeholder(&mut self, tag_id: u32) -> usize {
        self.code.push(OpCode::JumpIfTag as u8);
        self.code.extend_from_slice(&tag_id.to_le_bytes());
        let patch = self.code.len();
        self.code.extend_from_slice(&0u32.to_le_bytes());
        patch
    }

    fn patch_jump_to_current(&mut self, patch_pos: usize) {
        let target = self.code.len() as u32;
        self.code[patch_pos..patch_pos + 4].copy_from_slice(&target.to_le_bytes());
    }
}

fn restore_local(locals: &mut BTreeMap<String, u32>, name: &str, prev: Option<u32>) {
    if let Some(old) = prev {
        locals.insert(name.to_string(), old);
    } else {
        locals.remove(name);
    }
}

fn capture_plan(locals: &BTreeMap<String, u32>, params: &[Param]) -> Vec<String> {
    let param_names = params
        .iter()
        .map(|p| p.name.name.as_str())
        .collect::<HashSet<_>>();
    locals
        .keys()
        .filter(|name| !param_names.contains(name.as_str()))
        .cloned()
        .collect()
}

fn collect_ctors(program: &Program) -> HashSet<String> {
    let mut set = HashSet::new();
    set.insert("Ok".to_string());
    set.insert("Er".to_string());
    for decl in &program.module.decls {
        if let Decl::Type(td) = decl {
            for ctor in &td.ctors {
                set.insert(ctor.name.name.clone());
            }
        }
    }
    set
}

fn builtin_id(name: &str) -> Option<u8> {
    match name {
        "print" => Some(1),
        "println" => Some(2),
        "readln" => Some(3),
        "read" => Some(4),
        "write" => Some(5),
        "parse" => Some(6),
        "stringify" => Some(7),
        "run" => Some(8),
        "get" => Some(9),
        "+" => Some(20),
        "-" => Some(21),
        "*" => Some(22),
        "/" => Some(23),
        "%" => Some(24),
        "==" => Some(25),
        "!=" => Some(26),
        "<" => Some(27),
        "<=" => Some(28),
        ">" => Some(29),
        ">=" => Some(30),
        "and" => Some(31),
        "or" => Some(32),
        "not" => Some(33),
        "neg" => Some(34),
        "str_cat" => Some(35),
        "len" => Some(36),
        _ => None,
    }
}

fn encode(strings: &[String], functions: &[FunctionBytecode], entry_fn: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    for s in strings {
        let bytes = s.as_bytes();
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(bytes);
    }
    out.extend_from_slice(&(functions.len() as u32).to_le_bytes());
    for f in functions {
        out.push(f.arity);
        out.push(f.captures);
        out.extend_from_slice(&(f.code.len() as u32).to_le_bytes());
        out.extend_from_slice(&f.code);
    }
    out.extend_from_slice(&entry_fn.to_le_bytes());
    out
}
