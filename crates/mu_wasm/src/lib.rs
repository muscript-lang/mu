use muc::ast::{Decl, EffectAtom, EffectSet, Expr, Span};
use muc::bytecode;
use muc::fmt::{FmtMode, parse_and_format_mode};
use muc::lexer::{TokenKind, tokenize};
use muc::parser::{ParseError, parse_str};
use muc::typecheck::{TypeError, check_program};
use muc::vm::{self, VmError, VmHost};
use serde::Serialize;
use wasm_bindgen::prelude::*;

const MAX_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Serialize)]
struct ErrorItem {
    code: String,
    msg: String,
    line: usize,
    col: usize,
}

#[derive(Serialize)]
struct CheckResponse {
    ok: bool,
    errors: Vec<ErrorItem>,
}

#[derive(Serialize)]
struct StatsResponse {
    bytes: usize,
    tokens: usize,
    symtab_size: usize,
    avg_ref_width: f64,
    max_ref_width: usize,
}

#[derive(Serialize)]
struct RunResponse {
    exit_code: i32,
    stdout: String,
    stderr: String,
    fuel_used: u64,
    trapped: bool,
    trap_code: Option<String>,
}

#[wasm_bindgen]
pub fn fmt(src: String, mode: String) -> Result<String, JsValue> {
    let mode = parse_mode(&mode)?;
    parse_and_format_mode(&src, mode)
        .map_err(|err| JsValue::from_str(&format!("{}: {}", err.code.as_str(), err.message)))
}

#[wasm_bindgen]
pub fn check(src: String) -> JsValue {
    let response = check_impl(&src);
    to_js_value(&response)
}

#[wasm_bindgen]
pub fn stats(src: String, mode: String) -> JsValue {
    let response = match parse_mode(&mode) {
        Ok(mode) => stats_impl(&src, mode),
        Err(_) => StatsResponse {
            bytes: 0,
            tokens: 0,
            symtab_size: 0,
            avg_ref_width: 0.0,
            max_ref_width: 0,
        },
    };
    to_js_value(&response)
}

#[wasm_bindgen]
pub fn run(src: String, fuel: u32, stdin: Option<String>) -> JsValue {
    let response = run_impl(&src, u64::from(fuel), stdin);
    to_js_value(&response)
}

fn parse_mode(mode: &str) -> Result<FmtMode, JsValue> {
    match mode {
        "readable" => Ok(FmtMode::Readable),
        "compressed" => Ok(FmtMode::Compressed),
        _ => Err(JsValue::from_str(
            "invalid mode; expected readable or compressed",
        )),
    }
}

fn check_impl(src: &str) -> CheckResponse {
    let program = match parse_str(src) {
        Ok(program) => program,
        Err(err) => {
            return CheckResponse {
                ok: false,
                errors: vec![parse_error_item(src, &err)],
            };
        }
    };

    if let Err(err) = check_program(&program) {
        return CheckResponse {
            ok: false,
            errors: vec![type_error_item(src, &err)],
        };
    }

    let web_errors = validate_web_effects(src, &program);
    if !web_errors.is_empty() {
        return CheckResponse {
            ok: false,
            errors: web_errors,
        };
    }

    CheckResponse {
        ok: true,
        errors: Vec::new(),
    }
}

fn stats_impl(src: &str, mode: FmtMode) -> StatsResponse {
    let Ok(formatted) = parse_and_format_mode(src, mode) else {
        return StatsResponse {
            bytes: 0,
            tokens: 0,
            symtab_size: 0,
            avg_ref_width: 0.0,
            max_ref_width: 0,
        };
    };

    let tokens = token_economy_count(&formatted);
    let symtab_size = symtab_size_from_compressed(&formatted);
    let (avg_ref_width, max_ref_width) = symref_width_stats(&formatted);

    StatsResponse {
        bytes: formatted.len(),
        tokens,
        symtab_size,
        avg_ref_width,
        max_ref_width,
    }
}

fn run_impl(src: &str, fuel: u64, stdin: Option<String>) -> RunResponse {
    let program = match parse_str(src) {
        Ok(program) => program,
        Err(err) => {
            let item = parse_error_item(src, &err);
            return trapped_response(
                1,
                String::new(),
                format!(
                    "{}:{}:{}: {}: {}",
                    "<input>", item.line, item.col, item.code, item.msg
                ),
                0,
                Some(item.code),
            );
        }
    };

    if let Err(err) = check_program(&program) {
        let item = type_error_item(src, &err);
        return trapped_response(
            1,
            String::new(),
            format!(
                "{}:{}:{}: {}: {}",
                "<input>", item.line, item.col, item.code, item.msg
            ),
            0,
            Some(item.code),
        );
    }

    let web_errors = validate_web_effects(src, &program);
    if !web_errors.is_empty() {
        let first = &web_errors[0];
        return trapped_response(
            1,
            String::new(),
            format!(
                "{}:{}:{}: {}: {}",
                "<input>", first.line, first.col, first.code, first.msg
            ),
            0,
            Some(first.code.clone()),
        );
    }

    let bytecode = match bytecode::compile(&program) {
        Ok(bytes) => bytes,
        Err(err) => {
            let message = err.to_string();
            return trapped_response(
                1,
                String::new(),
                message.clone(),
                0,
                parse_vm_code(&message),
            );
        }
    };

    let mut host = WebHost::new(stdin, MAX_OUTPUT_BYTES);
    let fuel_limit = if fuel == 0 { 1 } else { fuel };
    let result = vm::run_bytecode_with_fuel_and_host(&bytecode, &[], fuel_limit, &mut host);
    let fuel_used = vm::last_fuel_used();

    match result {
        Ok(()) => RunResponse {
            exit_code: 0,
            stdout: host.stdout,
            stderr: host.stderr,
            fuel_used,
            trapped: false,
            trap_code: None,
        },
        Err(err) => {
            let mut stderr = host.stderr;
            if !stderr.is_empty() {
                stderr.push('\n');
            }
            stderr.push_str(&err.message);

            let mut trap_code = parse_vm_code(&err.message);
            if trap_code.as_deref() == Some("E4007") {
                trap_code = Some("E_FUEL".to_string());
            }

            let exit_code = parse_exit_code(&err.message).unwrap_or(1);
            trapped_response(exit_code, host.stdout, stderr, fuel_used, trap_code)
        }
    }
}

fn parse_error_item(src: &str, err: &ParseError) -> ErrorItem {
    let (line, col) = line_col(src, err.span);
    ErrorItem {
        code: err.code.as_str().to_string(),
        msg: err.message.clone(),
        line,
        col,
    }
}

fn type_error_item(src: &str, err: &TypeError) -> ErrorItem {
    let (line, col) = line_col(src, err.span);
    ErrorItem {
        code: err.code.as_str().to_string(),
        msg: err.message.clone(),
        line,
        col,
    }
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

fn validate_web_effects(src: &str, program: &muc::ast::Program) -> Vec<ErrorItem> {
    let mut errors = Vec::new();

    for decl in &program.module.decls {
        if let Decl::Import(import_decl) = decl {
            let module_name = import_decl.module.parts.join(".");
            if matches!(module_name.as_str(), "core.fs" | "core.proc" | "core.http") {
                let (line, col) = line_col(src, import_decl.span);
                errors.push(ErrorItem {
                    code: "E_WEB_EFFECT".to_string(),
                    msg: format!(
                        "import `{module_name}` is not allowed in browser mode (only !{{io}} is supported)"
                    ),
                    line,
                    col,
                });
            }
        }
    }

    for decl in &program.module.decls {
        match decl {
            Decl::Function(function_decl) => {
                collect_disallowed_effects(
                    src,
                    &function_decl.sig.effects,
                    function_decl.sig.span,
                    &mut errors,
                );
                collect_disallowed_lambda_effects(src, &function_decl.expr, &mut errors);
            }
            Decl::Value(value_decl) => {
                collect_disallowed_lambda_effects(src, &value_decl.expr, &mut errors);
            }
            _ => {}
        }
    }

    errors
}

fn collect_disallowed_effects(
    src: &str,
    effects: &EffectSet,
    span: Span,
    errors: &mut Vec<ErrorItem>,
) {
    for atom in &effects.atoms {
        if !matches!(atom, EffectAtom::Io) {
            let (line, col) = line_col(src, span);
            errors.push(ErrorItem {
                code: "E_WEB_EFFECT".to_string(),
                msg: format!(
                    "effect `{}` is not allowed in browser mode (only !{{io}} is supported)",
                    effect_name(*atom)
                ),
                line,
                col,
            });
        }
    }
}

fn collect_disallowed_lambda_effects(src: &str, expr: &Expr, errors: &mut Vec<ErrorItem>) {
    match expr {
        Expr::Block { prefix, tail, .. } => {
            for item in prefix {
                collect_disallowed_lambda_effects(src, item, errors);
            }
            collect_disallowed_lambda_effects(src, tail, errors);
        }
        Expr::Let { value, body, .. } => {
            collect_disallowed_lambda_effects(src, value, errors);
            collect_disallowed_lambda_effects(src, body, errors);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_disallowed_lambda_effects(src, cond, errors);
            collect_disallowed_lambda_effects(src, then_branch, errors);
            collect_disallowed_lambda_effects(src, else_branch, errors);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_disallowed_lambda_effects(src, scrutinee, errors);
            for arm in arms {
                collect_disallowed_lambda_effects(src, &arm.expr, errors);
            }
        }
        Expr::Call { callee, args, .. } => {
            collect_disallowed_lambda_effects(src, callee, errors);
            for arg in args {
                collect_disallowed_lambda_effects(src, arg, errors);
            }
        }
        Expr::Lambda {
            effects,
            body,
            span,
            ..
        } => {
            collect_disallowed_effects(src, effects, *span, errors);
            collect_disallowed_lambda_effects(src, body, errors);
        }
        Expr::Assert { cond, msg, .. } => {
            collect_disallowed_lambda_effects(src, cond, errors);
            if let Some(msg) = msg {
                collect_disallowed_lambda_effects(src, msg, errors);
            }
        }
        Expr::Require { expr, .. }
        | Expr::Ensure { expr, .. }
        | Expr::Paren { inner: expr, .. } => {
            collect_disallowed_lambda_effects(src, expr, errors);
        }
        Expr::NameApp { args, .. } => {
            for arg in args {
                collect_disallowed_lambda_effects(src, arg, errors);
            }
        }
        Expr::Unit(_) | Expr::Name(_) | Expr::Literal(_) => {}
    }
}

fn effect_name(effect: EffectAtom) -> &'static str {
    match effect {
        EffectAtom::Io => "io",
        EffectAtom::Fs => "fs",
        EffectAtom::Net => "net",
        EffectAtom::Proc => "proc",
        EffectAtom::Rand => "rand",
        EffectAtom::Time => "time",
        EffectAtom::St => "st",
    }
}

fn token_economy_count(src: &str) -> usize {
    match tokenize(src) {
        Ok(tokens) => tokens
            .iter()
            .map(|token| match token.kind {
                TokenKind::SymRef(_)
                | TokenKind::Ident(_)
                | TokenKind::Int(_)
                | TokenKind::String(_) => 1,
                TokenKind::Arrow
                | TokenKind::FatArrow
                | TokenKind::EqEq
                | TokenKind::NotEq
                | TokenKind::Le
                | TokenKind::Ge => 2,
                TokenKind::Eof => 0,
                _ => 1,
            })
            .sum(),
        Err(_) => 0,
    }
}

fn symtab_size_from_compressed(src: &str) -> usize {
    let Some(start) = src.find("$[") else {
        return 0;
    };
    let rest = &src[(start + 2)..];
    let Some(end) = rest.find(']') else {
        return 0;
    };
    let body = &rest[..end];
    if body.trim().is_empty() {
        0
    } else {
        body.split(',').count()
    }
}

fn symref_width_stats(src: &str) -> (f64, usize) {
    let Ok(tokens) = tokenize(src) else {
        return (0.0, 0);
    };
    let widths = tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::SymRef(idx) => Some(idx.to_string().len()),
            _ => None,
        })
        .collect::<Vec<_>>();

    let max_ref_width = widths.iter().copied().max().unwrap_or(0);
    let avg_ref_width = if widths.is_empty() {
        0.0
    } else {
        widths.iter().sum::<usize>() as f64 / widths.len() as f64
    };

    (avg_ref_width, max_ref_width)
}

fn trapped_response(
    exit_code: i32,
    stdout: String,
    stderr: String,
    fuel_used: u64,
    trap_code: Option<String>,
) -> RunResponse {
    RunResponse {
        exit_code,
        stdout,
        stderr,
        fuel_used,
        trapped: true,
        trap_code,
    }
}

fn parse_vm_code(message: &str) -> Option<String> {
    let (prefix, _) = message.split_once(':')?;
    if prefix.starts_with('E') {
        Some(prefix.trim().to_string())
    } else {
        None
    }
}

fn parse_exit_code(message: &str) -> Option<i32> {
    let marker = "program exited with status ";
    let idx = message.find(marker)?;
    message[(idx + marker.len())..].trim().parse::<i32>().ok()
}

fn to_js_value<T: Serialize>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value).unwrap_or(JsValue::NULL)
}

struct WebHost {
    stdin_lines: Vec<String>,
    stdin_index: usize,
    stdout: String,
    stderr: String,
    out_bytes: usize,
    max_output_bytes: usize,
}

impl WebHost {
    fn new(stdin: Option<String>, max_output_bytes: usize) -> Self {
        let stdin_lines = stdin
            .unwrap_or_default()
            .split('\n')
            .map(|line| line.trim_end_matches('\r').to_string())
            .collect::<Vec<_>>();

        Self {
            stdin_lines,
            stdin_index: 0,
            stdout: String::new(),
            stderr: String::new(),
            out_bytes: 0,
            max_output_bytes,
        }
    }

    fn push_stdout(&mut self, text: &str) -> Result<(), VmError> {
        let new_total = self.out_bytes.saturating_add(text.len());
        if new_total > self.max_output_bytes {
            return Err(VmError {
                message: "E_OUTPUT_LIMIT: output exceeded 64KB limit".to_string(),
            });
        }
        self.stdout.push_str(text);
        self.out_bytes = new_total;
        Ok(())
    }
}

impl VmHost for WebHost {
    fn io_print(&mut self, text: &str) -> Result<(), VmError> {
        self.push_stdout(text)
    }

    fn io_println(&mut self, text: &str) -> Result<(), VmError> {
        self.push_stdout(text)?;
        self.push_stdout("\n")
    }

    fn io_readln(&mut self) -> Result<String, VmError> {
        if self.stdin_index >= self.stdin_lines.len() {
            return Ok(String::new());
        }
        let line = self.stdin_lines[self.stdin_index].clone();
        self.stdin_index += 1;
        Ok(line)
    }

    fn fs_read_to_string(&mut self, _path: &str) -> Result<String, String> {
        Err("web sandbox: fs.read disabled".to_string())
    }

    fn fs_write_string(&mut self, _path: &str, _data: &str) -> Result<(), String> {
        Err("web sandbox: fs.write disabled".to_string())
    }

    fn proc_run(&mut self, _cmd: &str, _args: &[String]) -> Result<i32, String> {
        Err("web sandbox: proc.run disabled".to_string())
    }

    fn http_get(&mut self, _url: &str) -> Result<String, String> {
        Err("web sandbox: http.get disabled".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{symref_width_stats, token_economy_count};

    #[test]
    fn token_metric_matches_expected_weights() {
        let src = "#12 -> => == != <= >= name 42 \"x\" +";
        // #12, name, 42, "x", + each count as 1, the six two-char ops count as 2.
        assert_eq!(token_economy_count(src), 1 + 1 + 1 + 1 + 1 + (6 * 2));
    }

    #[test]
    fn symref_width_stats_handles_multiple_widths() {
        let src = "#1 #12 #123";
        let (avg, max) = symref_width_stats(src);
        assert_eq!(max, 3);
        assert!((avg - 2.0).abs() < f64::EPSILON);
    }
}
