//! `djs repl`: an interactive REPL for debugging the engine and scripts (issue 0064,
//! docs/cli.md). Built only on the public `Vm` API.
//!
//! Every input runs in fuel slices; between slices the REPL checks for Ctrl-C, the fuel
//! limit and `.suspend-every`. With stdin not a terminal it reads lines without prompts
//! or editing (line mode, for tests and pipes).

pub mod activations;
mod complete;
mod input;
mod inspect;
mod term;

use std::collections::HashMap;
use std::io::{BufRead, IsTerminal, Write};
use std::time::Instant;

use djs_core::bytecode::template::disasm_template;
use djs_core::compiler::{self, CompileError, CompileErrorKind, CompileOptions};
use djs_core::heap::{Handle, ObjectClass, Ref};
use djs_core::value::Value;
use djs_core::vm::{ErrorKind, ExecOptions, Status, Vm, VmOptions};

use crate::run::{CONSOLE_LOG, now_ms};
use crate::tools::{self, MAKE_TOOL, Tools};
use input::SCRIPT_COMPLETION_VALUES;
use term::{Input, Terminal};

const USAGE: &str = "usage: djs repl [--ts] [--gc-stress] [--no-history] [--frozen]\n\
                [--tools answers.json]\n\
  --ts            parse inputs as TypeScript (.ts toggles it)\n\
  --tools FILE    answer tools.x(...) calls from FILE (docs/cli.md); others wait for .resolve\n\
  --gc-stress     collect garbage at every safepoint\n\
  --no-history    do not read or write the history file ($DJS_HISTORY, ~/.djs_history)\n\
  --frozen        the VMs have frozen builtins, which .dump and .dump-every need (D2836);
                  `debugger;` pauses (.help)";

const HELP: &str = "\
.help                 this text
.exit                 leave (or Ctrl-D)
.load <file>          run a file as one input (a Script, or a top-level-await body)
.ts [on|off]          parse inputs as TypeScript
.decorators [tc39|legacy|metadata]  TypeScript decorators: the standard ones, or
                      experimentalDecorators (with emitDecoratorMetadata)
.disasm <code>        bytecode: of the function a code expression evaluates to, or of
                      the code itself (compiled, not run) when it is not an expression
.fuel [n|off]         fuel used by the last input; with n, a fuel limit per input
.suspend-every [n|off] run in slices of n fuel; at every stop, show where, and continue
                      in a fresh VM restored from the heap image
.step [n|off]         pause every n fuel (default 1000) with a step> prompt: Enter
                      runs the next slice, c continues to the end, bt shows the
                      frames, fuel the fuel so far, q stops the input
.complete <text>      the Tab completions for text (globals and properties)
.time [on|off]        show the time and fuel each input takes
.gc                   collect garbage, show the arena before and after
.heap [sections]      what is in the heap: this session's cells apart from the base
                      realm every VM starts with (sections summary, kinds, diff, prefix,
                      largest, strings, shapes, functions, session; default
                      summary,diff; `all` for every section)
.dump <file> [names]  write the session's globals (or the named ones) as JavaScript,
                      with what is in flight (open calls, waiting inputs): a Script
                      that recreates them, functions as their source, closures and
                      data as code, builtins by name (`-` prints it; needs --frozen)
.restore <file>       replace the VM with a fresh one that ran a .dump file
.clear                start over with a fresh VM

tools.<name>(args) is an async host call: an input that waits on one returns to the
prompt and prints its value when it settles.
.pending              the open tool calls
.resolve <id> <json>  answer call #id with a JSON value
.reject <id> <msg>    answer call #id with an Error
.bt <id|all>          the suspended activations waiting on call #id (or all of
                      them), where they stopped and their bindings
.restore <dump.js>    also restores a dump's waiting run and open calls
                      (`later(v)` calls are answered with v at once; print() prints)
.dump-every [on|off]  dump and reload at every idle point ([dumped N bytes]; needs
                      --frozen)

With --frozen, `debugger;` in an async function or an input pauses it ([paused] #id):
.continue, .c [id]    continue the pause (the oldest, or #id) in place
.abort                drop the paused input and everything else in flight (Ctrl-C
                      at an empty prompt does too)
.dump then .restore of the file continue from the dump: the pause is open again.

Ctrl-C cancels the line, or stops a running input between fuel slices.
Input continues on the next line while it is incomplete (an unclosed bracket, string
template or comment). Top-level let/const/class/function persist (Script globals).";

/// Fuel per slice: how often a running input checks for Ctrl-C (a few ms).
const SLICE: u64 = 1_000_000;
const FILENAME: &str = "repl";
/// The running input's result, as a handle of `.dump-every`'s dumps.
const ENTRY: &str = "$entry";
const HISTORY_MAX: usize = 1000;
/// `.dump` and `.dump-every` in a REPL whose VMs have no frozen builtins.
const NOT_FROZEN: &str = "dumps need frozen builtins (D2836): start the REPL with --frozen";

pub fn main(args: &[String]) -> i32 {
    let mut typescript = false;
    let mut gc_stress = false;
    let mut history = true;
    let mut frozen = false;
    let mut answers = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--tools" => match it.next().map(|p| tools::read_answers(p)) {
                Some(Ok(a)) => answers = Some(a),
                Some(Err(e)) => {
                    eprintln!("djs: {e}");
                    return 2;
                }
                None => {
                    eprintln!("{USAGE}");
                    return 2;
                }
            },
            "--ts" => typescript = true,
            "--gc-stress" => gc_stress = true,
            "--no-history" => history = false,
            "--frozen" => frozen = true,
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
    let mut repl = Repl::new(gc_stress, frozen);
    repl.typescript = typescript;
    repl.tools = Tools::with_answers(answers);
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

/// What the `step>` prompt decided.
enum Step {
    Next,
    Continue,
    Quit,
}

/// An input whose promise is still pending: it waits on tool calls.
struct Waiting {
    /// `input 3`, or the name of a loaded dump's handle.
    label: String,
    promise: Handle,
    /// Print its value when it settles.
    show: bool,
    /// The value is an expression input's `[value]`.
    unwrap: bool,
}

impl Waiting {
    /// Its name as a dump's handle: the label, and for an input whose value prints when
    /// it settles (an expression input, `[value]`) ` (value)` after it (D2604), so a
    /// restore prints it the same way. Other handles (a dump's `run`) print their value.
    fn handle_name(&self) -> String {
        if self.label.starts_with("input ") && self.show {
            format!("{} (value)", self.label)
        } else {
            self.label.clone()
        }
    }

    fn from_handle_name(name: String, promise: Handle) -> Waiting {
        if let Some(label) = name.strip_suffix(" (value)")
            && label.starts_with("input ")
        {
            return Waiting {
                label: label.to_string(),
                promise,
                show: true,
                unwrap: true,
            };
        }
        let input = name.starts_with("input ");
        Waiting {
            label: name,
            promise,
            show: !input,
            unwrap: false,
        }
    }
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
    /// `.decorators`: TypeScript's legacy decorators, and their metadata.
    legacy_decorators: bool,
    decorator_metadata: bool,
    /// `.step`: the slice after which a running input pauses at a `step>` prompt.
    step: Option<u64>,
    /// Line mode: the lines still to read (the `step>` prompt reads from them too).
    source: Option<Box<dyn Iterator<Item = String>>>,
    tools: Tools,
    /// Inputs evaluated so far (numbers the inputs).
    inputs: u32,
    waiting: Vec<Waiting>,
    /// What issued each open call: `input 3`, `.resolve 1`.
    origins: HashMap<u32, String>,
    /// The origin of calls made now.
    current: String,
    /// Calls made during the current run, reported as waiting when it stops.
    fresh: Vec<u32>,
    /// The last run announced a waiting call.
    announced: bool,
    /// `.dump-every`.
    dump_every: bool,
    /// `--frozen`: the VMs have frozen builtins, which dumps need (D2836).
    frozen: bool,
    /// The running input's result after `.dump-every` reloaded the VM.
    entry_override: Option<Handle>,
    /// A handle outside the waiting inputs is held: `.dump-every` must not replace the VM.
    hold_vm: bool,
    /// The dump file the VM was `.restore`d from (its functions' positions are in it).
    dump_file: Option<String>,
    /// A `debugger;` paused before: the hint on how to go on was shown.
    paused_before: bool,
}

impl Repl {
    fn new(gc_stress: bool, frozen: bool) -> Repl {
        Repl {
            vm: new_vm(gc_stress, frozen),
            frozen,
            gc_stress,
            typescript: false,
            fuel_limit: None,
            last_fuel: 0,
            suspend_every: None,
            time: false,
            term: None,
            legacy_decorators: false,
            decorator_metadata: false,
            step: None,
            source: None,
            tools: Tools::default(),
            inputs: 0,
            waiting: Vec::new(),
            origins: HashMap::new(),
            current: String::new(),
            fresh: Vec::new(),
            announced: false,
            dump_every: false,
            entry_override: None,
            hold_vm: false,
            dump_file: None,
            paused_before: false,
        }
    }

    /// Replaces the VM; waiting inputs belong to the old one.
    fn replace_vm(&mut self, vm: Vm) {
        self.vm = vm;
        self.entry_override = None;
        self.dump_file = None;
        self.waiting.clear();
        self.origins.clear();
        self.fresh.clear();
        self.tools.rescan(&mut self.vm);
    }

    /// Line mode: no prompts, no editing.
    fn lines(&mut self, input: impl BufRead + 'static) -> i32 {
        let mut buf = String::new();
        self.source = Some(Box::new(input.lines().map_while(Result::ok)));
        while let Some(line) = self.source.as_mut().and_then(|s| s.next()) {
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
            let Repl { term, vm, .. } = self;
            let read = term
                .as_mut()
                .unwrap()
                .read_line(prompt, &history, &mut |before| {
                    complete::complete(vm, before)
                });
            match read {
                Input::Eof => break,
                Input::Interrupt => {
                    if buf.is_empty() && !self.pauses().is_empty() {
                        self.abort_pause();
                    } else if buf.is_empty() {
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
            experimental_decorators: self.legacy_decorators,
            emit_decorator_metadata: self.decorator_metadata,
            // The dumping mode: `debugger;` pauses (D3060).
            debugger_awaits: self.frozen,
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
        let expr_first = expr && (!SCRIPT_COMPLETION_VALUES || input::starts_with_brace(src));
        let try_expr = |r: &mut Repl| {
            let body = input::expression_body(src)?;
            r.compile(&body, true, filename).ok()
        };
        if expr_first && let Some(f) = try_expr(self) {
            return Plan::Expr(f);
        }
        // A `debugger;` pauses in an async body only: an input with one runs as a
        // top-level-await input (D3060).
        if self.frozen
            && input::has_top_level_debugger(src)
            && let Some(Ok(p)) = self.plan_await(src, filename)
        {
            return p;
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
        match self.plan_await(src, filename) {
            Some(Ok(p)) => return p,
            Some(Err((e, body))) => {
                incomplete |= input::is_incomplete(&body, &e);
                err = e;
            }
            None => {}
        }
        if incomplete && !last && err.kind == CompileErrorKind::Syntax {
            return Plan::Incomplete;
        }
        Plan::Error(compile_error(&err, filename))
    }

    /// `src` as a top-level-await input: `None` if it does not rewrite, else its plan or
    /// the body's compile error and the body.
    fn plan_await(
        &mut self,
        src: &str,
        filename: &str,
    ) -> Option<Result<Plan, (CompileError, String)>> {
        let (declare, body, ts) = self.await_parts(src, filename)?;
        let saved = self.typescript;
        self.typescript = ts;
        let compiled = self.compile(&body, true, filename);
        self.typescript = saved;
        let f = match compiled {
            Ok(f) => f,
            Err(e) => return Some(Err((e, body))),
        };
        if declare.is_empty() {
            return Some(Ok(Plan::Await(None, f)));
        }
        let h = self.vm.heap_mut().new_handle(f).expect("out of memory");
        let d = self.compile(&declare, false, filename);
        let f = self.vm.heap().get_handle(h);
        self.vm.heap_mut().drop_handle(h);
        Some(Ok(match d {
            Ok(d) => Plan::Await(Some(d), f),
            Err(e) => Plan::Error(compile_error(&e, filename)),
        }))
    }

    /// The top-level-await rewrite of `src`: the declaring Script, the body, and whether
    /// the body is TypeScript. In TypeScript mode the types are erased first, so that
    /// annotated declarations rewrite as plain ones.
    fn await_parts(&mut self, src: &str, filename: &str) -> Option<(String, String, bool)> {
        let (js, ts) = if self.typescript {
            let opts = self.opts(true, filename);
            let js = compiler::strip_types(self.vm.heap_mut(), src.as_bytes(), &opts)
                .ok()
                .and_then(|b| String::from_utf8(b).ok());
            // Enums, namespaces and parameter properties are not erasable (strip_types
            // keeps them, so the text is not JavaScript): rewrite the TypeScript as it is.
            let js_opts = CompileOptions {
                typescript: false,
                ..opts
            };
            match js {
                Some(js)
                    if compiler::compile(self.vm.heap_mut(), js.as_bytes(), &js_opts).is_ok() =>
                {
                    (js, false)
                }
                _ => (src.to_string(), true),
            }
        } else {
            (src.to_string(), false)
        };
        let rw = input::rewrite_await(&js, self.frozen)?;
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
        if !matches!(plan, Plan::Incomplete) {
            self.inputs += 1;
            self.current = format!("input {}", self.inputs);
        }
        match plan {
            Plan::Incomplete => return false,
            Plan::Error(e) => println!("{e}"),
            Plan::Expr(f) => {
                if let Some(v) = self.run_body(f, true) {
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
                    && !(v.is_undefined() && input::starts_with_declaration(src))
                {
                    println!("{}", inspect::inspect(self.vm.heap(), v));
                }
            }
            Plan::Await(declare, body) => {
                let h = self.vm.heap_mut().new_handle(body).expect("out of memory");
                // `h` lives in this VM: no `.dump-every` reload while the declarations run.
                self.hold_vm = true;
                let ok = match declare {
                    Some(d) => matches!(self.report(d), Outcome::Done(_)),
                    None => true,
                };
                self.hold_vm = false;
                let body = self.vm.heap().get_handle(h);
                self.vm.heap_mut().drop_handle(h);
                if ok {
                    self.run_body(body, false);
                }
            }
        }
        self.check_waiting();
        self.report_rejections();
        if self.time {
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            println!("({ms:.3} ms, {} fuel)", self.last_fuel);
        }
        true
    }

    /// Runs an async body to its promise's settlement. Returns the fulfillment value; a
    /// rejection is reported.
    /// If it waits on tool calls, it is kept as a waiting input and `None` is returned;
    /// `show`: print its value once it settles.
    fn run_body(&mut self, f: Value, show: bool) -> Option<Value> {
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
            Some(None) if !self.vm.pending_host_calls().is_empty() => {
                if !self.announced {
                    println!("[input {} waiting]", self.inputs);
                }
                let promise = self.vm.heap_mut().new_handle(p).expect("out of memory");
                self.waiting.push(Waiting {
                    label: format!("input {}", self.inputs),
                    promise,
                    show,
                    unwrap: show,
                });
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
        self.pump(true)
    }

    /// Runs the VM until it stops: done, threw, or idle with only tool calls left (it
    /// answers those it has scripted answers for). `entry`: a call was started, so its
    /// outcome is the result; otherwise (after `.resolve`) the outcome is an old one.
    fn pump(&mut self, entry: bool) -> Outcome {
        let mut used: u64 = 0;
        if let Some(h) = self.entry_override.take() {
            self.vm.heap_mut().drop_handle(h);
        }
        // `.step`: pause at every slice until `c`.
        let mut stepping = self.step.is_some();
        let outcome = loop {
            let mut slice = self.suspend_every.unwrap_or(SLICE);
            if let Some(n) = self.step.filter(|_| stepping) {
                slice = slice.min(n);
            }
            if let Some(limit) = self.fuel_limit {
                slice = slice.min(limit.saturating_sub(used).max(1));
            }
            let (st, n) = self.vm.run_metered(slice);
            used += n;
            // After `.dump-every` reloaded the VM, its outcome is the load's: the running
            // input's result came through the dump.
            let st = match (st, self.entry_override.take()) {
                (Status::Done(_), Some(h)) => {
                    let v = self.vm.heap().get_handle(h);
                    self.vm.heap_mut().drop_handle(h);
                    Status::Done(v)
                }
                (st, h) => {
                    self.entry_override = h;
                    st
                }
            };
            match st {
                Status::Done(v) if entry => break Outcome::Done(v),
                Status::Threw(e) if entry => break Outcome::Threw(e),
                Status::Done(_) | Status::Threw(_) => break Outcome::Done(Value::UNDEFINED),
                Status::Idle => {
                    self.note_new_calls();
                    if self.dump_every && !self.hold_vm && !self.vm.pending_host_calls().is_empty()
                    {
                        match self.dump_and_reload(entry) {
                            Ok(n) => println!("[dumped {n} bytes]"),
                            Err(e) => {
                                println!("[not dumped] {e}");
                                self.dump_every = false;
                                println!("dumping at every idle point: off");
                            }
                        }
                    }
                    if self.answer_one() {
                        continue;
                    }
                    if let Some(h) = self.entry_override.take() {
                        let v = self.vm.heap().get_handle(h);
                        self.vm.heap_mut().drop_handle(h);
                        break if entry {
                            Outcome::Done(v)
                        } else {
                            Outcome::Done(Value::UNDEFINED)
                        };
                    }
                    break match self.vm.outcome() {
                        Some(Ok(v)) if entry => Outcome::Done(v),
                        Some(Err(e)) if entry => Outcome::Threw(e),
                        _ => Outcome::Done(Value::UNDEFINED),
                    };
                }
                Status::HostOutOfMemory => break Outcome::Stopped("out of host memory".into()),
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
                    if stepping {
                        match self.step_prompt(used) {
                            Step::Next => {}
                            Step::Continue => stepping = false,
                            Step::Quit => {
                                self.vm.abort();
                                break Outcome::Stopped("Stopped".into());
                            }
                        }
                    }
                    if self.suspend_every.is_some() {
                        let bt = self.vm.backtrace();
                        let at = bt.lines().next().unwrap_or("").trim();
                        println!("[suspended, {used} fuel] {at}");
                        let opts = self.vm.options();
                        self.vm = Vm::from_bytes(self.vm.image().to_vec(), opts)
                            .expect("a valid heap image");
                    }
                }
                Status::HostCall {
                    call_id,
                    fn_id: MAKE_TOOL,
                } => self.tools.make(&mut self.vm, call_id),
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
        self.note_new_calls();
        let fresh = std::mem::take(&mut self.fresh);
        let pending = self.vm.pending_host_calls();
        self.announced = false;
        for id in fresh.into_iter().filter(|id| pending.contains(id)) {
            self.announced = true;
            if self.tools.is_pause(&self.vm, id) {
                self.announce_pause(id);
            } else {
                println!("[waiting] #{id} {}", self.tools.describe(&mut self.vm, id));
            }
        }
        outcome
    }

    /// A `debugger;` pause (D3060): the call, the input that made it and where it stopped,
    /// with the source line; the first time, how to go on.
    fn announce_pause(&mut self, id: u32) {
        let origin = self.origins.get(&id).cloned().unwrap_or_default();
        let heap = self.vm.heap();
        let a = activations::call_promise(heap, id)
            .and_then(|p| activations::waiting_on(heap, p).first().copied())
            .and_then(|g| activations::activation(heap, g))
            .map(|a| self.in_dump_file(a));
        let at = a
            .as_ref()
            .map(|a| format!(" at {}", a.at))
            .unwrap_or_default();
        println!("[paused] #{id} {origin}{at}");
        if let Some(a) = a {
            print!("{}", a.source_lines("  "));
        }
        if !self.paused_before {
            self.paused_before = true;
            println!("(.dump - shows the continuation, .c continues, .abort drops it)");
        }
    }

    /// The open `debugger;` pauses, oldest first.
    fn pauses(&self) -> Vec<u32> {
        let mut ids = self.vm.pending_host_calls();
        ids.retain(|&id| self.tools.is_pause(&self.vm, id));
        ids
    }

    /// `.continue [id]`: answers the pause `#id` (the oldest one without an id) and runs on.
    fn continue_cmd(&mut self, arg: &str) {
        let pauses = self.pauses();
        let id = match arg.trim_start_matches('#') {
            "" => pauses.first().copied(),
            n => n.parse::<u32>().ok().filter(|id| pauses.contains(id)),
        };
        match id {
            // The paused input goes on: its next calls are its own.
            Some(id) => {
                let origin = self.origins.get(&id).cloned();
                let origin = origin.unwrap_or_else(|| format!(".continue {id}"));
                self.answer_and_run(id, Ok(Value::UNDEFINED), origin)
            }
            None if arg.is_empty() => println!("nothing is paused"),
            None => println!("no pause #{arg} (.pending lists the open calls)"),
        }
    }

    /// `.abort` (or Ctrl-C at an empty prompt while paused): drops what is in flight,
    /// the paused input with the others (`vm.abort()`, as Ctrl-C does to a running input;
    /// one input cannot be dropped alone). The globals stay.
    fn abort_pause(&mut self) {
        let pauses = self.pauses();
        if pauses.is_empty() {
            println!("nothing is paused");
            return;
        }
        let paused: Vec<String> = pauses
            .iter()
            .filter_map(|id| self.origins.get(id).cloned())
            .collect();
        let others = self
            .waiting
            .iter()
            .filter(|w| !paused.contains(&w.label))
            .count();
        let calls = self.vm.pending_host_calls().len() - pauses.len();
        self.vm.abort();
        for w in std::mem::take(&mut self.waiting) {
            self.vm.heap_mut().drop_handle(w.promise);
        }
        if let Some(h) = self.entry_override.take() {
            self.vm.heap_mut().drop_handle(h);
        }
        self.origins.clear();
        self.fresh.clear();
        let what = if paused.is_empty() {
            String::from("the paused input")
        } else {
            paused.join(", ")
        };
        let plural = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
        if others == 0 && calls == 0 {
            println!("aborted {what}");
        } else {
            println!(
                "aborted {what}, with {} and {}",
                plural(others, "other waiting input"),
                plural(calls, "open call")
            );
        }
    }

    /// Records the tool calls made since the last look.
    fn note_new_calls(&mut self) {
        for id in self.vm.new_host_calls() {
            self.origins.insert(id, self.current.clone());
            self.fresh.push(id);
        }
    }

    /// Answers the oldest open call the answers file has an answer for.
    /// `later(v)` calls are answered with their argument, quietly.
    fn answer_one(&mut self) -> bool {
        for id in self.vm.pending_host_calls() {
            let desc = self.tools.describe(&mut self.vm, id);
            let later = self
                .vm
                .host_call(id)
                .is_some_and(|c| c.fn_id == tools::LATER);
            if let Some(r) = self.tools.answer(&mut self.vm, id) {
                if !later {
                    println!("[answered] #{id} {desc}");
                }
                self.fresh.retain(|&c| c != id);
                self.origins.remove(&id);
                self.vm.resolve_host_call(id, r).expect("an open call");
                self.current = format!("answer #{id}");
                return true;
            }
        }
        false
    }

    /// Prints the inputs that settled and forgets them.
    fn check_waiting(&mut self) {
        let mut i = 0;
        while i < self.waiting.len() {
            let w = &self.waiting[i];
            let p = self.vm.heap().get_handle(w.promise);
            let (label, show, unwrap) = (w.label.clone(), w.show, w.unwrap);
            match self.vm.promise_state(p) {
                Some(Some(r)) => {
                    match r {
                        Ok(v) if show => {
                            let v = match v.as_object() {
                                Some(a) if unwrap => self
                                    .vm
                                    .heap()
                                    .get_element_fast(a, 0)
                                    .unwrap_or(Value::UNDEFINED),
                                _ => v,
                            };
                            println!("[{label}] {}", inspect::inspect(self.vm.heap(), v));
                        }
                        Ok(_) => {}
                        Err(e) => {
                            println!("[{label}] Uncaught {}", inspect::inspect(self.vm.heap(), e))
                        }
                    }
                    let w = self.waiting.remove(i);
                    self.vm.heap_mut().drop_handle(w.promise);
                }
                _ => i += 1,
            }
        }
    }

    /// `.resolve <id> <json>` / `.reject <id> <message>`.
    fn settle(&mut self, arg: &str, ok: bool) {
        let (id, rest) = match arg.split_once(char::is_whitespace) {
            Some((i, r)) => (i, r.trim()),
            None => (arg, ""),
        };
        let Some(id) = id.trim_start_matches('#').parse::<u32>().ok() else {
            println!("usage: .resolve <id> <json> or .reject <id> <message>");
            return;
        };
        if !self.vm.pending_host_calls().contains(&id) {
            println!("no open call #{id} (.pending lists them)");
            return;
        }
        let result = if ok {
            let text = if rest.is_empty() { "null" } else { rest };
            match tools::parse_json(&mut self.vm, text) {
                Ok(v) => Ok(v),
                Err(e) => {
                    println!("not JSON: {e}");
                    return;
                }
            }
        } else {
            Err(self.vm.new_error(ErrorKind::Error, rest).expect("oom"))
        };
        self.answer_and_run(id, result, format!(".resolve {id}"));
    }

    /// Answers the open call `id` and runs what that unblocks; the calls that makes are
    /// `origin`'s.
    fn answer_and_run(&mut self, id: u32, result: Result<Value, Value>, origin: String) {
        self.vm.resolve_host_call(id, result).expect("an open call");
        self.origins.remove(&id);
        self.current = origin;
        self.vm.set_now(now_ms());
        if let Outcome::Stopped(why) = self.pump(false) {
            println!("{why}");
        }
        self.check_waiting();
        self.report_rejections();
    }

    /// Positions in the functions of a `.restore`d dump under the dump's file name.
    fn in_dump_file(&self, a: activations::Activation) -> activations::Activation {
        match &self.dump_file {
            Some(f) => a.in_file(f),
            None => a,
        }
    }

    /// `.pending`: the open calls, what issued them and where the code waits.
    fn pending(&mut self) {
        let ids = self.vm.pending_host_calls();
        if ids.is_empty() {
            println!("no open calls");
        }
        for id in ids {
            let desc = self.tools.describe(&mut self.vm, id);
            let mut line = format!("#{id} {desc}");
            if let Some(o) = self.origins.get(&id) {
                line.push_str(&format!("  {o}"));
            }
            let heap = self.vm.heap();
            if let Some(p) = activations::call_promise(heap, id)
                && let Some(&g) = activations::waiting_on(heap, p).first()
                && let Some(a) = activations::activation(heap, g).map(|a| self.in_dump_file(a))
            {
                line.push_str(&format!(", at {}", a.at));
            }
            println!("{line}");
        }
    }

    /// `.bt <id|all>`: suspended activations with their bindings.
    fn backtrace_cmd(&mut self, arg: &str) {
        // Dead activations would be listed too.
        if arg == "all"
            && let Err(e) = self.vm.collect()
        {
            println!("collection failed: {e:?}");
            return;
        }
        let inputs: Vec<(Value, String)> = self
            .waiting
            .iter()
            .map(|w| (self.vm.heap().get_handle(w.promise), w.label.clone()))
            .collect();
        let print = |a: &activations::Activation| {
            let input = inputs
                .iter()
                .find(|(p, _)| *p == a.promise)
                .map(|(_, n)| format!(" ({n})"))
                .unwrap_or_default();
            print!("{}", a.describe(&input));
        };
        if arg == "all" {
            let all: Vec<_> = activations::all(self.vm.heap())
                .into_iter()
                .map(|a| self.in_dump_file(a))
                .collect();
            if all.is_empty() {
                println!("no suspended activations");
            }
            all.iter().for_each(print);
            return;
        }
        let Some(id) = arg.trim_start_matches('#').parse::<u32>().ok() else {
            println!("usage: .bt <id|all>");
            return;
        };
        let heap = self.vm.heap();
        let Some(p) = activations::call_promise(heap, id) else {
            println!("no open call #{id} (.pending lists them)");
            return;
        };
        let gens = activations::waiting_on(heap, p);
        let acts: Vec<_> = gens
            .iter()
            .filter_map(|&g| activations::activation(heap, g))
            .map(|a| self.in_dump_file(a))
            .collect();
        let desc = self.tools.describe(&mut self.vm, id);
        println!("#{id} {desc}");
        if acts.is_empty() {
            println!("  no suspended activation waits on it");
        }
        acts.iter().for_each(print);
    }

    /// The `step>` prompt of a paused input: shows where it stands, reads commands until
    /// one resumes it.
    fn step_prompt(&mut self, used: u64) -> Step {
        let bt = self.vm.backtrace();
        let at = bt.lines().next().unwrap_or("").trim().to_string();
        println!("[step, {used} fuel] {at}");
        loop {
            let Some(line) = self.read_nested("step> ") else {
                return Step::Continue;
            };
            match line.trim() {
                "" | "s" | "n" | "next" => return Step::Next,
                "c" | "continue" => return Step::Continue,
                "q" | "quit" | "stop" => return Step::Quit,
                "bt" | "backtrace" => print!("{bt}"),
                "fuel" => println!("{used} fuel"),
                other => println!(
                    "{other}: at step> use Enter (next slice), c (continue), bt, fuel or q (stop)"
                ),
            }
            let _ = std::io::stdout().flush();
        }
    }

    /// A line for a nested prompt: from the terminal, or the next input line.
    fn read_nested(&mut self, prompt: &str) -> Option<String> {
        if let Some(t) = self.term.as_mut() {
            return match t.read_line(prompt, &[], &mut |_| (0, Vec::new())) {
                Input::Line(l) => Some(l),
                Input::Eof => None,
                Input::Interrupt => Some(String::from("q")),
            };
        }
        self.source.as_mut()?.next()
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
            ".decorators" => {
                match arg {
                    "" => {}
                    "tc39" | "standard" => {
                        self.legacy_decorators = false;
                        self.decorator_metadata = false;
                    }
                    "legacy" => {
                        self.legacy_decorators = true;
                        self.decorator_metadata = false;
                    }
                    "metadata" => {
                        self.legacy_decorators = true;
                        self.decorator_metadata = true;
                    }
                    _ => {
                        println!(".decorators takes tc39, legacy or metadata");
                        return None;
                    }
                }
                let which = match (self.legacy_decorators, self.decorator_metadata) {
                    (false, _) => "tc39",
                    (true, false) => "legacy (experimentalDecorators)",
                    (true, true) => "legacy with emitDecoratorMetadata",
                };
                let note = if self.legacy_decorators && !self.typescript {
                    " (applies in TypeScript mode: .ts on)"
                } else {
                    ""
                };
                println!("decorators: {which}{note}");
            }
            ".step" => match number_or_off(arg) {
                Ok(n) => {
                    self.step = if arg.is_empty() { Some(1000) } else { n };
                    match self.step {
                        Some(n) => println!("stepping every {n} fuel"),
                        None => println!("stepping: off"),
                    }
                }
                Err(e) => println!("{e}"),
            },
            ".complete" => {
                let (_, cands) = complete::complete(&self.vm, arg);
                println!("{}", cands.join(" "));
            }
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
            ".heap" => self.heap_report(arg),
            ".pending" => self.pending(),
            ".continue" | ".c" => self.continue_cmd(arg),
            ".abort" => self.abort_pause(),
            ".resolve" if !arg.is_empty() => self.settle(arg, true),
            ".reject" if !arg.is_empty() => self.settle(arg, false),
            ".bt" if !arg.is_empty() => self.backtrace_cmd(arg),
            ".dump" if !arg.is_empty() => self.dump(arg),
            ".dump-every" => match on_off(arg, &mut self.dump_every) {
                Ok(()) if self.dump_every && !self.frozen => {
                    self.dump_every = false;
                    println!("{NOT_FROZEN}");
                }
                Ok(()) => println!(
                    "dumping at every idle point: {}",
                    if self.dump_every { "on" } else { "off" }
                ),
                Err(e) => println!("{e}"),
            },
            ".restore" if !arg.is_empty() => match std::fs::read(arg) {
                Ok(bytes) if crate::dumps::is_dump(&bytes) => {
                    let src = String::from_utf8_lossy(&bytes).into_owned();
                    self.restore_dump(arg, &src);
                }
                Ok(_) => println!("{arg} is not a djs dump (.restore reads what .dump writes)"),
                Err(e) => println!("cannot read {arg}: {e}"),
            },
            ".clear" => {
                self.replace_vm(new_vm(self.gc_stress, self.frozen));
                println!("a fresh VM");
            }
            ".disasm" if !arg.is_empty() => self.disasm(arg),
            _ => println!("unknown command or missing argument: {line} (.help lists them)"),
        }
        None
    }

    /// `.dump <file> [names]`: the session as a dump (0167, 0174, D2830: with what is in
    /// flight, the waiting inputs as handles).
    fn dump(&mut self, arg: &str) {
        if !self.frozen {
            println!("{NOT_FROZEN}");
            return;
        }
        let mut parts = arg.split_whitespace();
        let file = parts.next().unwrap_or("-");
        let names: Vec<&str> = parts.collect();
        let sel = (!names.is_empty()).then_some(names.as_slice());
        let handles = self.waiting_handles(None);
        let refs: Vec<(&str, Value)> = handles.iter().map(|(n, v)| (n.as_str(), *v)).collect();
        let origin = |id: u32| self.origins.get(&id).cloned();
        match crate::dumps::dump_names(&self.vm, &self.tools, &refs, sel, &origin, false) {
            Ok(d) if file == "-" => print!("{}", d.text),
            Ok(d) => match std::fs::write(file, &d.text) {
                Ok(()) => println!(
                    "wrote {} bytes to {file}{}",
                    d.text.len(),
                    crate::dumps::in_flight(&d)
                ),
                Err(e) => println!("cannot write {file}: {e}"),
            },
            Err(e) => println!("{e}"),
        }
    }

    /// The waiting inputs as the host's handles of a dump, by label; with `entry`, also
    /// the running input's result as `$entry`.
    fn waiting_handles(&self, entry: Option<Value>) -> Vec<(String, Value)> {
        let mut out: Vec<(String, Value)> = self
            .waiting
            .iter()
            .map(|w| (w.handle_name(), self.vm.heap().get_handle(w.promise)))
            .collect();
        out.extend(entry.map(|v| (String::from(ENTRY), v)));
        out
    }

    /// `.dump-every`: dumps the idle VM and continues in a fresh one that loaded the dump
    /// (D2604). Waiting inputs, the running input's result (`entry`), open calls and the
    /// tool registry carry over. The dump's size.
    fn dump_and_reload(&mut self, entry: bool) -> Result<usize, String> {
        let entry = match (self.entry_override, entry) {
            (Some(h), _) => Some(self.vm.heap().get_handle(h)),
            (None, true) => match self.vm.outcome() {
                Some(Ok(v)) => Some(v),
                _ => None,
            },
            (None, false) => None,
        };
        let handles = self.waiting_handles(entry);
        let refs: Vec<(&str, Value)> = handles.iter().map(|(n, v)| (n.as_str(), *v)).collect();
        let origin = |id: u32| self.origins.get(&id).cloned();
        let d = crate::dumps::dump(&self.vm, &self.tools, &refs, &origin, false)?;
        let mut vm = new_vm(self.gc_stress, self.frozen);
        // The fresh VM runs the dump; its prints are the run's (a reaction that ran).
        let mut log = |vm: &Vm, args: &[Value]| {
            let parts: Vec<String> = args
                .iter()
                .map(|&a| inspect::log_arg(vm.heap(), a))
                .collect();
            println!("{}", parts.join(" "));
        };
        crate::dumps::run_plain(
            &mut vm,
            &mut self.tools,
            &d.text,
            "<dump>",
            self.frozen,
            &mut log,
        )?;
        let mut handles = crate::dumps::plain_handles(&mut vm);
        // The old VM's handles go with it; positions are now in this dump, which has no file.
        self.vm = vm;
        self.dump_file = None;
        let mut kept = Vec::new();
        for mut w in std::mem::take(&mut self.waiting) {
            if let Some(h) = crate::dumps::take_handle(&mut handles, &w.handle_name()) {
                w.promise = h;
                kept.push(w);
            }
        }
        self.waiting = kept;
        self.entry_override = crate::dumps::take_handle(&mut handles, ENTRY);
        for (_, h) in handles {
            self.vm.heap_mut().drop_handle(h);
        }
        Ok(d.text.len())
    }

    /// `.restore` of a dump (D2603): a fresh VM loads it. The open calls it recreates
    /// keep their ids, named tools get their functions' ids from the registry, and the
    /// handles it names become waiting inputs (labelled by their names). The tool names
    /// are not read back from the heap afterwards: an open call's tool may have no
    /// function there yet.
    fn restore_dump(&mut self, arg: &str, src: &str) {
        self.replace_vm(new_vm(self.gc_stress, self.frozen));
        // A dump (D2830) is a Script: the fresh VM runs it, printing as inputs do.
        let mut log = |vm: &Vm, args: &[Value]| {
            let parts: Vec<String> = args
                .iter()
                .map(|&a| inspect::log_arg(vm.heap(), a))
                .collect();
            println!("{}", parts.join(" "));
        };
        // The values the dump names, and the open calls' states.
        let r = crate::dumps::run_plain(&mut self.vm, &mut self.tools, src, arg, true, &mut log);
        let (handles, states) = match r {
            Ok(l) => (crate::dumps::plain_handles(&mut self.vm), l.states),
            Err(e) => {
                println!("cannot load {arg}: {e}");
                return;
            }
        };
        println!("loaded {arg} into a fresh VM");
        self.dump_file = Some(arg.to_string());
        // New inputs are numbered after the restored ones (`input 3`), so labels stay
        // unique handle names for later dumps.
        self.inputs = handles
            .iter()
            .filter_map(|(n, _)| {
                let n = n.strip_prefix("input ")?;
                n.strip_suffix(" (value)").unwrap_or(n).parse::<u32>().ok()
            })
            .max()
            .unwrap_or(0);
        for id in self.vm.pending_host_calls() {
            let state = states.iter().find(|(i, _)| *i == id);
            let o = state.and_then(|(_, s)| Tools::state_origin(s));
            self.origins
                .insert(id, o.unwrap_or_else(|| String::from("the dump")));
        }
        for (name, h) in handles {
            let v = self.vm.heap().get_handle(h);
            match self.vm.promise_state(v) {
                Some(None) => {
                    self.vm.mark_promise_handled(v);
                    self.waiting.push(Waiting::from_handle_name(name, h));
                }
                _ => {
                    println!("[{name}] {}", inspect::inspect(self.vm.heap(), v));
                    self.vm.heap_mut().drop_handle(h);
                }
            }
        }
        self.pending_after_restore();
        // An idle point: `later` calls (and scripted answers) are answered at once.
        self.current = String::from("the dump");
        if let Outcome::Stopped(why) = self.pump(false) {
            println!("{why}");
        }
        self.check_waiting();
        self.report_rejections();
    }

    /// After a restore: the calls the image still has open.
    fn pending_after_restore(&mut self) {
        for id in self.vm.pending_host_calls() {
            if self.tools.is_pause(&self.vm, id) {
                self.announce_pause(id);
            } else {
                println!("waiting: #{id} {}", self.tools.describe(&mut self.vm, id));
            }
        }
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
            let Some(v) = self.run_body(f, true) else {
                return;
            };
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
                ObjectClass::Function => return Some(Ref(heap.closure_template(o))),
                ObjectClass::BoundFunction => o = heap.object_slot(o, 0).as_object()?,
                _ => return None,
            }
        }
        None
    }

    /// `.heap [sections]`: the heap analyzer over the live VM's image, against the base
    /// image, so the session's own cells show apart from the base realm (D2340).
    fn heap_report(&self, arg: &str) {
        let only = match arg {
            "" => Some(vec!["summary".into(), "diff".into()]),
            "all" => None,
            a => Some(a.split(',').map(|k| k.trim().to_string()).collect()),
        };
        let o = crate::heap::Options {
            only,
            ..Default::default()
        };
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
        let image = self.vm.bytes();
        match crate::heap::analyze(image, Some(("the base image", djs_prelude::BASE)), &o) {
            Ok(sections) => print!("{}", crate::heap::to_text(&sections)),
            Err(e) => println!("cannot analyze the heap: {e}"),
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
        CompileErrorKind::HostOutOfMemory => "djs: out of host memory while compiling".into(),
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
fn new_vm(gc_stress: bool, frozen_builtins: bool) -> Vm {
    let opts = VmOptions {
        exec: ExecOptions {
            gc_stress,
            ..Default::default()
        },
        frozen_builtins,
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
    tools::install(&mut vm);
    // The dumping mode is the debugger mode: `debugger;` awaits `debugger()` (D3060).
    if frozen_builtins {
        tools::install_debugger(&mut vm);
    }
    vm
}
