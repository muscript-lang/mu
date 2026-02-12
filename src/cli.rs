use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Span;
use crate::ast::{Decl, Program};
use crate::bytecode;
use crate::fmt::{collect_mu_files, parse_and_format};
use crate::parser::{ParseError, parse_str};
use crate::typecheck::{TypeError, check_program_with_modules, validate_modules};
use crate::vm::run_bytecode;

const HELP: &str = "muc - muScript compiler toolchain (v0.1)\n\nUSAGE:\n  muc fmt <file|dir> [--check]\n  muc check <file|dir>\n  muc run <file.mu|file.mub> [-- args...]\n  muc build <file.mu> -o <out.mub>\n";

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
        return Err("usage: muc run <file.mu|file.mub> [-- args...]".to_string());
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
        let formatted = parse_and_format(&src).map_err(|e| format_parse_error(&file, &src, &e))?;

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
    let loaded = if path.is_file() {
        load_entry_workspace(path)?
    } else {
        let files = collect_mu_files(path)?;
        if files.is_empty() {
            return Err(format!("no .mu files found under {}", path.display()));
        }
        load_programs(files)?
    };
    check_loaded_modules(&loaded)?;

    println!("check ok");
    Ok(())
}

fn cmd_run(file: &PathBuf, args: &[String]) -> Result<(), String> {
    let is_mub = file.extension().and_then(|s| s.to_str()) == Some("mub");
    if is_mub {
        let bytes =
            fs::read(file).map_err(|e| format!("failed reading {}: {e}", file.display()))?;
        return run_bytecode(&bytes, args).map_err(|e| e.to_string());
    }

    let loaded = load_entry_workspace(file)?;
    check_loaded_modules(&loaded)?;
    let program = entry_program(&loaded, file)?;
    let bytecode = bytecode::compile(&program).map_err(|e| format!("{}: {}", file.display(), e))?;
    run_bytecode(&bytecode, args).map_err(|e| e.to_string())
}

fn cmd_build(file: &Path, output: &Path) -> Result<(), String> {
    let loaded = load_entry_workspace(file)?;
    check_loaded_modules(&loaded)?;
    let program = entry_program(&loaded, file)?;
    let bytecode = bytecode::compile(&program).map_err(|e| format!("{}: {}", file.display(), e))?;
    fs::write(output, bytecode).map_err(|e| format!("failed writing {}: {e}", output.display()))?;
    println!("built {}", output.display());
    Ok(())
}

fn load_entry_workspace(entry_file: &Path) -> Result<Vec<(PathBuf, String, Program)>, String> {
    let entry_src = fs::read_to_string(entry_file)
        .map_err(|e| format!("failed reading {}: {e}", entry_file.display()))?;
    let entry_program =
        parse_str(&entry_src).map_err(|e| format_parse_error(entry_file, &entry_src, &e))?;
    let mut loaded = vec![(entry_file.to_path_buf(), entry_src, entry_program)];

    let root = entry_file.parent().unwrap_or_else(|| Path::new("."));
    let candidate_files = collect_local_mu_files(root)
        .into_iter()
        .filter(|path| !same_path(path, entry_file))
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for file in candidate_files {
        let Ok(src) = fs::read_to_string(&file) else {
            continue;
        };
        let Ok(program) = parse_str(&src) else {
            continue;
        };
        let module_name = module_name_of(&program);
        candidates.push((module_name, file, src, program));
    }

    loop {
        let needed = unresolved_imports(&loaded);
        if needed.is_empty() {
            break;
        }
        let mut progress = false;
        let mut i = 0;
        while i < candidates.len() {
            if needed.contains(&candidates[i].0) {
                let (_, path, src, program) = candidates.swap_remove(i);
                loaded.push((path, src, program));
                progress = true;
            } else {
                i += 1;
            }
        }
        if !progress {
            break;
        }
    }

    Ok(loaded)
}

fn load_programs(files: Vec<PathBuf>) -> Result<Vec<(PathBuf, String, Program)>, String> {
    let mut loaded = Vec::new();
    for file in files {
        let src = fs::read_to_string(&file)
            .map_err(|e| format!("failed reading {}: {e}", file.display()))?;
        let program = parse_str(&src).map_err(|e| format_parse_error(&file, &src, &e))?;
        loaded.push((file, src, program));
    }
    Ok(loaded)
}

fn collect_local_mu_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) == Some("mu") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn check_loaded_modules(loaded: &[(PathBuf, String, Program)]) -> Result<(), String> {
    let programs = loaded
        .iter()
        .map(|(_, _, program)| program.clone())
        .collect::<Vec<_>>();
    validate_modules(&programs).map_err(|e| format!("check failed: {e}"))?;
    for (file, src, program) in loaded {
        check_program_with_modules(program, &programs)
            .map_err(|e| format_type_error(file, src, &e))?;
    }
    Ok(())
}

fn entry_program(
    loaded: &[(PathBuf, String, Program)],
    entry_file: &Path,
) -> Result<Program, String> {
    loaded
        .iter()
        .find(|(path, _, _)| same_path(path, entry_file))
        .map(|(_, _, program)| program.clone())
        .ok_or_else(|| format!("entry module {} was not loaded", entry_file.display()))
}

fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(ac), Ok(bc)) => ac == bc,
        _ => false,
    }
}

fn unresolved_imports(loaded: &[(PathBuf, String, Program)]) -> std::collections::BTreeSet<String> {
    let mut known = std::collections::BTreeSet::new();
    for (_, _, program) in loaded {
        known.insert(module_name_of(program));
    }
    for module in builtin_module_names() {
        known.insert(module.to_string());
    }

    let mut unresolved = std::collections::BTreeSet::new();
    for (_, _, program) in loaded {
        for decl in &program.module.decls {
            if let Decl::Import(import_decl) = decl {
                let mod_name = import_decl.module.parts.join(".");
                if !known.contains(&mod_name) {
                    unresolved.insert(mod_name);
                }
            }
        }
    }
    unresolved
}

fn module_name_of(program: &Program) -> String {
    program.module.mod_id.parts.join(".")
}

fn builtin_module_names() -> [&'static str; 6] {
    [
        "core.prelude",
        "core.io",
        "core.fs",
        "core.json",
        "core.proc",
        "core.http",
    ]
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
