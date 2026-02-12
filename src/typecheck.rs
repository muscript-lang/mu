use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use crate::ast::{
    Decl, EffectAtom, EffectSet, Expr, FunctionType, Literal, Pattern, PrimType, Program, Span,
    TypeExpr,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeErrorCode {
    UnknownName,
    UnknownModule,
    InvalidExport,
    TypeMismatch,
    NotCallable,
    ArityMismatch,
    EffectViolation,
    NonExhaustiveMatch,
    InvalidPattern,
    DuplicateModule,
    DuplicateSymbol,
}

impl TypeErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            TypeErrorCode::UnknownName => "E3001",
            TypeErrorCode::UnknownModule => "E3002",
            TypeErrorCode::InvalidExport => "E3003",
            TypeErrorCode::TypeMismatch => "E3004",
            TypeErrorCode::NotCallable => "E3005",
            TypeErrorCode::ArityMismatch => "E3006",
            TypeErrorCode::EffectViolation => "E3007",
            TypeErrorCode::NonExhaustiveMatch => "E3008",
            TypeErrorCode::InvalidPattern => "E3009",
            TypeErrorCode::DuplicateModule => "E3010",
            TypeErrorCode::DuplicateSymbol => "E3011",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub code: TypeErrorCode,
    pub span: Span,
    pub message: String,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} at bytes {}..{}",
            self.code.as_str(),
            self.message,
            self.span.start,
            self.span.end
        )
    }
}

impl std::error::Error for TypeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Bool,
    String,
    I32,
    I64,
    U32,
    U64,
    F32,
    F64,
    Unit,
    Named(String, Vec<Type>),
    Optional(Box<Type>),
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: EffectSet,
    },
    Result(Box<Type>, Box<Type>),
    TypeVar(String),
}

#[derive(Debug, Clone)]
struct CtorSig {
    parent: String,
    type_params: Vec<String>,
    fields: Vec<TypeExpr>,
}

#[derive(Debug, Clone)]
struct ModuleSigs {
    values: BTreeMap<String, Type>,
    ctors: BTreeMap<String, CtorSig>,
    exports: BTreeSet<String>,
    imports: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct CheckCtx<'a> {
    module_name: &'a str,
    module: &'a ModuleSigs,
    locals: HashMap<String, Type>,
}

#[derive(Debug, Clone)]
struct ExprCheck {
    ty: Type,
    effects: EffectSet,
}

pub fn check_program(program: &Program) -> Result<(), TypeError> {
    check_programs(std::slice::from_ref(program))
}

pub fn check_programs(programs: &[Program]) -> Result<(), TypeError> {
    let modules = build_module_sigs(programs)?;
    for program in programs {
        let name = modid_to_string(&program.module.mod_id.parts);
        check_one_module(program, &name, &modules)?;
    }
    Ok(())
}

fn build_module_sigs(programs: &[Program]) -> Result<BTreeMap<String, ModuleSigs>, TypeError> {
    let mut modules = BTreeMap::new();
    for program in programs {
        let module_name = modid_to_string(&program.module.mod_id.parts);
        if modules.contains_key(&module_name) {
            return Err(TypeError {
                code: TypeErrorCode::DuplicateModule,
                span: program.module.span,
                message: format!("duplicate module `{module_name}`"),
            });
        }
        let mut values = BTreeMap::new();
        let mut ctors = BTreeMap::new();
        let mut exports = BTreeSet::new();
        let mut imports = BTreeMap::new();

        for decl in &program.module.decls {
            match decl {
                Decl::Import(d) => {
                    imports.insert(d.alias.name.clone(), modid_to_string(&d.module.parts));
                }
                Decl::Export(d) => {
                    for name in &d.names {
                        exports.insert(name.name.clone());
                    }
                }
                Decl::Type(d) => {
                    let type_params = d.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
                    for ctor in &d.ctors {
                        if ctors.contains_key(&ctor.name.name) {
                            return Err(TypeError {
                                code: TypeErrorCode::DuplicateSymbol,
                                span: ctor.span,
                                message: format!("duplicate constructor `{}`", ctor.name.name),
                            });
                        }
                        ctors.insert(
                            ctor.name.name.clone(),
                            CtorSig {
                                parent: d.name.name.clone(),
                                type_params: type_params.clone(),
                                fields: ctor.fields.clone(),
                            },
                        );
                    }
                }
                Decl::Value(d) => {
                    if values.contains_key(&d.name.name) {
                        return Err(TypeError {
                            code: TypeErrorCode::DuplicateSymbol,
                            span: d.span,
                            message: format!("duplicate value `{}`", d.name.name),
                        });
                    }
                    values.insert(d.name.name.clone(), ast_type_to_type(&d.ty)?);
                }
                Decl::Function(d) => {
                    if values.contains_key(&d.name.name) {
                        return Err(TypeError {
                            code: TypeErrorCode::DuplicateSymbol,
                            span: d.span,
                            message: format!("duplicate value `{}`", d.name.name),
                        });
                    }
                    values.insert(
                        d.name.name.clone(),
                        Type::Function {
                            params: d.sig.params.iter().map(ast_type_to_type).collect::<Result<_, _>>()?,
                            ret: Box::new(ast_type_to_type(&d.sig.ret)?),
                            effects: d.sig.effects.clone(),
                        },
                    );
                }
            }
        }

        modules.insert(
            module_name,
            ModuleSigs {
                values,
                ctors,
                exports,
                imports,
            },
        );
    }

    for program in programs {
        let module_name = modid_to_string(&program.module.mod_id.parts);
        let sigs = modules.get(&module_name).expect("module must exist");
        for target in sigs.imports.values() {
            if !modules.contains_key(target) {
                return Err(TypeError {
                    code: TypeErrorCode::UnknownModule,
                    span: program.module.span,
                    message: format!("unknown imported module `{target}`"),
                });
            }
        }
        for exported in &sigs.exports {
            if !sigs.values.contains_key(exported) && !sigs.ctors.contains_key(exported) {
                return Err(TypeError {
                    code: TypeErrorCode::InvalidExport,
                    span: program.module.span,
                    message: format!("exported name `{exported}` is not declared"),
                });
            }
        }
    }

    Ok(modules)
}

fn check_one_module(
    program: &Program,
    module_name: &str,
    modules: &BTreeMap<String, ModuleSigs>,
) -> Result<(), TypeError> {
    let module = modules.get(module_name).expect("module sig should exist");
    for decl in &program.module.decls {
        match decl {
            Decl::Import(_) | Decl::Export(_) | Decl::Type(_) => {}
            Decl::Value(v) => {
                let mut ctx = CheckCtx {
                    module_name,
                    module,
                    locals: HashMap::new(),
                };
                let got = check_expr(&mut ctx, &v.expr)?;
                let expected = ast_type_to_type(&v.ty)?;
                expect_type(&expected, &got.ty, v.expr.span())?;
            }
            Decl::Function(f) => {
                let mut ctx = CheckCtx {
                    module_name,
                    module,
                    locals: HashMap::new(),
                };
                for (idx, param_ty) in f.sig.params.iter().enumerate() {
                    let param_name = format!("arg{idx}");
                    ctx.locals.insert(param_name, ast_type_to_type(param_ty)?);
                }
                let got = check_expr(&mut ctx, &f.expr)?;
                let sig = function_type_to_type(&f.sig)?;
                if let Type::Function { ret, effects, .. } = sig {
                    expect_type(&ret, &got.ty, f.expr.span())?;
                    if !effects_is_superset(&effects, &got.effects) {
                        return Err(TypeError {
                            code: TypeErrorCode::EffectViolation,
                            span: f.expr.span(),
                            message: format!(
                                "function `{}` declared effects {} but body needs {}",
                                f.name.name,
                                effect_set_to_string(&effects),
                                effect_set_to_string(&got.effects)
                            ),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_expr(ctx: &mut CheckCtx<'_>, expr: &Expr) -> Result<ExprCheck, TypeError> {
    match expr {
        Expr::Unit(_) => Ok(ExprCheck {
            ty: Type::Unit,
            effects: EffectSet::default(),
        }),
        Expr::Literal(Literal::Bool(_, _)) => Ok(ExprCheck {
            ty: Type::Bool,
            effects: EffectSet::default(),
        }),
        Expr::Literal(Literal::Int(v, _span)) => {
            if *v >= i32::MIN as i64 && *v <= i32::MAX as i64 {
                Ok(ExprCheck {
                    ty: Type::I32,
                    effects: EffectSet::default(),
                })
            } else {
                Ok(ExprCheck {
                    ty: Type::I64,
                    effects: EffectSet::default(),
                })
            }
        }
        Expr::Literal(Literal::String(_, _)) => Ok(ExprCheck {
            ty: Type::String,
            effects: EffectSet::default(),
        }),
        Expr::Name(name) => resolve_name_type(ctx, &name.name, name.span).map(|ty| ExprCheck {
            ty,
            effects: EffectSet::default(),
        }),
        Expr::NameApp { name, args, span } => {
            if let Some(ctor) = ctx.module.ctors.get(&name.name) {
                let (fields, result_ty) = instantiate_ctor_sig(ctor);
                if fields.len() != args.len() {
                    return Err(TypeError {
                        code: TypeErrorCode::ArityMismatch,
                        span: *span,
                        message: format!(
                            "constructor `{}` expects {} args, got {}",
                            name.name,
                            fields.len(),
                            args.len()
                        ),
                    });
                }
                let mut effects = EffectSet::default();
                for (arg, expected) in args.iter().zip(fields.iter()) {
                    let got = check_expr(ctx, arg)?;
                    effects = union_effects(&effects, &got.effects);
                    expect_type(expected, &got.ty, arg.span())?;
                }
                return Ok(ExprCheck {
                    ty: result_ty,
                    effects,
                });
            }
            let callee_ty = resolve_name_type(ctx, &name.name, name.span)?;
            call_type(ctx, callee_ty, args, *span)
        }
        Expr::Call { callee, args, span } => {
            let callee_checked = check_expr(ctx, callee)?;
            let call = call_type(ctx, callee_checked.ty, args, *span)?;
            Ok(ExprCheck {
                ty: call.ty,
                effects: union_effects(&callee_checked.effects, &call.effects),
            })
        }
        Expr::Let {
            name,
            ty,
            value,
            body,
            ..
        } => {
            let value_checked = check_expr(ctx, value)?;
            let bind_ty = if let Some(ann) = ty {
                let ann_ty = ast_type_to_type(ann)?;
                expect_type(&ann_ty, &value_checked.ty, value.span())?;
                ann_ty
            } else {
                value_checked.ty.clone()
            };
            let prev = ctx.locals.insert(name.name.clone(), bind_ty);
            let body_checked = check_expr(ctx, body)?;
            if let Some(old) = prev {
                ctx.locals.insert(name.name.clone(), old);
            } else {
                ctx.locals.remove(&name.name);
            }
            Ok(ExprCheck {
                ty: body_checked.ty,
                effects: union_effects(&value_checked.effects, &body_checked.effects),
            })
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let cond_checked = check_expr(ctx, cond)?;
            expect_type(&Type::Bool, &cond_checked.ty, cond.span())?;
            let then_checked = check_expr(ctx, then_branch)?;
            let else_checked = check_expr(ctx, else_branch)?;
            expect_type(&then_checked.ty, &else_checked.ty, else_branch.span())?;
            Ok(ExprCheck {
                ty: then_checked.ty,
                effects: union_effects(
                    &cond_checked.effects,
                    &union_effects(&then_checked.effects, &else_checked.effects),
                ),
            })
        }
        Expr::Block { prefix, tail, .. } => {
            let mut effects = EffectSet::default();
            for e in prefix {
                let checked = check_expr(ctx, e)?;
                effects = union_effects(&effects, &checked.effects);
            }
            let tail_checked = check_expr(ctx, tail)?;
            Ok(ExprCheck {
                ty: tail_checked.ty,
                effects: union_effects(&effects, &tail_checked.effects),
            })
        }
        Expr::Assert { cond, msg, .. } => {
            let cond_checked = check_expr(ctx, cond)?;
            expect_type(&Type::Bool, &cond_checked.ty, cond.span())?;
            let mut effects = cond_checked.effects;
            if let Some(msg) = msg {
                let msg_checked = check_expr(ctx, msg)?;
                effects = union_effects(&effects, &msg_checked.effects);
            }
            Ok(ExprCheck {
                ty: Type::Unit,
                effects,
            })
        }
        Expr::Require { expr, .. } | Expr::Ensure { expr, .. } => {
            let checked = check_expr(ctx, expr)?;
            expect_type(&Type::Bool, &checked.ty, expr.span())?;
            Ok(ExprCheck {
                ty: Type::Unit,
                effects: checked.effects,
            })
        }
        Expr::Lambda {
            params,
            ret,
            effects,
            body,
            ..
        } => {
            let mut nested = ctx.clone();
            let mut param_types = Vec::new();
            for p in params {
                let ty = ast_type_to_type(&p.ty)?;
                nested.locals.insert(p.name.name.clone(), ty.clone());
                param_types.push(ty);
            }
            let body_checked = check_expr(&mut nested, body)?;
            let ret_ty = ast_type_to_type(ret)?;
            expect_type(&ret_ty, &body_checked.ty, body.span())?;
            if !effects_is_superset(effects, &body_checked.effects) {
                return Err(TypeError {
                    code: TypeErrorCode::EffectViolation,
                    span: body.span(),
                    message: format!(
                        "lambda declared effects {} but body needs {}",
                        effect_set_to_string(effects),
                        effect_set_to_string(&body_checked.effects)
                    ),
                });
            }
            Ok(ExprCheck {
                ty: Type::Function {
                    params: param_types,
                    ret: Box::new(ret_ty),
                    effects: effects.clone(),
                },
                effects: EffectSet::default(),
            })
        }
        Expr::Match { scrutinee, arms, span } => {
            let scrut = check_expr(ctx, scrutinee)?;
            let mut arm_ty: Option<Type> = None;
            let mut effects = scrut.effects;
            let mut seen_bool_true = false;
            let mut seen_bool_false = false;
            let mut seen_wild = false;
            let mut seen_ctors = BTreeSet::new();
            let adt_ctors = adt_constructor_names(ctx, &scrut.ty);

            for arm in arms {
                let mut local_ctx = ctx.clone();
                let cover = check_pattern(&mut local_ctx, &arm.pattern, &scrut.ty)?;
                match cover {
                    PatternCover::BoolTrue => seen_bool_true = true,
                    PatternCover::BoolFalse => seen_bool_false = true,
                    PatternCover::Ctor(name) => {
                        seen_ctors.insert(name);
                    }
                    PatternCover::Wildcard => seen_wild = true,
                    PatternCover::Other => {}
                }
                let arm_checked = check_expr(&mut local_ctx, &arm.expr)?;
                effects = union_effects(&effects, &arm_checked.effects);
                if let Some(expected) = &arm_ty {
                    expect_type(expected, &arm_checked.ty, arm.expr.span())?;
                } else {
                    arm_ty = Some(arm_checked.ty);
                }
            }

            if !seen_wild {
                if scrut.ty == Type::Bool && !(seen_bool_true && seen_bool_false) {
                    return Err(TypeError {
                        code: TypeErrorCode::NonExhaustiveMatch,
                        span: *span,
                        message: "non-exhaustive boolean match".to_string(),
                    });
                }
                if let Some(all) = adt_ctors {
                    if !all.is_empty() && seen_ctors != all {
                        return Err(TypeError {
                            code: TypeErrorCode::NonExhaustiveMatch,
                            span: *span,
                            message: "non-exhaustive ADT match".to_string(),
                        });
                    }
                }
            }

            Ok(ExprCheck {
                ty: arm_ty.unwrap_or(Type::Unit),
                effects,
            })
        }
        Expr::Paren { inner, .. } => check_expr(ctx, inner),
    }
}

enum PatternCover {
    BoolTrue,
    BoolFalse,
    Ctor(String),
    Wildcard,
    Other,
}

fn check_pattern(ctx: &mut CheckCtx<'_>, pat: &Pattern, expected: &Type) -> Result<PatternCover, TypeError> {
    match pat {
        Pattern::Wildcard(_) => Ok(PatternCover::Wildcard),
        Pattern::Literal(Literal::Bool(v, span)) => {
            expect_type(&Type::Bool, expected, *span)?;
            Ok(if *v {
                PatternCover::BoolTrue
            } else {
                PatternCover::BoolFalse
            })
        }
        Pattern::Literal(Literal::Int(v, span)) => {
            let lit_ty = if *v >= i32::MIN as i64 && *v <= i32::MAX as i64 {
                Type::I32
            } else {
                Type::I64
            };
            expect_type(&lit_ty, expected, *span)?;
            Ok(PatternCover::Other)
        }
        Pattern::Literal(Literal::String(_, span)) => {
            expect_type(&Type::String, expected, *span)?;
            Ok(PatternCover::Other)
        }
        Pattern::Name(name) => {
            if let Some(ctor) = ctx.module.ctors.get(&name.name) {
                if ctor.fields.is_empty() {
                    let (_, ctor_ty) = instantiate_ctor_sig(ctor);
                    expect_type(&ctor_ty, expected, name.span)?;
                    return Ok(PatternCover::Ctor(name.name.clone()));
                }
            }
            ctx.locals.insert(name.name.clone(), expected.clone());
            Ok(PatternCover::Other)
        }
        Pattern::Ctor { name, args, span } => {
            let ctor = ctx.module.ctors.get(&name.name).ok_or_else(|| TypeError {
                code: TypeErrorCode::InvalidPattern,
                span: name.span,
                message: format!("unknown constructor `{}`", name.name),
            })?;
            let (fields, ctor_ty) = instantiate_ctor_sig(ctor);
            expect_type(&ctor_ty, expected, *span)?;
            if fields.len() != args.len() {
                return Err(TypeError {
                    code: TypeErrorCode::ArityMismatch,
                    span: *span,
                    message: format!(
                        "constructor `{}` pattern expects {} args, got {}",
                        name.name,
                        fields.len(),
                        args.len()
                    ),
                });
            }
            for (arg, field_ty) in args.iter().zip(fields.iter()) {
                check_pattern(ctx, arg, field_ty)?;
            }
            Ok(PatternCover::Ctor(name.name.clone()))
        }
        Pattern::Tuple { items, span } => {
            let Type::Tuple(expected_items) = expected else {
                return Err(TypeError {
                    code: TypeErrorCode::InvalidPattern,
                    span: *span,
                    message: "tuple pattern requires tuple scrutinee".to_string(),
                });
            };
            if items.len() != expected_items.len() {
                return Err(TypeError {
                    code: TypeErrorCode::ArityMismatch,
                    span: *span,
                    message: format!(
                        "tuple pattern expects {} items, got {}",
                        expected_items.len(),
                        items.len()
                    ),
                });
            }
            for (item, expected_item) in items.iter().zip(expected_items.iter()) {
                check_pattern(ctx, item, expected_item)?;
            }
            Ok(PatternCover::Other)
        }
        Pattern::Paren { inner, .. } => check_pattern(ctx, inner, expected),
    }
}

fn call_type(
    ctx: &mut CheckCtx<'_>,
    callee_ty: Type,
    args: &[Expr],
    span: Span,
) -> Result<ExprCheck, TypeError> {
    let Type::Function {
        params,
        ret,
        effects: call_effects,
    } = callee_ty
    else {
        return Err(TypeError {
            code: TypeErrorCode::NotCallable,
            span,
            message: "attempted to call a non-function value".to_string(),
        });
    };
    if params.len() != args.len() {
        return Err(TypeError {
            code: TypeErrorCode::ArityMismatch,
            span,
            message: format!("call expects {} args, got {}", params.len(), args.len()),
        });
    }
    let mut effects = call_effects;
    for (arg, expected) in args.iter().zip(params.iter()) {
        let got = check_expr(ctx, arg)?;
        effects = union_effects(&effects, &got.effects);
        expect_type(expected, &got.ty, arg.span())?;
    }
    Ok(ExprCheck { ty: *ret, effects })
}

fn resolve_name_type(ctx: &CheckCtx<'_>, name: &str, span: Span) -> Result<Type, TypeError> {
    if let Some(ty) = ctx.locals.get(name) {
        return Ok(ty.clone());
    }
    if let Some(ty) = ctx.module.values.get(name) {
        return Ok(ty.clone());
    }
    if let Some(ty) = builtin_values().get(name) {
        return Ok(ty.clone());
    }
    Err(TypeError {
        code: TypeErrorCode::UnknownName,
        span,
        message: format!("unknown name `{name}` in module `{}`", ctx.module_name),
    })
}

fn function_type_to_type(sig: &FunctionType) -> Result<Type, TypeError> {
    Ok(Type::Function {
        params: sig.params.iter().map(ast_type_to_type).collect::<Result<_, _>>()?,
        ret: Box::new(ast_type_to_type(&sig.ret)?),
        effects: sig.effects.clone(),
    })
}

fn instantiate_ctor_sig(sig: &CtorSig) -> (Vec<Type>, Type) {
    let mut map = HashMap::new();
    for tp in &sig.type_params {
        map.insert(tp.clone(), Type::TypeVar(tp.clone()));
    }
    let fields = sig
        .fields
        .iter()
        .map(|field| ast_type_to_type_with_vars(field, &map))
        .collect::<Vec<_>>();
    let params = sig
        .type_params
        .iter()
        .map(|tp| map.get(tp).cloned().expect("type var must exist"))
        .collect::<Vec<_>>();
    (fields, Type::Named(sig.parent.clone(), params))
}

fn adt_constructor_names(ctx: &CheckCtx<'_>, ty: &Type) -> Option<BTreeSet<String>> {
    let Type::Named(name, _) = ty else {
        return None;
    };
    let mut out = BTreeSet::new();
    for (ctor, sig) in &ctx.module.ctors {
        if &sig.parent == name {
            out.insert(ctor.clone());
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

fn ast_type_to_type(ty: &TypeExpr) -> Result<Type, TypeError> {
    Ok(ast_type_to_type_with_vars(ty, &HashMap::new()))
}

fn ast_type_to_type_with_vars(ty: &TypeExpr, vars: &HashMap<String, Type>) -> Type {
    match ty {
        TypeExpr::Prim(prim, _) => match prim {
            PrimType::Bool => Type::Bool,
            PrimType::String => Type::String,
            PrimType::I32 => Type::I32,
            PrimType::I64 => Type::I64,
            PrimType::U32 => Type::U32,
            PrimType::U64 => Type::U64,
            PrimType::F32 => Type::F32,
            PrimType::F64 => Type::F64,
            PrimType::Unit => Type::Unit,
        },
        TypeExpr::Named { name, args, .. } => {
            if args.is_empty() {
                if let Some(v) = vars.get(&name.name) {
                    return v.clone();
                }
            }
            Type::Named(
                name.name.clone(),
                args.iter().map(|a| ast_type_to_type_with_vars(a, vars)).collect(),
            )
        }
        TypeExpr::Optional { inner, .. } => Type::Optional(Box::new(ast_type_to_type_with_vars(inner, vars))),
        TypeExpr::Array { inner, .. } => Type::Array(Box::new(ast_type_to_type_with_vars(inner, vars))),
        TypeExpr::Map { key, value, .. } => Type::Map(
            Box::new(ast_type_to_type_with_vars(key, vars)),
            Box::new(ast_type_to_type_with_vars(value, vars)),
        ),
        TypeExpr::Tuple { items, .. } => {
            Type::Tuple(items.iter().map(|i| ast_type_to_type_with_vars(i, vars)).collect())
        }
        TypeExpr::Function { sig, .. } => Type::Function {
            params: sig
                .params
                .iter()
                .map(|p| ast_type_to_type_with_vars(p, vars))
                .collect(),
            ret: Box::new(ast_type_to_type_with_vars(&sig.ret, vars)),
            effects: sig.effects.clone(),
        },
        TypeExpr::ResultSugar { ok, err, .. } => Type::Result(
            Box::new(ast_type_to_type_with_vars(ok, vars)),
            Box::new(ast_type_to_type_with_vars(err, vars)),
        ),
        TypeExpr::Group { inner, .. } => ast_type_to_type_with_vars(inner, vars),
    }
}

fn expect_type(expected: &Type, got: &Type, span: Span) -> Result<(), TypeError> {
    if expected == got {
        return Ok(());
    }
    Err(TypeError {
        code: TypeErrorCode::TypeMismatch,
        span,
        message: format!("type mismatch: expected {}, got {}", show_type(expected), show_type(got)),
    })
}

fn show_type(ty: &Type) -> String {
    match ty {
        Type::Bool => "b".to_string(),
        Type::String => "s".to_string(),
        Type::I32 => "i32".to_string(),
        Type::I64 => "i64".to_string(),
        Type::U32 => "u32".to_string(),
        Type::U64 => "u64".to_string(),
        Type::F32 => "f32".to_string(),
        Type::F64 => "f64".to_string(),
        Type::Unit => "unit".to_string(),
        Type::Named(name, args) => {
            if args.is_empty() {
                name.clone()
            } else {
                format!(
                    "{}[{}]",
                    name,
                    args.iter().map(show_type).collect::<Vec<_>>().join(",")
                )
            }
        }
        Type::Optional(inner) => format!("?{}", show_type(inner)),
        Type::Array(inner) => format!("{}[]", show_type(inner)),
        Type::Map(k, v) => format!("{{{}:{}}}", show_type(k), show_type(v)),
        Type::Tuple(items) => format!("({})", items.iter().map(show_type).collect::<Vec<_>>().join(",")),
        Type::Function {
            params,
            ret,
            effects,
        } => format!(
            "({})->{}{}",
            params.iter().map(show_type).collect::<Vec<_>>().join(","),
            show_type(ret),
            effect_set_to_string(effects)
        ),
        Type::Result(ok, err) => format!("{}!{}", show_type(ok), show_type(err)),
        Type::TypeVar(v) => v.clone(),
    }
}

fn effects_is_superset(container: &EffectSet, needed: &EffectSet) -> bool {
    needed.atoms.iter().all(|a| container.atoms.contains(a))
}

fn union_effects(a: &EffectSet, b: &EffectSet) -> EffectSet {
    let mut atoms = Vec::new();
    for atom in [
        EffectAtom::Io,
        EffectAtom::Fs,
        EffectAtom::Net,
        EffectAtom::Proc,
        EffectAtom::Rand,
        EffectAtom::Time,
        EffectAtom::St,
    ] {
        if a.atoms.contains(&atom) || b.atoms.contains(&atom) {
            atoms.push(atom);
        }
    }
    EffectSet { atoms }
}

fn effect_set_to_string(effects: &EffectSet) -> String {
    if effects.atoms.is_empty() {
        String::new()
    } else {
        let names = effects
            .atoms
            .iter()
            .map(|a| match a {
                EffectAtom::Io => "io",
                EffectAtom::Fs => "fs",
                EffectAtom::Net => "net",
                EffectAtom::Proc => "proc",
                EffectAtom::Rand => "rand",
                EffectAtom::Time => "time",
                EffectAtom::St => "st",
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("!{{{names}}}")
    }
}

fn modid_to_string(parts: &[String]) -> String {
    parts.join(".")
}

fn builtin_values() -> &'static BTreeMap<String, Type> {
    use std::sync::OnceLock;
    static MAP: OnceLock<BTreeMap<String, Type>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = BTreeMap::new();
        map.insert(
            "print".to_string(),
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Unit),
                effects: EffectSet {
                    atoms: vec![EffectAtom::Io],
                },
            },
        );
        map.insert(
            "println".to_string(),
            Type::Function {
                params: vec![Type::String],
                ret: Box::new(Type::Unit),
                effects: EffectSet {
                    atoms: vec![EffectAtom::Io],
                },
            },
        );
        map.insert(
            "readln".to_string(),
            Type::Function {
                params: vec![],
                ret: Box::new(Type::String),
                effects: EffectSet {
                    atoms: vec![EffectAtom::Io],
                },
            },
        );
        map
    })
}
