use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::ast::{Decl, Expr, Literal, Pattern, Program};

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
}

#[derive(Debug, Clone)]
pub struct FunctionBytecode {
    pub arity: u8,
    pub code: Vec<u8>,
}

#[derive(Default)]
struct CompileCtx {
    strings: Vec<String>,
    string_ids: HashMap<String, u32>,
    ctor_names: HashSet<String>,
    fn_ids: HashMap<String, u32>,
}

struct Lowerer<'a> {
    ctx: &'a mut CompileCtx,
    code: Vec<u8>,
    locals: HashMap<String, u32>,
    next_local: u32,
}

pub fn compile(program: &Program) -> Result<Vec<u8>, BytecodeError> {
    let mut functions = Vec::new();
    for decl in &program.module.decls {
        if let Decl::Function(f) = decl {
            functions.push(f);
        }
    }
    if functions.is_empty() {
        return Err(BytecodeError {
            message: "missing `main` function".to_string(),
        });
    }

    let mut ctx = CompileCtx {
        ctor_names: collect_ctors(program),
        ..CompileCtx::default()
    };
    for (idx, f) in functions.iter().enumerate() {
        ctx.fn_ids.insert(f.name.name.clone(), idx as u32);
    }
    let main_id = *ctx.fn_ids.get("main").ok_or_else(|| BytecodeError {
        message: "missing `main` function".to_string(),
    })?;

    let mut fn_bodies = Vec::new();
    for f in functions {
        let mut lowerer = Lowerer::new(&mut ctx, f.sig.params.len() as u32);
        lowerer.lower_expr(&f.expr)?;
        lowerer.code.push(OpCode::Return as u8);
        fn_bodies.push(FunctionBytecode {
            arity: f.sig.params.len() as u8,
            code: lowerer.code,
        });
    }

    Ok(encode(&ctx.strings, &fn_bodies, main_id))
}

impl<'a> Lowerer<'a> {
    fn new(ctx: &'a mut CompileCtx, arity: u32) -> Self {
        let mut locals = HashMap::new();
        for i in 0..arity {
            locals.insert(format!("arg{i}"), i);
        }
        Lowerer {
            ctx,
            code: Vec::new(),
            locals,
            next_local: arity,
        }
    }

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
                let slot = self.locals.get(&id.name).ok_or_else(|| BytecodeError {
                    message: format!("unsupported unresolved name `{}` in lowering", id.name),
                })?;
                self.code.push(OpCode::LoadLocal as u8);
                self.code.extend_from_slice(&slot.to_le_bytes());
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
                if let Some(old) = prev {
                    self.locals.insert(name.name.clone(), old);
                } else {
                    self.locals.remove(&name.name);
                }
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
                let Expr::Name(name) = &**callee else {
                    return Err(BytecodeError {
                        message: "only direct named calls are supported in v0.1 runtime".to_string(),
                    });
                };
                for arg in args {
                    self.lower_expr(arg)?;
                }
                if let Some(builtin_id) = builtin_id(&name.name) {
                    self.code.push(OpCode::CallBuiltin as u8);
                    self.code.push(builtin_id);
                    self.code.push(args.len() as u8);
                } else if let Some(fn_id) = self.ctx.fn_ids.get(&name.name).copied() {
                    self.code.push(OpCode::CallFn as u8);
                    self.code.extend_from_slice(&fn_id.to_le_bytes());
                    self.code.push(args.len() as u8);
                } else {
                    return Err(BytecodeError {
                        message: format!("unsupported call target `{}`", name.name),
                    });
                }
            }
            Expr::Match { scrutinee, arms, .. } => {
                self.lower_expr(scrutinee)?;
                let scrut_slot = self.alloc_local();
                self.code.push(OpCode::StoreLocal as u8);
                self.code.extend_from_slice(&scrut_slot.to_le_bytes());
                let mut end_jumps = Vec::new();
                for arm in arms {
                    match &arm.pattern {
                        Pattern::Wildcard(_) => {
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
                                if let Some(old_slot) = prev {
                                    self.locals.insert(name, old_slot);
                                } else {
                                    self.locals.remove(&name);
                                }
                            }
                            let end_patch = self.emit_jump_placeholder(OpCode::Jump);
                            end_jumps.push(end_patch);
                            self.patch_jump_to_current(next_patch);
                        }
                        _ => {
                            return Err(BytecodeError {
                                message:
                                    "only boolean and constructor/wildcard patterns are supported in bytecode lowering"
                                        .to_string(),
                            });
                        }
                    }
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
                self.code.push(OpCode::AssertConst as u8);
                self.code.extend_from_slice(&msg_id.to_le_bytes());
            }
            Expr::Ensure { expr, .. } => {
                self.lower_expr(expr)?;
                let msg_id = self.intern_string("contract ensure failure");
                self.code.push(OpCode::AssertConst as u8);
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
            Expr::Lambda { .. } => {
                return Err(BytecodeError {
                    message: "expression form not supported by bytecode lowering yet".to_string(),
                });
            }
        }
        Ok(())
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

fn collect_ctors(program: &Program) -> HashSet<String> {
    let mut set = HashSet::new();
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
        out.extend_from_slice(&(f.code.len() as u32).to_le_bytes());
        out.extend_from_slice(&f.code);
    }
    out.extend_from_slice(&entry_fn.to_le_bytes());
    out
}
