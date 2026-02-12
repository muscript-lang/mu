use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{BinaryOp, Expr, Item, Program, Stmt};
use crate::parser::{ParseError, parse_str};

pub fn parse_and_format(src: &str) -> Result<String, ParseError> {
    let program = parse_str(src)?;
    Ok(format_program(&program))
}

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();
    for item in &program.items {
        match item {
            Item::Stmt(stmt) => {
                format_stmt(stmt, &mut out);
                out.push('\n');
            }
        }
    }
    out
}

fn format_stmt(stmt: &Stmt, out: &mut String) {
    match stmt {
        Stmt::Let { name, value, .. } => {
            out.push_str("let ");
            out.push_str(&name.name);
            out.push_str(" = ");
            format_expr(value, out, 0);
            out.push(';');
        }
        Stmt::Expr { expr, .. } => {
            format_expr(expr, out, 0);
            out.push(';');
        }
    }
}

fn format_expr(expr: &Expr, out: &mut String, parent_prec: u8) {
    match expr {
        Expr::Ident(id) => out.push_str(&id.name),
        Expr::Int(v, _) => out.push_str(&v.to_string()),
        Expr::String(v, _) => {
            out.push('"');
            for ch in v.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    other => out.push(other),
                }
            }
            out.push('"');
        }
        Expr::Bool(v, _) => out.push_str(if *v { "true" } else { "false" }),
        Expr::List(values, _) => {
            out.push('[');
            for (idx, value) in values.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                format_expr(value, out, 0);
            }
            out.push(']');
        }
        Expr::Call { callee, args, .. } => {
            format_expr(callee, out, 9);
            out.push('(');
            for (idx, arg) in args.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                format_expr(arg, out, 0);
            }
            out.push(')');
        }
        Expr::Binary { lhs, op, rhs, .. } => {
            let prec = precedence(*op);
            let needs_paren = prec < parent_prec;
            if needs_paren {
                out.push('(');
            }
            format_expr(lhs, out, prec);
            out.push(' ');
            out.push_str(op.as_str());
            out.push(' ');
            format_expr(rhs, out, prec + 1);
            if needs_paren {
                out.push(')');
            }
        }
    }
}

fn precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::EqEq => 1,
        BinaryOp::Add | BinaryOp::Sub => 2,
        BinaryOp::Mul | BinaryOp::Div => 3,
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
