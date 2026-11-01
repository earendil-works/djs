//! `djs repl`: an interactive REPL for debugging the engine and scripts (issue 0064,
//! docs/cli.md). Built only on the public `Vm` API.
//!
//! Every input runs in fuel slices; between slices the REPL checks for Ctrl-C, the fuel
//! limit and `.suspend-every`. With stdin not a terminal it reads lines without prompts
//! or editing (line mode, for tests and pipes).

mod input;
mod inspect;
mod term;

use std::io::{BufRead, IsTerminal, Write};
use std::time::Instant;

use djs_core::bytecode::template::disasm_template;
use djs_core::compiler::{self, CompileError, CompileErrorKind, CompileOptions};
use djs_core::heap::cells::Object;
use djs_core::heap::{CellKind, ObjectClass, Ref};
use djs_core::value::Value;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

use crate::run::{CONSOLE_LOG, now_ms};
use input::SCRIPT_COMPLETION_VALUES;
use term::{Input, Terminal};

const USAGE: &str = "usage: djs repl [--ts] [--gc-stress] [--no-history]\n\
  --ts            parse inputs as TypeScript (.ts toggles it)\n\
  --gc-stress     collect garbage at every safepoint\n\
  --no-history    do not read or write the history file ($DJS_HISTORY, ~/.djs_history)";

const HELP: &str = "\
.help                 this text
.exit                 leave (or Ctrl-D)
.load <file>          run a file as one input (a Script, or a top-level-await body)
.ts [on|off]          parse inputs as TypeScript
.disasm <code>        bytecode: of the function a code expression evaluates to, or of
                      the code itself (compiled, not run) when it is not an expression
.fuel [n|off]         fuel used by the last input; with n, a fuel limit per input
.suspend-every [n|off] run in slices of n fuel; at every stop, show where, and continue
                      in a fresh VM restored from the heap image
.time [on|off]        show the time and fuel each input takes
.gc                   collect garbage, show the arena before and after
.heap                 arena size and limit, cells per kind, objects per class
.snapshot <file>      write the heap image (after a collection)
.restore <file>       replace the VM with one restored from a heap image
.clear                start over with a fresh VM

Ctrl-C cancels the line, or stops a running input between fuel slices.
Input continues on the next line while it is incomplete (an unclosed bracket, string
template or comment). Top-level let/const/class/function persist (Script globals).";

/// Fuel per slice: how often a running input checks for Ctrl-C (a few ms).
const SLICE: u64 = 1_000_000;
const FILENAME: &str = "repl";
const HISTORY_MAX: usize = 1000;

pub fn main(args: &[String]) -> i32 {
    let mut typescript = false;
    let mut gc_stress = false;
    let mut history = true;
    for a in args {
        match a.as_str() {
            "--ts" => typescript = true,
            "--gc-stress" => gc_stress = true,
            "--no-history" => history = false,
            "-h" | "--help" => {
                println!("{USAGE}");
                return 0;
            }
            _ => {
                eprintln!("{USAGE}");
                return 2;
            }
        }
    }
    let term = if std::io::stdin().is_terminal() {
        Terminal::new()
    } else {
        None
    };
    let mut repl = Repl::new(gc_stress);
    repl.typescript = typescript;
    match term {
        Some(t) => {
            let path = history.then(history_path).flatten();
            repl.interactive(t, path)
        }
        None => repl.lines(std::io::stdin().lock()),
    }
}

fn history_path() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("DJS_HISTORY") {
        return Some(p.into());
    }
    std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".djs_history"))
}

/// What an input compiles to.
enum Plan {
    /// An expression: an async body returning `[value]`.
    Expr(Value),
    /// A Script.
    Script(Value),
    /// A top-level-`await` input: the Script that declares its names (if any), then the
    /// rewritten async body.
    Await(Option<Value>, Value),
    Incomplete,
    Error(String),
}

/// How a run ended.
enum Outcome {
    Done(Value),
    Threw(Value),
    Stopped(String),
}

struct Repl {
    vm: Vm,
    gc_stress: bool,
    typescript: bool,
    fuel_limit: Option<u64>,
    last_fuel: u64,
    suspend_every: Option<u64>,
    time: bool,
    term: Option<Terminal>,
}

impl Repl {
    fn new(gc_stress: bool) -> Repl {
        Repl {
            vm: new_vm(gc_stress),
            gc_stress,
            typescript: false,
            fuel_limit: None,
            last_fuel: 0,
            suspend_every: None,
            time: false,
            term: None,
        }
    }

    /// Line mode: no prompts, no editing.
    fn lines(&mut self, input: impl BufRead) -> i32 {
        let mut buf = String::new();
        for line in input.lines() {
            let Ok(line) = line else { break };
            match self.line(&mut buf, &line) {
                Some(code) => return code,
                None => continue,
            }
        }
        if !buf.trim().is_empty() {
            self.eval(&buf, true);
        }
        0
    }

    fn interactive(&mut self, term: Terminal, history_file: Option<std::path::PathBuf>) -> i32 {
        self.term = Some(term);
        let mut history = load_history(history_file.as_deref());
        let mut file = history_file.and_then(|p| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .ok()
        });
        println!("djs REPL. .help for commands, Ctrl-D to exit.");
        let mut buf = String::new();
        loop {
            let prompt = if buf.is_empty() { "> " } else { "... " };
            let read = self.term.as_mut().unwrap().read_line(prompt, &history);
            match read {
                Input::Eof => break,
                Input::Interrupt => {
                    if buf.is_empty() {
                        println!("(To exit, press Ctrl-D or type .exit)");
                    }
                    buf.clear();
                }
                Input::Line(line) => {
                    if !line.trim().is_empty() && history.last() != Some(&line) {
                        history.push(line.clone());
                        if let Some(f) = file.as_mut() {
                            let _ = writeln!(f, "{line}");
                        }
                    }
                    if let Some(code) = self.line(&mut buf, &line) {
                        return code;
                    }
                }
            }
        }
        0
    }

    /// Handles one line: a meta command, or part of an input. `Some(code)` exits.
    fn line(&mut self, buf: &mut String, line: &str) -> Option<i32> {
        let trimmed = line.trim();
        if buf.is_empty()
            && trimmed.starts_with('.')
            && trimmed[1..].starts_with(|c: char| c.is_ascii_alphabetic())
        {
            return self.command(trimmed);
        }
        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(line);
        if buf.trim().is_empty() || self.eval(buf, false) {
            buf.clear();
        }
        let _ = std::io::stdout().flush();
        None
    }

    fn opts<'a>(&self, goal_body: bool, filename: &'a str) -> CompileOptions<'a> {
        let o = if goal_body {
            CompileOptions::async_body(filename, &[])
        } else {
            CompileOptions::script(filename)
        };
        CompileOptions {
            typescript: self.typescript,
            ..o
        }
    }

    fn compile(&mut self, src: &str, body: bool, filename: &str) -> Result<Value, CompileError> {
        let opts = self.opts(body, filename);
        self.vm.compile(src.as_bytes(), &opts)
    }

    /// Compiles an input. `last`: no more lines come, so an incomplete input is an
    /// error. `expr`: try it as an expression first.
    fn plan(&mut self, src: &str, last: bool, expr: bool, filename: &str) -> Plan {
        let expr_first = expr && !SCRIPT_COMPLETION_VALUES;
        let try_expr = |r: &mut Repl| {
            let body = input::expression_body(src)?;
            r.compile(&body, true, filename).ok()
        };
        if expr_first && let Some(f) = try_expr(self) {
            return Plan::Expr(f);
        }
        let script_err = match self.compile(src, false, filename) {
            Ok(f) => return Plan::Script(f),
            Err(e) => e,
        };
        if expr
            && !expr_first
            && let Some(f) = try_expr(self)
        {
            return Plan::Expr(f);
        }
        let mut err = script_err.clone();
        let mut incomplete = input::is_incomplete(src, &script_err);
        if let Some((declare, body, ts)) = self.await_parts(src, filename) {
            let saved = self.typescript;
            self.typescript = ts;
            let compiled = self.compile(&body, true, filename);
            self.typescript = saved;
            match compiled {
                Ok(f) => {
                    if declare.is_empty() {
                        return Plan::Await(None, f);
                    }
                    let h = self.vm.heap_mut().new_handle(f).expect("out of memory");
                    let d = self.compile(&declare, false, filename);
                    let f = self.vm.heap().get_handle(h);
                    self.vm.heap_mut().drop_handle(h);
                    return match d {
                        Ok(d) => Plan::Await(Some(d), f),
                        Err(e) => Plan::Error(compile_error(&e, filename)),
                    };
                }
                Err(e) => {
                    incomplete |= input::is_incomplete(&body, &e);
                    err = e;
                }
            }
        }
        if incomplete && !last && err.kind == CompileErrorKind::Syntax {
            return Plan::Incomplete;
        }
        Plan::Error(compile_error(&err, filename))
    }

    /// The top-level-await rewrite of `src`: the declaring Script, the body, and whether
    /// the body is TypeScript. In TypeScript mode the types are erased first, so that
    /// annotated declarations rewrite as plain ones.
    fn await_parts(&mut self, src: &str, filename: &str) -> Option<(String, String, bool)> {
        let (js, ts) = if self.typescript {
            let opts = self.opts(true, filename);
            match compiler::strip_types(self.vm.heap_mut(), src.as_bytes(), &opts) {
                Ok(b) => (String::from_utf8(b).ok()?, false),
                // Enums and namespaces are not erasable: rewrite the TypeScript as it is.
                Err(_) => (src.to_string(), true),
            }
        } else {
            (src.to_string(), false)
        };
        let rw = input::rewrite_await(&js)?;
        Some((rw.declare, rw.body, ts))
    }

    /// Evaluates an input and prints the outcome. Returns false if it is incomplete.
    fn eval(&mut self, src: &str, last: bool) -> bool {
        self.eval_named(src, last, true, FILENAME)
    }

    fn eval_named(&mut self, src: &str, last: bool, expr: bool, filename: &str) -> bool {
        let start = Instant::now();
        let plan = self.plan(src, last, expr, filename);
        self.last_fuel = 0;
        match plan {
            Plan::Incomplete => return false,
            Plan::Error(e) => println!("{e}"),
            Plan::Expr(f) => {
                if let Some(v) = self.run_body(f) {
                    let v = match v.as_object() {
                        Some(a) => self
                            .vm
                            .heap()
                            .get_element_fast(a, 0)
                            .unwrap_or(Value::UNDEFINED),
                        None => v,
                    };
                    println!("{}", inspect::inspect(self.vm.heap(), v));
                }
            }
            Plan::Script(f) => {
                // Declarations complete with `undefined`: not worth a line each.
                if let Outcome::Done(v) = self.report(f)
                    && SCRIPT_COMPLETION_VALUES
                    && !v.is_undefined()
                {
                    println!("{}", inspect::inspect(self.vm.heap(), v));
                }
            }
            Plan::Await(declare, body) => {
                let h = self.vm.heap_mut().new_handle(body).expect("out of memory");
                let ok = match declare {
                    Some(d) => matches!(self.report(d), Outcome::Done(_)),
                    None => true,
                };
                let body = self.vm.heap().get_handle(h);
                self.vm.heap_mut().drop_handle(h);
                if ok {
                    self.run_body(body);
                }
            }
        }
        self.report_rejections();
        if self.time {
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            println!("({ms:.3} ms, {} fuel)", self.last_fuel);
        }
        true
    }

    /// Runs an async body to its promise's settlement. Returns the fulfillment value; a
    /// rejection is reported.
    fn run_body(&mut self, f: Value) -> Option<Value> {
        let Outcome::Done(p) = self.report(f) else {
            return None;
        };
        self.vm.mark_promise_handled(p);
        match self.vm.promise_state(p) {
            Some(Some(Ok(v))) => Some(v),
            Some(Some(Err(e))) => {
                self.print_uncaught(e);
                None
            }
            Some(None) => {
                println!("Uncaught: the input's promise never settles (nothing is left to run)");
                None
            }
            None => Some(p),
        }
    }

    /// Runs `f` and prints an exception or a stop.
    fn report(&mut self, f: Value) -> Outcome {
        let o = self.drive(f);
        match &o {
            Outcome::Done(_) => {}
            Outcome::Threw(e) => self.print_uncaught(*e),
            Outcome::Stopped(why) => println!("{why}"),
        }
        o
    }

    fn print_uncaught(&mut self, e: Value) {
        println!("Uncaught {}", inspect::inspect(self.vm.heap(), e));
    }

    fn report_rejections(&mut self) {
        for r in self.vm.take_unhandled_rejections() {
            println!(
                "Possibly unhandled promise rejection: {}",
                inspect::inspect(self.vm.heap(), r)
            );
        }
    }

    /// Calls `f` (with `this` = the global object) and runs it in fuel slices.
    fn drive(&mut self, f: Value) -> Outcome {
        self.vm.set_now(now_ms());
        let this = Value::object(self.vm.global());
        if self.vm.call(f, this, &[]).is_err() {
            return Outcome::Stopped("djs: the VM is busy".into());
        }
        let mut used: u64 = 0;
        let outcome = loop {
            let mut slice = self.suspend_every.unwrap_or(SLICE);
            if let Some(limit) = self.fuel_limit {
                slice = slice.min(limit.saturating_sub(used).max(1));
            }
            let (st, n) = self.vm.run_metered(slice);
            used += n;
            match st {
                Status::Done(v) => break Outcome::Done(v),
                Status::Idle => break Outcome::Done(Value::UNDEFINED),
                Status::Threw(e) => break Outcome::Threw(e),
                Status::OutOfFuel => {
                    if self.term.as_mut().is_some_and(|t| t.interrupted()) {
                        self.vm.abort();
                        break Outcome::Stopped("Interrupted".into());
                    }
                    if let Some(limit) = self.fuel_limit.filter(|&l| used >= l) {
                        self.vm.abort();
                        break Outcome::Stopped(format!(
                            "Stopped: the fuel limit ({limit}) ran out"
                        ));
                    }
                    if self.suspend_every.is_some() {
                        let bt = self.vm.backtrace();
                        let at = bt.lines().next().unwrap_or("").trim();
                        println!("[suspended, {used} fuel] {at}");
                        let opts = self.vm.options();
                        self.vm = Vm::from_bytes(self.vm.bytes().to_vec(), opts)
                            .expect("a valid heap image");
                    }
                }
                Status::HostCall { call_id, fn_id } => {
                    let info = self.vm.host_call(call_id).expect("parked call");
                    let result = if fn_id == CONSOLE_LOG {
                        let heap = self.vm.heap();
                        let parts: Vec<String> = info
                            .args
                            .iter()
                            .map(|&a| inspect::log_arg(heap, a))
                            .collect();
                        println!("{}", parts.join(" "));
                        Ok(Value::UNDEFINED)
                    } else {
                        Err(Value::UNDEFINED)
                    };
                    self.vm
                        .resolve_host_call(call_id, result)
                        .expect("parked on this call");
                }
            }
        };
        self.last_fuel += used;
        outcome
    }

    /// A meta command. `Some(code)` exits.
    fn command(&mut self, line: &str) -> Option<i32> {
        let (cmd, arg) = match line.split_once(char::is_whitespace) {
            Some((c, a)) => (c, a.trim()),
            None => (line, ""),
        };
        match cmd {
            ".help" => println!("{HELP}"),
            ".exit" => return Some(0),
            ".load" => self.load(arg),
            ".ts" => match on_off(arg, &mut self.typescript) {
                Ok(()) => println!("TypeScript: {}", if self.typescript { "on" } else { "off" }),
                Err(e) => println!("{e}"),
            },
            ".time" => match on_off(arg, &mut self.time) {
                Ok(()) => println!("timing: {}", if self.time { "on" } else { "off" }),
                Err(e) => println!("{e}"),
            },
            ".fuel" => match number_or_off(arg) {
                Ok(None) if arg.is_empty() => {
                    let limit = self
                        .fuel_limit
                        .map_or_else(|| "none".to_string(), |n| n.to_string());
                    println!("last input: {} fuel; limit: {limit}", self.last_fuel);
                }
                Ok(n) => {
                    self.fuel_limit = n;
                    match n {
                        Some(n) => println!("fuel limit: {n} per input"),
                        None => println!("fuel limit: none"),
                    }
                }
                Err(e) => println!("{e}"),
            },
            ".suspend-every" => match number_or_off(arg) {
                Ok(n) => {
                    if !arg.is_empty() {
                        self.suspend_every = n;
                    }
                    match self.suspend_every {
                        Some(n) => println!("suspending every {n} fuel"),
                        None => println!("suspending: off"),
                    }
                }
                Err(e) => println!("{e}"),
            },
            ".gc" => {
                let before = self.vm.heap().arena().len();
                match self.vm.collect() {
                    Ok(()) => {
                        let after = self.vm.heap().arena().len();
                        println!("arena: {before} -> {after} bytes");
                    }
                    Err(e) => println!("collection failed: {e:?}"),
                }
            }
            ".heap" => self.heap_stats(),
            ".snapshot" if !arg.is_empty() => {
                if let Err(e) = self.vm.collect() {
                    println!("collection failed: {e:?}");
                    return None;
                }
                match std::fs::write(arg, self.vm.bytes()) {
                    Ok(()) => println!("wrote {} bytes to {arg}", self.vm.bytes().len()),
                    Err(e) => println!("cannot write {arg}: {e}"),
                }
            }
            ".restore" if !arg.is_empty() => match std::fs::read(arg) {
                Ok(bytes) => {
                    let n = bytes.len();
                    match Vm::from_bytes(bytes, self.vm.options()) {
                        Ok(vm) => {
                            self.vm = vm;
                            println!("restored {n} bytes from {arg}");
                        }
                        Err(e) => println!("not a heap image of this build: {e:?}"),
                    }
                }
                Err(e) => println!("cannot read {arg}: {e}"),
            },
            ".clear" => {
                self.vm = new_vm(self.gc_stress);
                println!("a fresh VM");
            }
            ".disasm" if !arg.is_empty() => self.disasm(arg),
            _ => println!("unknown command or missing argument: {line} (.help lists them)"),
        }
        None
    }

    fn load(&mut self, path: &str) {
        if path.is_empty() {
            println!(".load needs a file");
            return;
        }
        match std::fs::read_to_string(path) {
            Ok(src) => {
                let saved = self.typescript;
                if path.ends_with(".ts") || path.ends_with(".mts") {
                    self.typescript = true;
                }
                self.eval_named(&src, true, false, path);
                self.typescript = saved;
            }
            Err(e) => println!("cannot read {path}: {e}"),
        }
    }

    /// `.disasm`: the function an expression evaluates to, else the compiled code.
    fn disasm(&mut self, src: &str) {
        if let Some(body) = input::expression_body(src)
            && let Ok(f) = self.compile(&body, true, FILENAME)
        {
            let Some(v) = self.run_body(f) else { return };
            let v = v
                .as_object()
                .and_then(|a| self.vm.heap().get_element_fast(a, 0))
                .unwrap_or(Value::UNDEFINED);
            if let Some(t) = self.template_of(v) {
                print!("{}", disasm_template(self.vm.heap(), t));
                return;
            }
        }
        let opts = self.opts(false, FILENAME);
        match compiler::compile(self.vm.heap_mut(), src.as_bytes(), &opts) {
            Ok(t) => print!("{}", disasm_template(self.vm.heap(), t)),
            Err(e) => {
                // A top-level-await input: its rewritten body.
                if let Some((_, body, ts)) = self.await_parts(src, FILENAME) {
                    let opts = CompileOptions {
                        typescript: ts,
                        ..self.opts(true, FILENAME)
                    };
                    if let Ok(t) = compiler::compile(self.vm.heap_mut(), body.as_bytes(), &opts) {
                        print!("{}", disasm_template(self.vm.heap(), t));
                        return;
                    }
                }
                println!("{}", compile_error(&e, FILENAME));
            }
        }
    }

    /// The template of a JS function (through bound functions).
    fn template_of(&self, v: Value) -> Option<Ref> {
        let heap = self.vm.heap();
        let mut o = v.as_object()?;
        for _ in 0..100 {
            match heap.object_class(o) {
                ObjectClass::Function => return heap.object_slot(o, 0).as_internal().map(Ref),
                ObjectClass::BoundFunction => o = heap.object_slot(o, 0).as_object()?,
                _ => return None,
            }
        }
        None
    }

    fn heap_stats(&self) {
        let heap = self.vm.heap();
        let a = heap.arena();
        println!(
            "arena: {} bytes used, {} allocated, limit {}; {} handles, {} atoms",
            a.len(),
            a.capacity(),
            a.limit(),
            heap.handle_count(),
            heap.atom_count()
        );
        let mut kinds: Vec<(CellKind, usize, u64)> = Vec::new();
        let mut classes: Vec<(ObjectClass, usize, u64)> = Vec::new();
        let mut unknown = 0usize;
        for c in heap.cells() {
            match kinds.iter_mut().find(|k| k.0 == c.kind) {
                Some(k) => {
                    k.1 += 1;
                    k.2 += u64::from(c.size);
                }
                None => kinds.push((c.kind, 1, u64::from(c.size))),
            }
            if c.kind == CellKind::Object {
                let class: u32 = a.get(c.offset, Object::CLASS);
                match ObjectClass::from_u32(class) {
                    Some(cl) => match classes.iter_mut().find(|k| k.0 == cl) {
                        Some(k) => {
                            k.1 += 1;
                            k.2 += u64::from(c.size);
                        }
                        None => classes.push((cl, 1, u64::from(c.size))),
                    },
                    None => unknown += 1,
                }
            }
        }
        kinds.sort_by_key(|k| std::cmp::Reverse(k.2));
        classes.sort_by_key(|k| std::cmp::Reverse(k.2));
        println!("{:<22} {:>8} {:>10}", "cell kind", "cells", "bytes");
        for (k, n, b) in &kinds {
            println!("{:<22} {n:>8} {b:>10}", k.layout().name);
        }
        println!("{:<22} {:>8} {:>10}", "object class", "objects", "bytes");
        for (k, n, b) in &classes {
            println!("{:<22} {n:>8} {b:>10}", format!("{k:?}"));
        }
        if unknown > 0 {
            println!("({unknown} objects of an unknown class)");
        }
    }
}

fn on_off(arg: &str, flag: &mut bool) -> Result<(), String> {
    match arg {
        "" => Ok(()),
        "on" => {
            *flag = true;
            Ok(())
        }
        "off" => {
            *flag = false;
            Ok(())
        }
        _ => Err(format!("expected on or off, not {arg}")),
    }
}

fn number_or_off(arg: &str) -> Result<Option<u64>, String> {
    match arg {
        "" | "off" => Ok(None),
        n => match n.replace('_', "").parse::<u64>() {
            Ok(n) if n > 0 => Ok(Some(n)),
            _ => Err(format!("expected a positive number or off, not {arg}")),
        },
    }
}

fn compile_error(e: &CompileError, filename: &str) -> String {
    match e.kind {
        CompileErrorKind::Syntax => {
            format!(
                "SyntaxError: {} at {filename}:{}:{}",
                e.message, e.line, e.col
            )
        }
        CompileErrorKind::OutOfMemory => "djs: out of memory while compiling".into(),
    }
}

fn load_history(path: Option<&std::path::Path>) -> Vec<String> {
    let Some(text) = path.and_then(|p| std::fs::read_to_string(p).ok()) else {
        return Vec::new();
    };
    let lines: Vec<String> = text.lines().map(String::from).collect();
    let skip = lines.len().saturating_sub(HISTORY_MAX);
    if skip > 0
        && let Some(p) = path
    {
        // Keep the file from growing without bound.
        let _ = std::fs::write(p, lines[skip..].join("\n") + "\n");
    }
    lines[skip..].to_vec()
}

/// The REPL's VM: the base snapshot, the wall clock, and `console.log` (`info`, `warn`,
/// `error`, `debug` too) as a host function printing with the REPL's printer.
fn new_vm(gc_stress: bool) -> Vm {
    let opts = VmOptions {
        exec: ExecOptions {
            gc_stress,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut vm = djs_prelude::new_vm(opts).expect("out of memory creating the VM");
    vm.set_now(now_ms());
    let console = vm.new_object().expect("oom");
    for name in ["log", "info", "warn", "error", "debug"] {
        let f = vm.new_host_function(CONSOLE_LOG, name).expect("oom");
        vm.define(console, name, f).expect("oom");
    }
    vm.define_global("console", Value::object(console))
        .expect("oom");
    vm
}
