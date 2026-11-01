//! djs-bgen: the build-time compiler for the builtin language (ARCH.md "Builtins").
//! v0: subset checker, resume-point analysis and the JS (bytecode backend) emitter.

pub mod analysis;
pub mod ast;
pub mod check;
pub mod diag;
pub mod emit_js;
pub mod emit_rust;
pub mod lexer;
pub mod parser;

use std::fs;
use std::path::Path;

use diag::Sources;

/// The generated files, relative to the builtins directory.
pub struct Output {
    /// `gen/prelude.js`
    pub prelude: String,
    /// `gen/resume-points.txt`
    pub report: String,
    /// `crates/djs-core/src/builtins/gen/natives.rs`: the Rust backend (0063).
    pub natives: String,
}

pub const PRELUDE_PATH: &str = "gen/prelude.js";
pub const REPORT_PATH: &str = "gen/resume-points.txt";
/// Relative to the builtins directory.
pub const NATIVES_PATH: &str = "../crates/djs-core/src/builtins/gen/natives.rs";

/// Compiles in-memory sources: `intrinsics` is intrinsics.d.ts, `modules` are
/// (name, text) pairs of builtin files in output order. Errors are rendered.
pub fn compile(intrinsics: (&str, &str), modules: &[(String, String)]) -> Result<Output, String> {
    let mut sources = Sources::default();
    let decl_file = sources.add(intrinsics.0, intrinsics.1);
    let mut errors = Vec::new();
    let decls = match parser::parse_declarations(decl_file, intrinsics.1) {
        Ok(d) => Some(d),
        Err(e) => {
            errors.push(e);
            None
        }
    };
    let mut parsed = Vec::new();
    for (name, text) in modules {
        let file = sources.add(name.clone(), text.clone());
        match parser::parse_module(file, text) {
            Ok(m) => parsed.push(m),
            Err(e) => errors.push(e),
        }
    }
    let render = |errors: &[diag::Diagnostic], sources: &Sources| {
        let mut s: String = errors.iter().map(|e| sources.render(e)).collect();
        s.push_str(&format!(
            "{} error{}\n",
            errors.len(),
            if errors.len() == 1 { "" } else { "s" }
        ));
        s
    };
    if !errors.is_empty() {
        return Err(render(&errors, &sources));
    }
    let program =
        check::check(decls.unwrap(), parsed).map_err(|errors| render(&errors, &sources))?;
    let analysis = analysis::analyze(&program);
    let natives = emit_rust::emit(&program, &analysis, &sources)
        .map_err(|errors| render(&errors, &sources))?;
    Ok(Output {
        prelude: emit_js::emit(&program, &sources),
        report: analysis::report(&program, &analysis, &sources),
        natives,
    })
}

/// Compiles the builtins directory: `intrinsics.d.ts` and every other `*.ts`, sorted
/// by file name. Names in messages are relative to the directory's parent.
pub fn compile_dir(dir: &Path) -> Result<Output, String> {
    let prefix = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let read = |p: &Path| fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let intrinsics = read(&dir.join("intrinsics.d.ts"))?;
    let mut names: Vec<String> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".ts") && !n.ends_with(".d.ts"))
        .collect();
    names.sort();
    let mut modules = Vec::new();
    for n in names {
        modules.push((format!("{prefix}/{n}"), read(&dir.join(&n))?));
    }
    compile(
        (&format!("{prefix}/intrinsics.d.ts"), &intrinsics),
        &modules,
    )
}
