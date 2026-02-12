use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{
    Decl, EffectAtom, EffectSet, Expr, FunctionType, Literal, Pattern, PrimType, Program, TypeExpr,
};
use crate::parser::{ParseError, parse_str};

pub fn parse_and_format(src: &str) -> Result<String, ParseError> {
    let program = parse_str(src)?;
    Ok(format_program(&program))
}

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();
    out.push('@');
    out.push_str(&program.module.mod_id.parts.join("."));
    out.push('{');
    for decl in &program.module.decls {
        format_decl(decl, &mut out);
    }
    out.push('}');
    out.push('\n');
    out
}

fn format_decl(decl: &Decl, out: &mut String) {
    match decl {
        Decl::Import(d) => {
            out.push(':');
            out.push_str(&d.alias.name);
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
                out.push_str(&name.name);
            }
            out.push(']');
            out.push(';');
        }
        Decl::Type(d) => {
            out.push_str("T ");
            out.push_str(&d.name.name);
            if !d.params.is_empty() {
                out.push('[');
                for (i, p) in d.params.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&p.name);
                }
                out.push(']');
            }
            out.push('=');
            for (i, ctor) in d.ctors.iter().enumerate() {
                if i > 0 {
                    out.push('|');
                }
                out.push_str(&ctor.name.name);
                if !ctor.fields.is_empty() {
                    out.push('(');
                    for (j, ty) in ctor.fields.iter().enumerate() {
                        if j > 0 {
                            out.push(',');
                        }
                        format_type(ty, out);
                    }
                    out.push(')');
                }
            }
            out.push(';');
        }
        Decl::Value(d) => {
            out.push_str("V ");
            out.push_str(&d.name.name);
            out.push(':');
            format_type(&d.ty, out);
            out.push('=');
            format_expr(&d.expr, out);
            out.push(';');
        }
        Decl::Function(d) => {
            out.push_str("F ");
            out.push_str(&d.name.name);
            if !d.type_params.is_empty() {
                out.push('[');
                for (i, tp) in d.type_params.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&tp.name);
                }
                out.push(']');
            }
            out.push(':');
            format_function_type(&d.sig, out);
            out.push('=');
            format_expr(&d.expr, out);
            out.push(';');
        }
    }
}

fn format_effect_set(effects: &EffectSet, out: &mut String) {
    let atoms = canonical_effect_atoms(effects);
    if atoms.is_empty() {
        return;
    }
    out.push_str("!{");
    for (i, atom) in atoms.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(match atom {
            EffectAtom::Io => "io",
            EffectAtom::Fs => "fs",
            EffectAtom::Net => "net",
            EffectAtom::Proc => "proc",
            EffectAtom::Rand => "rand",
            EffectAtom::Time => "time",
            EffectAtom::St => "st",
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

fn format_function_type(sig: &FunctionType, out: &mut String) {
    out.push('(');
    for (i, ty) in sig.params.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        format_type(ty, out);
    }
    out.push(')');
    out.push_str("->");
    format_type(&sig.ret, out);
    format_effect_set(&sig.effects, out);
}

fn format_type(ty: &TypeExpr, out: &mut String) {
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
            out.push_str(&name.name);
            if !args.is_empty() {
                out.push('[');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    format_type(arg, out);
                }
                out.push(']');
            }
        }
        TypeExpr::Optional { inner, .. } => {
            out.push('?');
            format_type(inner, out);
        }
        TypeExpr::Array { inner, .. } => {
            format_type(inner, out);
            out.push_str("[]");
        }
        TypeExpr::Map { key, value, .. } => {
            out.push('{');
            format_type(key, out);
            out.push(':');
            format_type(value, out);
            out.push('}');
        }
        TypeExpr::Tuple { items, .. } => {
            out.push('(');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                format_type(item, out);
            }
            out.push(')');
        }
        TypeExpr::Function { sig, .. } => format_function_type(sig, out),
        TypeExpr::ResultSugar { ok, err, .. } => {
            format_type(ok, out);
            out.push('!');
            format_type(err, out);
        }
        TypeExpr::Group { inner, .. } => {
            out.push('(');
            format_type(inner, out);
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

fn format_expr(expr: &Expr, out: &mut String) {
    match expr {
        Expr::Block { prefix, tail, .. } => {
            out.push('{');
            for e in prefix {
                format_expr(e, out);
                out.push(';');
            }
            format_expr(tail, out);
            out.push('}');
        }
        Expr::Unit(_) => out.push_str("()"),
        Expr::Let {
            name,
            ty,
            value,
            body,
            ..
        } => {
            out.push_str("v(");
            out.push_str(&name.name);
            if let Some(ty) = ty {
                out.push(':');
                format_type(ty, out);
            }
            out.push('=');
            format_expr(value, out);
            out.push(',');
            format_expr(body, out);
            out.push(')');
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            out.push_str("i(");
            format_expr(cond, out);
            out.push(',');
            format_expr(then_branch, out);
            out.push(',');
            format_expr(else_branch, out);
            out.push(')');
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            out.push_str("m(");
            format_expr(scrutinee, out);
            out.push_str("){");
            for arm in arms {
                format_pattern(&arm.pattern, out);
                out.push_str("=>");
                format_expr(&arm.expr, out);
                out.push(';');
            }
            out.push('}');
        }
        Expr::Call { callee, args, .. } => {
            out.push_str("c(");
            format_expr(callee, out);
            for arg in args {
                out.push(',');
                format_expr(arg, out);
            }
            out.push(')');
        }
        Expr::Lambda {
            params,
            ret,
            effects,
            body,
            ..
        } => {
            out.push_str("l(");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&p.name.name);
                out.push(':');
                format_type(&p.ty, out);
            }
            out.push_str("):");
            format_type(ret, out);
            format_effect_set(effects, out);
            out.push('=');
            format_expr(body, out);
        }
        Expr::Assert { cond, msg, .. } => {
            out.push_str("a(");
            format_expr(cond, out);
            if let Some(msg) = msg {
                out.push(',');
                format_expr(msg, out);
            }
            out.push(')');
        }
        Expr::Require { expr, .. } => {
            out.push('^');
            format_expr(expr, out);
        }
        Expr::Ensure { expr, .. } => {
            out.push('_');
            out.push(' ');
            format_expr(expr, out);
        }
        Expr::Name(id) => out.push_str(&id.name),
        Expr::NameApp { name, args, .. } => {
            out.push_str(&name.name);
            out.push('(');
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                format_expr(arg, out);
            }
            out.push(')');
        }
        Expr::Literal(lit) => format_literal(lit, out),
        Expr::Paren { inner, .. } => {
            out.push('(');
            format_expr(inner, out);
            out.push(')');
        }
    }
}

fn format_pattern(pat: &Pattern, out: &mut String) {
    match pat {
        Pattern::Wildcard(_) => out.push('_'),
        Pattern::Literal(lit) => format_literal(lit, out),
        Pattern::Name(id) => out.push_str(&id.name),
        Pattern::Ctor { name, args, .. } => {
            out.push_str(&name.name);
            if !args.is_empty() {
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    format_pattern(arg, out);
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
                format_pattern(item, out);
            }
            out.push(')');
        }
        Pattern::Paren { inner, .. } => {
            out.push('(');
            format_pattern(inner, out);
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
