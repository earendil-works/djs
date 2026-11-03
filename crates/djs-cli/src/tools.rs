//! A host-provided `tools` object for `djs repl` and `djs run` (0174, D2600): every
//! `tools.<name>(...)` is an async host call (D1374) that the user answers by hand
//! (`.resolve`) or that an answers file answers (`--tools answers.json`).
//!
//! `tools` is a Proxy over an object of made functions. Reading a new name calls the sync
//! host function `make`, which creates an async host function with a fresh id and that
//! name; the Proxy keeps it. So a tool's id and name live in the heap, and survive images.

use std::cell::RefCell;
use std::collections::HashMap;

use djs_core::compiler::CompileOptions;
use djs_core::heap::ObjectClass;
use djs_core::interp::{Intrinsic, intrinsic};
use djs_core::json;
use djs_core::value::Value;
use djs_core::vm::{ErrorKind, Status, Vm};

use crate::heap::json::J;

/// The sync host function behind `tools`'s Proxy: `make(name)` returns a new tool.
pub const MAKE_TOOL: u32 = 2;
/// Tool functions have ids from here on, in the order they were made.
pub const TOOL_BASE: u32 = 16;

/// Defines `tools` (an async body called with `make`). `then` is left out, so that
/// `await tools` does not make a tool.
const SETUP: &str = "const made = Object.create(null);\n\
Object.defineProperty(globalThis, 'tools', {\n\
  value: new Proxy(made, {\n\
    get(t, k) {\n\
      if (typeof k !== 'string' || k === 'then') return undefined;\n\
      return t[k] ?? (t[k] = make(k));\n\
    },\n\
  }),\n\
  writable: true,\n\
  configurable: true,\n\
});";

/// Defines `tools` in `vm`.
pub fn install(vm: &mut Vm) {
    let params = ["make"];
    let opts = CompileOptions::async_body("<tools>", &params);
    let f = vm
        .compile(SETUP.as_bytes(), &opts)
        .expect("the tools setup compiles");
    let make = vm.new_host_function(MAKE_TOOL, "make").expect("oom");
    let this = Value::object(vm.global());
    vm.call(f, this, &[make]).expect("the VM is idle");
    match vm.run(u64::MAX) {
        Status::Done(_) => {}
        s => panic!("the tools setup did not finish: {s:?}"),
    }
}

/// The tools of a VM by id, and the scripted answers.
#[derive(Default)]
pub struct Tools {
    /// Names by `fn_id - TOOL_BASE`. A cell: a dump's loader asks for ids through a
    /// shared reference ([`Tools::id_for_call_name`]).
    names: RefCell<Vec<String>>,
    answers: Option<Answers>,
}

impl Tools {
    pub fn with_answers(answers: Option<Answers>) -> Tools {
        Tools {
            names: RefCell::new(Vec::new()),
            answers,
        }
    }

    pub fn has_answers(&self) -> bool {
        self.answers.is_some()
    }

    /// Serves `make(name)`, the parked sync call `call_id`.
    pub fn make(&mut self, vm: &mut Vm, call_id: u32) {
        let info = vm.host_call(call_id).expect("parked call");
        let name = info
            .args
            .first()
            .and_then(|v| v.as_string())
            .map(|s| vm.heap().str_to_string(s))
            .unwrap_or_default();
        let id = self.id_for(&name);
        let f = vm.new_async_host_function(id, &name).expect("oom");
        vm.resolve_host_call(call_id, Ok(f))
            .expect("parked on this call");
    }

    /// The id of the tool `name`: the one it has (made before, or registered by a loaded
    /// dump's open call), else the next free one. A restored session and new calls of
    /// the same tool agree on it.
    pub fn id_for(&self, name: &str) -> u32 {
        let mut names = self.names.borrow_mut();
        let i = match names.iter().position(|n| n == name) {
            Some(i) => i,
            None => {
                names.push(name.to_string());
                names.len() - 1
            }
        };
        TOOL_BASE + i as u32
    }

    /// The host function id for a call name a dump names (`tools.read`, 0171's
    /// `LoadOptions::host_fn`); `None` for a name that is not a tool.
    // Wired to `Vm::load_dump` once 0171 lands.
    #[allow(dead_code)]
    pub fn id_for_call_name(&self, call_name: &str) -> Option<u32> {
        let name = call_name.strip_prefix("tools.")?;
        (!name.is_empty()).then(|| self.id_for(name))
    }

    /// The call name of a tool's host function id (`tools.read`), for a dump's exporter.
    pub fn call_name(&self, fn_id: u32) -> Option<String> {
        let i = fn_id.checked_sub(TOOL_BASE)? as usize;
        self.names.borrow().get(i).map(|n| format!("tools.{n}"))
    }

    /// Reads the tools back from the heap (after a restore): the functions the Proxy
    /// keeps, with their ids.
    pub fn rescan(&mut self, vm: &mut Vm) {
        let names = self.names.get_mut();
        names.clear();
        let Ok(key) = vm.heap_mut().atom_str("tools") else {
            return;
        };
        let heap = vm.heap();
        let Some(proxy) = heap
            .find_property(vm.global(), key)
            .and_then(|(_, p)| p.value())
            .and_then(|v| v.as_object())
            .filter(|&o| heap.is_proxy(o))
        else {
            return;
        };
        let Some(made) = heap.proxy_target(proxy) else {
            return;
        };
        for k in heap.own_keys(made) {
            let f = heap
                .get_own_property(made, k)
                .and_then(|p| p.value())
                .and_then(|v| v.as_object())
                .filter(|&f| heap.object_class(f) == ObjectClass::HostFunction);
            let Some(id) = f.and_then(|f| heap.object_slot(f, 0).as_int()) else {
                continue;
            };
            let Some(i) = (id as u32).checked_sub(TOOL_BASE) else {
                continue;
            };
            let i = i as usize;
            if names.len() <= i {
                names.resize(i + 1, String::from("?"));
            }
            names[i] = heap.atom_name(k);
        }
    }

    /// `tools.read`, or `host function 3` for another id.
    pub fn name(&self, fn_id: u32) -> String {
        self.call_name(fn_id)
            .unwrap_or_else(|| format!("host function {fn_id}"))
    }

    /// `tools.read({"path":"a.ts"})`: the call's name and its arguments as JSON.
    pub fn describe(&self, vm: &mut Vm, call_id: u32) -> String {
        let Some(info) = vm.host_call(call_id) else {
            return format!("call {call_id}");
        };
        let args: Vec<String> = info.args.iter().map(|&a| to_json(vm, a)).collect();
        format!("{}({})", self.name(info.fn_id), args.join(", "))
    }

    /// The scripted answer for the pending call `call_id`, if the answers file has one.
    pub fn answer(&mut self, vm: &mut Vm, call_id: u32) -> Option<Result<Value, Value>> {
        let info = vm.host_call(call_id)?;
        let name = self.call_name(info.fn_id)?["tools.".len()..].to_string();
        let parts: Vec<String> = info.args.iter().map(|&a| to_json(vm, a)).collect();
        let args = J::parse(&format!("[{}]", parts.join(","))).ok();
        let entry = self.answers.as_mut()?.pick(&name, args.as_ref())?;
        Some(match entry {
            Ok(v) => Ok(parse_json(vm, &v.render()).expect("valid JSON")),
            Err(msg) => Err(vm.new_error(ErrorKind::Error, &msg).expect("oom")),
        })
    }
}

/// JSON text parsed into a VM value (`JSON.parse`).
pub fn parse_json(vm: &mut Vm, text: &str) -> Result<Value, String> {
    let heap = vm.heap_mut();
    let opts = json::ParseOptions {
        object_proto: intrinsic(heap, Intrinsic::ObjectProto).as_object(),
        array_proto: intrinsic(heap, Intrinsic::ArrayProto).as_object(),
        ..json::ParseOptions::default()
    };
    json::parse_utf8(heap, text.as_bytes(), &opts).map_err(|e| match e {
        json::ParseError::Syntax(e) => e.text(),
        json::ParseError::Heap(_) => String::from("out of memory"),
    })
}

/// A value as JSON text, or `undefined` / the display form when it has none.
pub fn to_json(vm: &mut Vm, v: Value) -> String {
    match json::stringify_fast(vm.heap_mut(), v, &[]) {
        Ok(Some(s)) => vm.heap().str_to_string(s),
        Ok(None) => String::from("undefined"),
        Err(_) => vm.display(v),
    }
}

/// `--tools answers.json` (D2601): an object from tool name to a list of answers, each
/// `{"value": v}` or `{"error": "message"}`, optionally with `"args": [...]`. A call is
/// answered by the first entry whose `args` equal its arguments (as JSON; such an entry
/// answers every matching call), else by the next entry without `args` in order (the
/// last one repeats). A call with no answer stays open.
pub struct Answers {
    tools: Vec<(String, Vec<Entry>)>,
    next: HashMap<String, usize>,
}

struct Entry {
    args: Option<J>,
    result: Result<J, String>,
}

impl Answers {
    pub fn parse(text: &str) -> Result<Answers, String> {
        let J::Obj(fields) = J::parse(text)? else {
            return Err("expected an object from tool names to lists of answers".into());
        };
        let mut tools = Vec::new();
        for (name, list) in fields {
            let J::Arr(items) = list else {
                return Err(format!("{name}: expected a list of answers"));
            };
            let mut entries = Vec::new();
            for (i, item) in items.into_iter().enumerate() {
                let J::Obj(f) = item else {
                    return Err(format!("{name}[{i}]: expected an object"));
                };
                let get = |k: &str| f.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
                let result = match (get("value"), get("error")) {
                    (Some(v), None) => Ok(v),
                    (None, Some(J::Str(m))) => Err(m),
                    _ => {
                        return Err(format!(
                            "{name}[{i}]: needs \"value\" or \"error\" (a string)"
                        ));
                    }
                };
                let args = get("args");
                if args.as_ref().is_some_and(|a| !matches!(a, J::Arr(_))) {
                    return Err(format!("{name}[{i}]: \"args\" must be a list"));
                }
                entries.push(Entry { args, result });
            }
            tools.push((name, entries));
        }
        Ok(Answers {
            tools,
            next: HashMap::new(),
        })
    }

    fn pick(&mut self, name: &str, args: Option<&J>) -> Option<Result<J, String>> {
        let entries = &self.tools.iter().find(|(n, _)| n == name)?.1;
        if let Some(e) = entries.iter().find(|e| {
            e.args.is_some() && e.args.as_ref().zip(args).is_some_and(|(a, b)| same(a, b))
        }) {
            return Some(e.result.clone());
        }
        let ordered: Vec<&Entry> = entries.iter().filter(|e| e.args.is_none()).collect();
        let n = self.next.entry(name.to_string()).or_insert(0);
        let e = ordered.get(*n).or(ordered.last())?;
        *n += 1;
        Some(e.result.clone())
    }
}

/// JSON equality, with objects compared regardless of key order.
fn same(a: &J, b: &J) -> bool {
    match (a, b) {
        (J::Arr(x), J::Arr(y)) => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b)),
        (J::Obj(x), J::Obj(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.iter().any(|(k2, v2)| k == k2 && same(v, v2)))
        }
        _ => a == b,
    }
}

impl J {
    /// Parses JSON text (the answers file, and arguments rendered by the VM).
    pub fn parse(text: &str) -> Result<J, String> {
        let mut p = JsonParser {
            s: text.as_bytes(),
            i: 0,
        };
        let v = p.value()?;
        p.ws();
        if p.i != p.s.len() {
            return Err(p.err("unexpected text after the value"));
        }
        Ok(v)
    }
}

struct JsonParser<'a> {
    s: &'a [u8],
    i: usize,
}

impl JsonParser<'_> {
    fn err(&self, what: &str) -> String {
        format!("JSON: {what} at byte {}", self.i)
    }

    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<J, String> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'{') => {
                self.i += 1;
                let mut fields = Vec::new();
                if self.eat(b'}') {
                    return Ok(J::Obj(fields));
                }
                loop {
                    self.ws();
                    let J::Str(k) = self.string()? else {
                        unreachable!()
                    };
                    if !self.eat(b':') {
                        return Err(self.err("expected ':'"));
                    }
                    fields.push((k, self.value()?));
                    if self.eat(b'}') {
                        return Ok(J::Obj(fields));
                    }
                    if !self.eat(b',') {
                        return Err(self.err("expected ',' or '}'"));
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                if self.eat(b']') {
                    return Ok(J::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    if self.eat(b']') {
                        return Ok(J::Arr(items));
                    }
                    if !self.eat(b',') {
                        return Err(self.err("expected ',' or ']'"));
                    }
                }
            }
            Some(b'"') => self.string(),
            Some(_) => {
                for (word, v) in [
                    ("true", J::Bool(true)),
                    ("false", J::Bool(false)),
                    ("null", J::Null),
                ] {
                    if self.s[self.i..].starts_with(word.as_bytes()) {
                        self.i += word.len();
                        return Ok(v);
                    }
                }
                let start = self.i;
                while self
                    .s
                    .get(self.i)
                    .is_some_and(|c| matches!(c, b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'))
                {
                    self.i += 1;
                }
                std::str::from_utf8(&self.s[start..self.i])
                    .ok()
                    .and_then(|t| t.parse::<f64>().ok())
                    .map(J::Num)
                    .ok_or_else(|| self.err("unexpected character"))
            }
            None => Err(self.err("unexpected end")),
        }
    }

    fn string(&mut self) -> Result<J, String> {
        if self.s.get(self.i) != Some(&b'"') {
            return Err(self.err("expected a string"));
        }
        self.i += 1;
        let mut units: Vec<u16> = Vec::new();
        loop {
            let Some(&c) = self.s.get(self.i) else {
                return Err(self.err("unterminated string"));
            };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = *self.s.get(self.i).ok_or_else(|| self.err("bad escape"))?;
                    self.i += 1;
                    match e {
                        b'n' => units.push(10),
                        b't' => units.push(9),
                        b'r' => units.push(13),
                        b'b' => units.push(8),
                        b'f' => units.push(12),
                        b'u' => {
                            let hex = self
                                .s
                                .get(self.i..self.i + 4)
                                .and_then(|h| std::str::from_utf8(h).ok())
                                .and_then(|h| u16::from_str_radix(h, 16).ok())
                                .ok_or_else(|| self.err("bad \\u escape"))?;
                            self.i += 4;
                            units.push(hex);
                        }
                        c => units.push(u16::from(c)),
                    }
                }
                _ => {
                    // Copy a run of UTF-8 as is.
                    let start = self.i - 1;
                    while self.s.get(self.i).is_some_and(|&c| c != b'"' && c != b'\\') {
                        self.i += 1;
                    }
                    let run = std::str::from_utf8(&self.s[start..self.i])
                        .map_err(|_| self.err("invalid UTF-8"))?;
                    units.extend(run.encode_utf16());
                }
            }
        }
        Ok(J::Str(String::from_utf16_lossy(&units)))
    }
}

/// Reads `--tools FILE`.
pub fn read_answers(path: &str) -> Result<Answers, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    Answers::parse(&text).map_err(|e| format!("{path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json() {
        let j = J::parse(r#"{"a": [1, -2.5e1, true, null], "b": "x\n\u00e9é"}"#).unwrap();
        assert_eq!(
            j,
            J::Obj(vec![
                (
                    "a".into(),
                    J::Arr(vec![J::Num(1.0), J::Num(-25.0), J::Bool(true), J::Null])
                ),
                ("b".into(), J::Str("x\néé".into())),
            ])
        );
        assert!(J::parse("[1,]").is_err());
        assert!(J::parse("{} x").is_err());
    }

    #[test]
    fn ids_by_name() {
        let t = Tools::default();
        assert_eq!(t.id_for_call_name("tools.read"), Some(TOOL_BASE));
        assert_eq!(t.id_for("glob"), TOOL_BASE + 1);
        assert_eq!(t.id_for("read"), TOOL_BASE);
        assert_eq!(t.id_for_call_name("console.log"), None);
        assert_eq!(t.id_for_call_name("tools."), None);
        assert_eq!(t.call_name(TOOL_BASE + 1).as_deref(), Some("tools.glob"));
        assert_eq!(t.call_name(TOOL_BASE + 2), None);
        assert_eq!(t.call_name(1), None);
    }

    #[test]
    fn picking_answers() {
        let mut a = Answers::parse(
            r#"{"read": [{"args": [{"path": "b"}], "value": "B"}, {"value": 1}, {"error": "no"}]}"#,
        )
        .unwrap();
        let args = |s: &str| J::parse(s).unwrap();
        assert_eq!(
            a.pick("read", Some(&args(r#"[{"path":"b"}]"#))),
            Some(Ok(J::Str("B".into())))
        );
        assert_eq!(
            a.pick("read", Some(&args(r#"[{"path":"a"}]"#))),
            Some(Ok(J::Num(1.0)))
        );
        assert_eq!(a.pick("read", None), Some(Err("no".into())));
        assert_eq!(a.pick("read", None), Some(Err("no".into())));
        assert_eq!(a.pick("glob", None), None);
        assert!(Answers::parse(r#"{"x": [{"value": 1, "error": "e"}]}"#).is_err());
    }
}
