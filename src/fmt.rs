use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{
    Decl, EffectAtom, EffectSet, Expr, FunctionType, Ident, Literal, MatchArm, Module, Name, Param,
    Pattern, PrimType, Program, TypeExpr,
};
use crate::parser::{ParseError, parse_str};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmtMode {
    Readable,
    Compressed,
}

pub fn parse_and_format(src: &str) -> Result<String, ParseError> {
    parse_and_format_mode(src, FmtMode::Readable)
}

pub fn parse_and_format_mode(src: &str, mode: FmtMode) -> Result<String, ParseError> {
    let program = parse_str(src)?;
    Ok(format_program_mode(&program, mode))
}

pub fn format_program(program: &Program) -> String {
    format_program_mode(program, FmtMode::Readable)
}

pub fn format_program_mode(program: &Program, mode: FmtMode) -> String {
    let mut out = String::new();
    out.push('@');
    out.push_str(&program.module.mod_id.parts.join("."));
    out.push('{');

    let compressed_table = match mode {
        FmtMode::Readable => None,
        FmtMode::Compressed => Some(build_compressed_symtab(&program.module)),
    };

    if let Some(table) = compressed_table.as_ref() {
        out.push('$');
        out.push('[');
        for (i, name) in table.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(name);
        }
        out.push(']');
        out.push(';');
    }

    for decl in &program.module.decls {
        format_decl(
            decl,
            &program.module,
            compressed_table.as_ref(),
            mode,
            &mut out,
        );
    }
    out.push('}');
    out.push('\n');
    out
}

fn build_compressed_symtab(module: &Module) -> Vec<String> {
    let mut eligible = BTreeSet::new();
    for decl in &module.decls {
        collect_decl_bindings(decl, module, &mut eligible);
    }
    let mut counts = BTreeMap::new();
    for decl in &module.decls {
        count_decl_names(decl, module, &eligible, &mut counts);
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(name_a, count_a), (name_b, count_b)| {
        count_b.cmp(count_a).then_with(|| name_a.cmp(name_b))
    });
    let mut selected = Vec::new();
    for (name, count) in ranked {
        let index = selected.len();
        let index_width = digit_count(index);
        let replacement_gain =
            (count as isize) * (name.len() as isize - (1 + index_width) as isize);
        let table_cost = name.len() as isize + if selected.is_empty() { 0 } else { 1 };
        if replacement_gain > table_cost {
            selected.push(name);
        }
    }
    selected
}

fn digit_count(n: usize) -> usize {
    n.to_string().len()
}

fn collect_decl_bindings(decl: &Decl, module: &Module, out: &mut BTreeSet<String>) {
    match decl {
        Decl::Import(d) => {
            collect_binding_ident(&d.alias, module, out);
        }
        Decl::Export(d) => {
            for name in &d.names {
                collect_binding_ident(name, module, out);
            }
        }
        Decl::Type(d) => {
            collect_binding_ident(&d.name, module, out);
            for p in &d.params {
                collect_binding_ident(p, module, out);
            }
            for ctor in &d.ctors {
                collect_binding_ident(&ctor.name, module, out);
                for field in &ctor.fields {
                    collect_type_bindings(field, module, out);
                }
            }
        }
        Decl::Value(d) => {
            collect_binding_ident(&d.name, module, out);
            collect_type_bindings(&d.ty, module, out);
            collect_expr_bindings(&d.expr, module, out);
        }
        Decl::Function(d) => {
            collect_binding_ident(&d.name, module, out);
            for tp in &d.type_params {
                collect_binding_ident(tp, module, out);
            }
            collect_function_type_bindings(&d.sig, module, out);
            collect_expr_bindings(&d.expr, module, out);
        }
    }
}

fn collect_function_type_bindings(sig: &FunctionType, module: &Module, out: &mut BTreeSet<String>) {
    for p in &sig.params {
        collect_type_bindings(p, module, out);
    }
    collect_type_bindings(&sig.ret, module, out);
}

fn collect_type_bindings(ty: &TypeExpr, module: &Module, out: &mut BTreeSet<String>) {
    match ty {
        TypeExpr::Prim(_, _) => {}
        TypeExpr::Named { name, args, .. } => {
            collect_binding_ident(name, module, out);
            for arg in args {
                collect_type_bindings(arg, module, out);
            }
        }
        TypeExpr::Optional { inner, .. }
        | TypeExpr::Array { inner, .. }
        | TypeExpr::Group { inner, .. } => {
            collect_type_bindings(inner, module, out);
        }
        TypeExpr::Map { key, value, .. } => {
            collect_type_bindings(key, module, out);
            collect_type_bindings(value, module, out);
        }
        TypeExpr::Tuple { items, .. } => {
            for item in items {
                collect_type_bindings(item, module, out);
            }
        }
        TypeExpr::Function { sig, .. } => collect_function_type_bindings(sig, module, out),
        TypeExpr::ResultSugar { ok, err, .. } => {
            collect_type_bindings(ok, module, out);
            collect_type_bindings(err, module, out);
        }
    }
}

fn collect_expr_bindings(expr: &Expr, module: &Module, out: &mut BTreeSet<String>) {
    match expr {
        Expr::Block { prefix, tail, .. } => {
            for e in prefix {
                collect_expr_bindings(e, module, out);
            }
            collect_expr_bindings(tail, module, out);
        }
        Expr::Unit(_) | Expr::Literal(_) => {}
        Expr::Let {
            name,
            ty,
            value,
            body,
            ..
        } => {
            collect_binding_ident(name, module, out);
            if let Some(ty) = ty {
                collect_type_bindings(ty, module, out);
            }
            collect_expr_bindings(value, module, out);
            collect_expr_bindings(body, module, out);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_bindings(cond, module, out);
            collect_expr_bindings(then_branch, module, out);
            collect_expr_bindings(else_branch, module, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_expr_bindings(scrutinee, module, out);
            for arm in arms {
                collect_pattern_bindings(&arm.pattern, module, out);
                collect_expr_bindings(&arm.expr, module, out);
            }
        }
        Expr::Call { callee, args, .. } => {
            collect_expr_bindings(callee, module, out);
            for arg in args {
                collect_expr_bindings(arg, module, out);
            }
        }
        Expr::Lambda {
            params, ret, body, ..
        } => {
            for p in params {
                collect_binding_ident(&p.name, module, out);
                collect_type_bindings(&p.ty, module, out);
            }
            collect_type_bindings(ret, module, out);
            collect_expr_bindings(body, module, out);
        }
        Expr::Assert { cond, msg, .. } => {
            collect_expr_bindings(cond, module, out);
            if let Some(msg) = msg {
                collect_expr_bindings(msg, module, out);
            }
        }
        Expr::Require { expr, .. }
        | Expr::Ensure { expr, .. }
        | Expr::Paren { inner: expr, .. } => {
            collect_expr_bindings(expr, module, out);
        }
        Expr::Name(_) => {}
        Expr::NameApp { name, args, .. } => {
            collect_binding_ident(name, module, out);
            for arg in args {
                collect_expr_bindings(arg, module, out);
            }
        }
    }
}

fn collect_pattern_bindings(pat: &Pattern, module: &Module, out: &mut BTreeSet<String>) {
    match pat {
        Pattern::Wildcard(_) | Pattern::Literal(_) => {}
        Pattern::Name(id) => collect_binding_ident(id, module, out),
        Pattern::Ctor { name, args, .. } => {
            collect_binding_ident(name, module, out);
            for arg in args {
                collect_pattern_bindings(arg, module, out);
            }
        }
        Pattern::Tuple { items, .. } => {
            for item in items {
                collect_pattern_bindings(item, module, out);
            }
        }
        Pattern::Paren { inner, .. } => collect_pattern_bindings(inner, module, out),
    }
}

fn collect_binding_ident(id: &Ident, module: &Module, out: &mut BTreeSet<String>) {
    if let Some(name) = resolve_ident(module, id) {
        if !is_core_literal_name(&name) {
            out.insert(name);
        }
    }
}

fn count_decl_names(
    decl: &Decl,
    module: &Module,
    eligible: &BTreeSet<String>,
    out: &mut BTreeMap<String, usize>,
) {
    match decl {
        Decl::Import(d) => {
            count_ident(&d.alias, module, eligible, out);
        }
        Decl::Export(d) => {
            for name in &d.names {
                count_ident(name, module, eligible, out);
            }
        }
        Decl::Type(d) => {
            count_ident(&d.name, module, eligible, out);
            for p in &d.params {
                count_ident(p, module, eligible, out);
            }
            for ctor in &d.ctors {
                count_ident(&ctor.name, module, eligible, out);
                for field in &ctor.fields {
                    count_type_names(field, module, eligible, out);
                }
            }
        }
        Decl::Value(d) => {
            count_ident(&d.name, module, eligible, out);
            count_type_names(&d.ty, module, eligible, out);
            count_expr_names(&d.expr, module, eligible, out);
        }
        Decl::Function(d) => {
            count_ident(&d.name, module, eligible, out);
            for tp in &d.type_params {
                count_ident(tp, module, eligible, out);
            }
            count_function_type_names(&d.sig, module, eligible, out);
            count_expr_names(&d.expr, module, eligible, out);
        }
    }
}

fn count_function_type_names(
    sig: &FunctionType,
    module: &Module,
    eligible: &BTreeSet<String>,
    out: &mut BTreeMap<String, usize>,
) {
    for p in &sig.params {
        count_type_names(p, module, eligible, out);
    }
    count_type_names(&sig.ret, module, eligible, out);
}

fn count_type_names(
    ty: &TypeExpr,
    module: &Module,
    eligible: &BTreeSet<String>,
    out: &mut BTreeMap<String, usize>,
) {
    match ty {
        TypeExpr::Prim(_, _) => {}
        TypeExpr::Named { name, args, .. } => {
            count_ident(name, module, eligible, out);
            for arg in args {
                count_type_names(arg, module, eligible, out);
            }
        }
        TypeExpr::Optional { inner, .. }
        | TypeExpr::Array { inner, .. }
        | TypeExpr::Group { inner, .. } => {
            count_type_names(inner, module, eligible, out);
        }
        TypeExpr::Map { key, value, .. } => {
            count_type_names(key, module, eligible, out);
            count_type_names(value, module, eligible, out);
        }
        TypeExpr::Tuple { items, .. } => {
            for item in items {
                count_type_names(item, module, eligible, out);
            }
        }
        TypeExpr::Function { sig, .. } => count_function_type_names(sig, module, eligible, out),
        TypeExpr::ResultSugar { ok, err, .. } => {
            count_type_names(ok, module, eligible, out);
            count_type_names(err, module, eligible, out);
        }
    }
}

fn count_expr_names(
    expr: &Expr,
    module: &Module,
    eligible: &BTreeSet<String>,
    out: &mut BTreeMap<String, usize>,
) {
    match expr {
        Expr::Block { prefix, tail, .. } => {
            for e in prefix {
                count_expr_names(e, module, eligible, out);
            }
            count_expr_names(tail, module, eligible, out);
        }
        Expr::Unit(_) | Expr::Literal(_) => {}
        Expr::Let {
            name,
            ty,
            value,
            body,
            ..
        } => {
            count_ident(name, module, eligible, out);
            if let Some(ty) = ty {
                count_type_names(ty, module, eligible, out);
            }
            count_expr_names(value, module, eligible, out);
            count_expr_names(body, module, eligible, out);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            count_expr_names(cond, module, eligible, out);
            count_expr_names(then_branch, module, eligible, out);
            count_expr_names(else_branch, module, eligible, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            count_expr_names(scrutinee, module, eligible, out);
            for arm in arms {
                count_pattern_names(&arm.pattern, module, eligible, out);
                count_expr_names(&arm.expr, module, eligible, out);
            }
        }
        Expr::Call { callee, args, .. } => {
            count_expr_names(callee, module, eligible, out);
            for arg in args {
                count_expr_names(arg, module, eligible, out);
            }
        }
        Expr::Lambda {
            params, ret, body, ..
        } => {
            for p in params {
                count_ident(&p.name, module, eligible, out);
                count_type_names(&p.ty, module, eligible, out);
            }
            count_type_names(ret, module, eligible, out);
            count_expr_names(body, module, eligible, out);
        }
        Expr::Assert { cond, msg, .. } => {
            count_expr_names(cond, module, eligible, out);
            if let Some(msg) = msg {
                count_expr_names(msg, module, eligible, out);
            }
        }
        Expr::Require { expr, .. }
        | Expr::Ensure { expr, .. }
        | Expr::Paren { inner: expr, .. } => {
            count_expr_names(expr, module, eligible, out);
        }
        Expr::Name(name) => count_ident(name, module, eligible, out),
        Expr::NameApp { name, args, .. } => {
            count_ident(name, module, eligible, out);
            for arg in args {
                count_expr_names(arg, module, eligible, out);
            }
        }
    }
}

fn count_pattern_names(
    pat: &Pattern,
    module: &Module,
    eligible: &BTreeSet<String>,
    out: &mut BTreeMap<String, usize>,
) {
    match pat {
        Pattern::Wildcard(_) | Pattern::Literal(_) => {}
        Pattern::Name(id) => count_ident(id, module, eligible, out),
        Pattern::Ctor { name, args, .. } => {
            count_ident(name, module, eligible, out);
            for arg in args {
                count_pattern_names(arg, module, eligible, out);
            }
        }
        Pattern::Tuple { items, .. } => {
            for item in items {
                count_pattern_names(item, module, eligible, out);
            }
        }
        Pattern::Paren { inner, .. } => count_pattern_names(inner, module, eligible, out),
    }
}

fn count_ident(
    id: &Ident,
    module: &Module,
    eligible: &BTreeSet<String>,
    out: &mut BTreeMap<String, usize>,
) {
    if let Some(name) = resolve_ident(module, id) {
        if eligible.contains(&name) && !is_core_literal_name(&name) {
            *out.entry(name).or_insert(0) += 1;
        }
    }
}

fn is_core_literal_name(name: &str) -> bool {
    matches!(
        name,
        "E" | "T" | "V" | "F" | "v" | "i" | "m" | "l" | "c" | "a" | "t" | "f"
    )
}

fn resolve_ident(module: &Module, id: &Ident) -> Option<String> {
    match &id.name {
        Name::Ident(name) => Some(name.clone()),
        Name::Sym(idx) => module
            .symtab
            .as_ref()
            .and_then(|s| s.get(*idx as usize))
            .cloned(),
    }
}

fn render_name(
    module: &Module,
    id: &Ident,
    compressed_table: Option<&Vec<String>>,
    mode: FmtMode,
) -> String {
    match mode {
        FmtMode::Readable => resolve_ident(module, id).unwrap_or_else(|| id.display()),
        FmtMode::Compressed => {
            let Some(table) = compressed_table else {
                return resolve_ident(module, id).unwrap_or_else(|| id.display());
            };
            let Some(name) = resolve_ident(module, id) else {
                return id.display();
            };
            let mut index_by_name = HashMap::new();
            for (i, sym) in table.iter().enumerate() {
                index_by_name.insert(sym.as_str(), i);
            }
            if let Some(idx) = index_by_name.get(name.as_str()) {
                format!("#{idx}")
            } else {
                name
            }
        }
    }
}

fn format_decl(
    decl: &Decl,
    module: &Module,
    compressed_table: Option<&Vec<String>>,
    mode: FmtMode,
    out: &mut String,
) {
    match decl {
        Decl::Import(d) => {
            out.push(':');
            out.push_str(&render_name(module, &d.alias, compressed_table, mode));
            out.push('=');
            out.push_str(&d.module.parts.join("."));
            out.push(';');
        }
        Decl::Export(d) => {
            out.push('E');
            out.push('[');
            for (i, name) in d.names.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&render_name(module, name, compressed_table, mode));
            }
            out.push(']');
            out.push(';');
        }
        Decl::Type(d) => {
            out.push_str("T ");
            out.push_str(&render_name(module, &d.name, compressed_table, mode));
            if !d.params.is_empty() {
                out.push('[');
                for (i, p) in d.params.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&render_name(module, p, compressed_table, mode));
                }
                out.push(']');
            }
            out.push('=');
            for (i, ctor) in d.ctors.iter().enumerate() {
                if i > 0 {
                    out.push('|');
                }
                out.push_str(&render_name(module, &ctor.name, compressed_table, mode));
                if !ctor.fields.is_empty() {
                    out.push('(');
                    for (j, ty) in ctor.fields.iter().enumerate() {
                        if j > 0 {
                            out.push(',');
                        }
                        format_type(ty, module, compressed_table, mode, out);
                    }
                    out.push(')');
                }
            }
            out.push(';');
        }
        Decl::Value(d) => {
            out.push_str("V ");
            out.push_str(&render_name(module, &d.name, compressed_table, mode));
            out.push(':');
            format_type(&d.ty, module, compressed_table, mode, out);
            out.push('=');
            format_expr(&d.expr, module, compressed_table, mode, out);
            out.push(';');
        }
        Decl::Function(d) => {
            out.push_str("F ");
            out.push_str(&render_name(module, &d.name, compressed_table, mode));
            if !d.type_params.is_empty() {
                out.push('[');
                for (i, tp) in d.type_params.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&render_name(module, tp, compressed_table, mode));
                }
                out.push(']');
            }
            out.push(':');
            format_function_type(&d.sig, module, compressed_table, mode, out);
            out.push('=');
            format_expr(&d.expr, module, compressed_table, mode, out);
            out.push(';');
        }
    }
}

fn format_effect_set(effects: &EffectSet, mode: FmtMode, out: &mut String) {
    let atoms = canonical_effect_atoms(effects);
    if atoms.is_empty() {
        return;
    }
    out.push_str("!{");
    for (i, atom) in atoms.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(match (mode, atom) {
            (FmtMode::Readable, EffectAtom::Io) => "io",
            (FmtMode::Readable, EffectAtom::Fs) => "fs",
            (FmtMode::Readable, EffectAtom::Net) => "net",
            (FmtMode::Readable, EffectAtom::Proc) => "proc",
            (FmtMode::Readable, EffectAtom::Rand) => "rand",
            (FmtMode::Readable, EffectAtom::Time) => "time",
            (FmtMode::Readable, EffectAtom::St) => "st",
            (FmtMode::Compressed, EffectAtom::Io) => "I",
            (FmtMode::Compressed, EffectAtom::Fs) => "F",
            (FmtMode::Compressed, EffectAtom::Net) => "N",
            (FmtMode::Compressed, EffectAtom::Proc) => "P",
            (FmtMode::Compressed, EffectAtom::Rand) => "R",
            (FmtMode::Compressed, EffectAtom::Time) => "T",
            (FmtMode::Compressed, EffectAtom::St) => "S",
        });
    }
    out.push('}');
}

fn canonical_effect_atoms(effects: &EffectSet) -> Vec<EffectAtom> {
    let mut out = Vec::new();
    for atom in [
        EffectAtom::Io,
        EffectAtom::Fs,
        EffectAtom::Net,
        EffectAtom::Proc,
        EffectAtom::Rand,
        EffectAtom::Time,
        EffectAtom::St,
    ] {
        if effects.atoms.contains(&atom) {
            out.push(atom);
        }
    }
    out
}

fn format_function_type(
    sig: &FunctionType,
    module: &Module,
    compressed_table: Option<&Vec<String>>,
    mode: FmtMode,
    out: &mut String,
) {
    out.push('(');
    for (i, ty) in sig.params.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        format_type(ty, module, compressed_table, mode, out);
    }
    out.push(')');
    out.push_str("->");
    format_type(&sig.ret, module, compressed_table, mode, out);
    format_effect_set(&sig.effects, mode, out);
}

fn format_type(
    ty: &TypeExpr,
    module: &Module,
    compressed_table: Option<&Vec<String>>,
    mode: FmtMode,
    out: &mut String,
) {
    match ty {
        TypeExpr::Prim(prim, _) => out.push_str(match prim {
            PrimType::Bool => "b",
            PrimType::String => "s",
            PrimType::I32 => "i32",
            PrimType::I64 => "i64",
            PrimType::U32 => "u32",
            PrimType::U64 => "u64",
            PrimType::F32 => "f32",
            PrimType::F64 => "f64",
            PrimType::Unit => "unit",
        }),
        TypeExpr::Named { name, args, .. } => {
            out.push_str(&render_name(module, name, compressed_table, mode));
            if !args.is_empty() {
                out.push('[');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    format_type(arg, module, compressed_table, mode, out);
                }
                out.push(']');
            }
        }
        TypeExpr::Optional { inner, .. } => {
            out.push('?');
            format_type(inner, module, compressed_table, mode, out);
        }
        TypeExpr::Array { inner, .. } => {
            format_type(inner, module, compressed_table, mode, out);
            out.push_str("[]");
        }
        TypeExpr::Map { key, value, .. } => {
            out.push('{');
            format_type(key, module, compressed_table, mode, out);
            out.push(':');
            format_type(value, module, compressed_table, mode, out);
            out.push('}');
        }
        TypeExpr::Tuple { items, .. } => {
            out.push('(');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                format_type(item, module, compressed_table, mode, out);
            }
            out.push(')');
        }
        TypeExpr::Function { sig, .. } => {
            format_function_type(sig, module, compressed_table, mode, out)
        }
        TypeExpr::ResultSugar { ok, err, .. } => {
            format_type(ok, module, compressed_table, mode, out);
            out.push('!');
            format_type(err, module, compressed_table, mode, out);
        }
        TypeExpr::Group { inner, .. } => {
            out.push('(');
            format_type(inner, module, compressed_table, mode, out);
            out.push(')');
        }
    }
}

fn format_literal(lit: &Literal, out: &mut String) {
    match lit {
        Literal::Int(v, _) => out.push_str(&v.to_string()),
        Literal::Bool(v, _) => out.push_str(if *v { "t" } else { "f" }),
        Literal::String(v, _) => {
            out.push('"');
            for ch in v.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
    }
}

fn format_expr(
    expr: &Expr,
    module: &Module,
    compressed_table: Option<&Vec<String>>,
    mode: FmtMode,
    out: &mut String,
) {
    match expr {
        Expr::Block { prefix, tail, .. } => {
            out.push('{');
            for e in prefix {
                format_expr(e, module, compressed_table, mode, out);
                out.push(';');
            }
            format_expr(tail, module, compressed_table, mode, out);
            out.push('}');
        }
        Expr::Unit(_) => out.push_str("()"),
        Expr::Let {
            name,
            ty,
            value,
            body,
            ..
        } => match mode {
            FmtMode::Readable => {
                out.push_str("v(");
                out.push_str(&render_name(module, name, compressed_table, mode));
                if let Some(ty) = ty {
                    out.push(':');
                    format_type(ty, module, compressed_table, mode, out);
                }
                out.push('=');
                format_expr(value, module, compressed_table, mode, out);
                out.push(',');
                format_expr(body, module, compressed_table, mode, out);
                out.push(')');
            }
            FmtMode::Compressed => {
                out.push_str("[v ");
                out.push_str(&render_name(module, name, compressed_table, mode));
                out.push(' ');
                format_expr(value, module, compressed_table, mode, out);
                out.push(' ');
                format_expr(body, module, compressed_table, mode, out);
                out.push(']');
            }
        },
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => match mode {
            FmtMode::Readable => {
                out.push_str("i(");
                format_expr(cond, module, compressed_table, mode, out);
                out.push(',');
                format_expr(then_branch, module, compressed_table, mode, out);
                out.push(',');
                format_expr(else_branch, module, compressed_table, mode, out);
                out.push(')');
            }
            FmtMode::Compressed => {
                out.push_str("[i ");
                format_expr(cond, module, compressed_table, mode, out);
                out.push(' ');
                format_expr(then_branch, module, compressed_table, mode, out);
                out.push(' ');
                format_expr(else_branch, module, compressed_table, mode, out);
                out.push(']');
            }
        },
        Expr::Match {
            scrutinee, arms, ..
        } => match mode {
            FmtMode::Readable => {
                out.push_str("m(");
                format_expr(scrutinee, module, compressed_table, mode, out);
                out.push_str("){");
                for arm in arms {
                    format_pattern(&arm.pattern, module, compressed_table, mode, out);
                    out.push_str("=>");
                    format_expr(&arm.expr, module, compressed_table, mode, out);
                    out.push(';');
                }
                out.push('}');
            }
            FmtMode::Compressed => {
                out.push_str("[m ");
                format_expr(scrutinee, module, compressed_table, mode, out);
                for arm in arms {
                    out.push(' ');
                    format_compressed_match_arm(arm, module, compressed_table, mode, out);
                }
                out.push(']');
            }
        },
        Expr::Call { callee, args, .. } => match mode {
            FmtMode::Readable => {
                out.push_str("c(");
                format_expr(callee, module, compressed_table, mode, out);
                for arg in args {
                    out.push(',');
                    format_expr(arg, module, compressed_table, mode, out);
                }
                out.push(')');
            }
            FmtMode::Compressed => {
                out.push('(');
                format_expr(callee, module, compressed_table, mode, out);
                for arg in args {
                    out.push(' ');
                    format_expr(arg, module, compressed_table, mode, out);
                }
                out.push(')');
            }
        },
        Expr::Lambda {
            params,
            ret,
            effects,
            body,
            ..
        } => match mode {
            FmtMode::Readable => {
                out.push_str("l(");
                format_params(params, module, compressed_table, mode, out);
                out.push_str("):");
                format_type(ret, module, compressed_table, mode, out);
                format_effect_set(effects, mode, out);
                out.push('=');
                format_expr(body, module, compressed_table, mode, out);
            }
            FmtMode::Compressed => {
                out.push_str("[l (");
                format_params(params, module, compressed_table, mode, out);
                out.push_str("):");
                format_type(ret, module, compressed_table, mode, out);
                format_effect_set(effects, mode, out);
                out.push(' ');
                format_expr(body, module, compressed_table, mode, out);
                out.push(']');
            }
        },
        Expr::Assert { cond, msg, .. } => {
            out.push_str("a(");
            format_expr(cond, module, compressed_table, mode, out);
            if let Some(msg) = msg {
                out.push(',');
                format_expr(msg, module, compressed_table, mode, out);
            }
            out.push(')');
        }
        Expr::Require { expr, .. } => {
            out.push('^');
            format_expr(expr, module, compressed_table, mode, out);
        }
        Expr::Ensure { expr, .. } => {
            out.push('_');
            out.push(' ');
            format_expr(expr, module, compressed_table, mode, out);
        }
        Expr::Name(id) => out.push_str(&render_name(module, id, compressed_table, mode)),
        Expr::NameApp { name, args, .. } => {
            out.push_str(&render_name(module, name, compressed_table, mode));
            out.push('(');
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                format_expr(arg, module, compressed_table, mode, out);
            }
            out.push(')');
        }
        Expr::Literal(lit) => format_literal(lit, out),
        Expr::Paren { inner, .. } => {
            out.push('(');
            format_expr(inner, module, compressed_table, mode, out);
            out.push(')');
        }
    }
}

fn format_params(
    params: &[Param],
    module: &Module,
    compressed_table: Option<&Vec<String>>,
    mode: FmtMode,
    out: &mut String,
) {
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&render_name(module, &p.name, compressed_table, mode));
        out.push(':');
        format_type(&p.ty, module, compressed_table, mode, out);
    }
}

fn format_compressed_match_arm(
    arm: &MatchArm,
    module: &Module,
    compressed_table: Option<&Vec<String>>,
    mode: FmtMode,
    out: &mut String,
) {
    out.push('{');
    format_pattern(&arm.pattern, module, compressed_table, mode, out);
    out.push(' ');
    format_expr(&arm.expr, module, compressed_table, mode, out);
    out.push('}');
}

fn format_pattern(
    pat: &Pattern,
    module: &Module,
    compressed_table: Option<&Vec<String>>,
    mode: FmtMode,
    out: &mut String,
) {
    match pat {
        Pattern::Wildcard(_) => out.push('_'),
        Pattern::Literal(lit) => format_literal(lit, out),
        Pattern::Name(id) => out.push_str(&render_name(module, id, compressed_table, mode)),
        Pattern::Ctor { name, args, .. } => {
            out.push_str(&render_name(module, name, compressed_table, mode));
            if !args.is_empty() {
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    format_pattern(arg, module, compressed_table, mode, out);
                }
                out.push(')');
            }
        }
        Pattern::Tuple { items, .. } => {
            out.push('(');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                format_pattern(item, module, compressed_table, mode, out);
            }
            out.push(')');
        }
        Pattern::Paren { inner, .. } => {
            out.push('(');
            format_pattern(inner, module, compressed_table, mode, out);
            out.push(')');
        }
    }
}

pub fn collect_mu_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return if is_mu_file(path) {
            Ok(vec![path.to_path_buf()])
        } else {
            Err(format!("expected a .mu file: {}", path.display()))
        };
    }

    if path.is_dir() {
        let mut files = Vec::new();
        collect_mu_files_rec(path, &mut files).map_err(|e| format!("walk error: {e}"))?;
        files.sort();
        return Ok(files);
    }

    Err(format!("path does not exist: {}", path.display()))
}

fn collect_mu_files_rec(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            collect_mu_files_rec(&child, files)?;
        } else if child.is_file() && is_mu_file(&child) {
            files.push(child);
        }
    }
    Ok(())
}

fn is_mu_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("mu")
}
