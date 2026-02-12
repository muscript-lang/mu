use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Span;
use crate::bytecode;
use crate::fmt::{collect_mu_files, parse_and_format};
use crate::parser::{ParseError, parse_str};
use crate::typecheck::{TypeError, check_program, validate_modules};
use crate::vm::run_bytecode;

const HELP: &str = "muc - muScript compiler toolchain (v0.1 scaffold)\n\nUSAGE:\n  muc fmt <file|dir> [--check]\n  muc check <file|dir>\n  muc run <file.mu> [-- args...]\n  muc build <file.mu> -o <out.mub>\n";

pub fn run() -> Result<(), String> {
    let mut args: Vec<String> = env::args().collect();
    if args.len() <= 1 || args[1] == "--help" || args[1] == "-h" {
        print!("{HELP}");
        return Ok(());
    }

    if args[1] == "--version" || args[1] == "-V" {
        println!("muc {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let command = args.remove(1);
    let command_args = &args[1..];

    match command.as_str() {
        "fmt" => parse_fmt(command_args).and_then(|(path, check)| cmd_fmt(&path, check)),
        "check" => parse_check(command_args).and_then(|path| cmd_check(&path)),
        "run" => parse_run(command_args).and_then(|(file, rest)| cmd_run(&file, &rest)),
        "build" => parse_build(command_args).and_then(|(file, out)| cmd_build(&file, &out)),
        other => Err(format!("unknown command `{other}`\n\n{HELP}")),
    }
}

fn parse_fmt(args: &[String]) -> Result<(PathBuf, bool), String> {
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("usage: muc fmt <file|dir> [--check]".to_string());
    }
    let mut path: Option<PathBuf> = None;
    let mut check = false;
    for arg in args {
        if arg == "--check" {
            check = true;
        } else if path.is_none() {
            path = Some(PathBuf::from(arg));
        } else {
            return Err(format!("unknown argument for fmt: `{arg}`"));
        }
    }
    let path = path.ok_or_else(|| "usage: muc fmt <file|dir> [--check]".to_string())?;
    Ok((path, check))
}

fn parse_check(args: &[String]) -> Result<PathBuf, String> {
    if args.len() != 1 || args[0] == "--help" || args[0] == "-h" {
        return Err("usage: muc check <file|dir>".to_string());
    }
    Ok(PathBuf::from(&args[0]))
}

fn parse_run(args: &[String]) -> Result<(PathBuf, Vec<String>), String> {
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        return Err("usage: muc run <file.mu> [-- args...]".to_string());
    }
    let file = PathBuf::from(&args[0]);
    if let Some((idx, _)) = args.iter().enumerate().find(|(_, a)| *a == "--") {
        return Ok((file, args[(idx + 1)..].to_vec()));
    }
    Ok((file, Vec::new()))
}

fn parse_build(args: &[String]) -> Result<(PathBuf, PathBuf), String> {
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        return Err("usage: muc build <file.mu> -o <out.mub>".to_string());
    }
    if args.len() != 3 || args[1] != "-o" {
        return Err("usage: muc build <file.mu> -o <out.mub>".to_string());
    }
    Ok((PathBuf::from(&args[0]), PathBuf::from(&args[2])))
}

fn cmd_fmt(path: &Path, check: bool) -> Result<(), String> {
    let files = collect_mu_files(path)?;
    if files.is_empty() {
        return Err(format!("no .mu files found under {}", path.display()));
    }

    let mut changed = Vec::new();

    for file in files {
        let src = fs::read_to_string(&file)
            .map_err(|e| format!("failed reading {}: {e}", file.display()))?;
        let formatted =
            parse_and_format(&src).map_err(|e| format_parse_error(&file, &src, &e))?;

        if src != formatted {
            if check {
                changed.push(file);
            } else {
                fs::write(&file, formatted)
                    .map_err(|e| format!("failed writing {}: {e}", file.display()))?;
                println!("formatted {}", file.display());
            }
        }
    }

    if check && !changed.is_empty() {
        for file in changed {
            eprintln!("would reformat {}", file.display());
        }
        return Err("format check failed".to_string());
    }

    Ok(())
}

fn cmd_check(path: &Path) -> Result<(), String> {
    let files = collect_mu_files(path)?;
    if files.is_empty() {
        return Err(format!("no .mu files found under {}", path.display()));
    }

    let mut sources = Vec::new();
    let mut programs = Vec::new();
    for file in files {
        let src = fs::read_to_string(&file)
            .map_err(|e| format!("failed reading {}: {e}", file.display()))?;
        let program = parse_str(&src).map_err(|e| format_parse_error(&file, &src, &e))?;
        sources.push((file, src));
        programs.push(program);
    }

    validate_modules(&programs).map_err(|e| format!("check failed: {e}"))?;
    for ((file, src), program) in sources.iter().zip(programs.iter()) {
        check_program(program).map_err(|e| format_type_error(file, src, &e))?;
    }

    println!("check ok");
    Ok(())
}

fn cmd_run(file: &PathBuf, args: &[String]) -> Result<(), String> {
    let src =
        fs::read_to_string(file).map_err(|e| format!("failed reading {}: {e}", file.display()))?;
    let program = parse_str(&src).map_err(|e| format_parse_error(file, &src, &e))?;
    check_program(&program).map_err(|e| format_type_error(file, &src, &e))?;
    let bytecode = bytecode::compile(&program).map_err(|e| format!("{}: {}", file.display(), e))?;
    run_bytecode(&bytecode, args).map_err(|e| e.to_string())
}

fn cmd_build(file: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let src =
        fs::read_to_string(file).map_err(|e| format!("failed reading {}: {e}", file.display()))?;
    let program = parse_str(&src).map_err(|e| format_parse_error(file, &src, &e))?;
    check_program(&program).map_err(|e| format_type_error(file, &src, &e))?;
    let bytecode = bytecode::compile(&program).map_err(|e| format!("{}: {}", file.display(), e))?;
    fs::write(output, bytecode).map_err(|e| format!("failed writing {}: {e}", output.display()))?;
    println!("built {}", output.display());
    Ok(())
}

fn format_parse_error(path: &Path, src: &str, err: &ParseError) -> String {
    let (line, col) = line_col(src, err.span);
    format!(
        "{}:{}:{}: {}: {}",
        path.display(),
        line,
        col,
        err.code.as_str(),
        err.message
    )
}

fn format_type_error(path: &Path, src: &str, err: &TypeError) -> String {
    let (line, col) = line_col(src, err.span);
    format!(
        "{}:{}:{}: {}: {}",
        path.display(),
        line,
        col,
        err.code.as_str(),
        err.message
    )
}

fn line_col(src: &str, span: Span) -> (usize, usize) {
    let target = span.start.min(src.len());
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, ch) in src.char_indices() {
        if idx >= target {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
