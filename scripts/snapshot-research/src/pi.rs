//! A Rust copy of js/pi/vm.ts's session logic (deterministic mode), enough to run pi's
//! prelude and a codemode script against fake tools and take images at idle points.

use djs_core::compiler::CompileOptions;
use djs_core::heap::Handle;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions, WeakRefs};

pub const HOST_GLOBALS: &str = r#"
const define = (name, value) =>
  Object.defineProperty(globalThis, name, { value, writable: true, enumerable: false, configurable: true });
const now = Date.now;
let origin = 0;
const performance = {};
Object.defineProperty(performance, "now", { value: { now() { return now() - origin; } }.now, writable: true, configurable: true });
define("performance", performance);
return (time) => { origin = time; };
"#;

const BRIDGE_FN: u32 = 1;
pub const SCRIPT_PREFIX: &str = "return (async (tools, console) => {";

#[derive(Debug, Clone)]
pub struct Call {
    pub id: u32,
    pub name: String,
    pub args: Option<String>,
}

#[derive(Debug, Default)]
pub struct Events {
    pub calls: Vec<Call>,
    pub output: Vec<String>,
    pub done: Option<(bool, String)>,
    pub issued: u32,
}

pub struct Session {
    pub vm: Vm,
    pub api: Handle,
    pub settle: Handle,
    pub stalled: Handle,
    pub run: Handle,
    pub set_origin: Handle,
    pub ev: Events,
    pub now: f64,
}

pub fn opts() -> VmOptions {
    VmOptions {
        weak_refs: WeakRefs::Absent,
        ..VmOptions::default()
    }
}

impl Session {
    fn h(&mut self, v: Value) -> Handle {
        self.vm.heap_mut().new_handle(v).expect("handle")
    }
    fn get(&self, h: Handle) -> Value {
        self.vm.heap().get_handle(h)
    }
    fn str_val(&mut self, s: &str) -> Value {
        Value::string(self.vm.heap_mut().alloc_str(s).expect("str"))
    }
    fn to_string(&self, v: Value) -> Option<String> {
        v.as_string().map(|s| self.vm.heap().str_to_string(s))
    }

    /// Calls `f` and runs until idle; returns a handle to the result (or undefined).
    pub fn invoke(&mut self, f: Value, this: Value, args: &[Value]) -> Result<Handle, String> {
        self.vm.call(f, this, args).map_err(|e| format!("{e:?}"))?;
        loop {
            self.vm.set_now(self.now);
            match self.vm.run(u64::MAX) {
                Status::Done(v) => return Ok(self.h(v)),
                Status::Idle => return Ok(self.h(Value::UNDEFINED)),
                Status::Threw(e) => return Err(self.vm.display(e)),
                Status::HostCall { call_id, fn_id } => {
                    assert_eq!(fn_id, BRIDGE_FN);
                    let info = self.vm.host_call(call_id).unwrap();
                    self.bridge(&info.args);
                    self.vm
                        .resolve_host_call(call_id, Ok(Value::UNDEFINED))
                        .unwrap();
                }
                s => return Err(format!("unexpected {s:?}")),
            }
        }
    }

    fn bridge(&mut self, args: &[Value]) {
        let kind = self.to_string(args[0]).unwrap();
        let a = args.get(1).copied().unwrap_or(Value::UNDEFINED);
        let b = args.get(2).copied().unwrap_or(Value::UNDEFINED);
        let c = args.get(3).copied().unwrap_or(Value::UNDEFINED);
        match kind.as_str() {
            "call" | "global" => {
                self.ev.issued += 1;
                self.ev.calls.push(Call {
                    id: a.as_number().unwrap() as u32,
                    name: self.to_string(b).unwrap(),
                    args: self.to_string(c),
                });
            }
            "output" => {
                let t = self.to_string(b).unwrap_or_default();
                self.ev.output.push(t);
            }
            "done" => {
                let ok = a.as_bool() == Some(true);
                let t = self.to_string(b).unwrap_or_default();
                self.ev.done = Some((ok, t));
            }
            _ => panic!("bridge {kind}"),
        }
    }

    fn evaluate(&mut self, f: Value, args: &[Value]) -> Result<Handle, String> {
        let p = self.invoke(f, Value::UNDEFINED, args)?;
        let pv = self.get(p);
        match self.vm.promise_state(pv) {
            Some(Some(Ok(v))) => {
                let h = self.h(v);
                self.vm.heap_mut().drop_handle(p);
                Ok(h)
            }
            other => Err(format!("evaluate: {other:?}")),
        }
    }

    fn compile(&mut self, file: &str, params: &[&str], src: &str) -> Value {
        let o = CompileOptions::async_body(file, params);
        self.vm
            .compile(src.as_bytes(), &o)
            .unwrap_or_else(|e| panic!("{file}: {e}"))
    }

    /// A fresh VM from the base, with pi's host globals and prelude: the state pi's
    /// adapter caches as the prelude image.
    pub fn prepare(prelude: &str, tools_json: &str, started_at: f64) -> Session {
        let vm = djs_prelude::new_vm(opts()).unwrap();
        let mut s = Session {
            vm,
            api: Handle::from_index(0),
            settle: Handle::from_index(0),
            stalled: Handle::from_index(0),
            run: Handle::from_index(0),
            set_origin: Handle::from_index(0),
            ev: Events::default(),
            now: started_at,
        };
        let bridge = s.vm.new_host_function(BRIDGE_FN, "bridge").unwrap();
        let bridge = s.h(bridge);
        let getter = s.compile("codemode-prelude.js", &["o", "k"], "return o[k]");
        let getter = s.h(getter);
        let host = s.compile("codemode-prelude.js", &[], HOST_GLOBALS);
        let set_origin = s.evaluate(host, &[]).unwrap();
        let prelude_fn = s.compile("codemode-prelude.js", &[], &format!("return {prelude}"));
        let factory = s.evaluate(prelude_fn, &[]).unwrap();
        let args = [
            s.get(bridge),
            s.str_val(tools_json),
            s.str_val("[]"),
            s.str_val("{}"),
        ];
        let f = s.get(factory);
        let api = s.invoke(f, Value::UNDEFINED, &args).unwrap();
        let member = |s: &mut Session, name: &str| {
            let k = s.str_val(name);
            let g = s.get(getter);
            let a = s.get(api);
            s.evaluate(g, &[a, k]).unwrap()
        };
        s.settle = member(&mut s, "settle");
        s.stalled = member(&mut s, "stalled");
        s.run = member(&mut s, "run");
        s.api = api;
        s.set_origin = set_origin;
        for h in [bridge, getter, factory] {
            s.vm.heap_mut().drop_handle(h);
        }
        s
    }

    /// From a prepared session: seed, origin, compile and start the script.
    pub fn start(&mut self, code: &str, seed: u64) {
        self.vm.set_random_seed(seed);
        let origin = Value::number(self.now);
        let so = self.get(self.set_origin);
        let r = self.invoke(so, Value::UNDEFINED, &[origin]).unwrap();
        self.vm.heap_mut().drop_handle(r);
        let src = format!("{SCRIPT_PREFIX}{code}\n}})");
        let f = self.compile("codemode.js", &[], &src);
        let fh = self.h(f);
        let fnv = self.evaluate(f, &[]).unwrap();
        self.vm.heap_mut().drop_handle(fh);
        let run = self.get(self.run);
        let api = self.get(self.api);
        let fv = self.get(fnv);
        let r = self.invoke(run, api, &[fv]).unwrap();
        self.vm.heap_mut().drop_handle(r);
        self.vm.heap_mut().drop_handle(fnv);
        self.drain();
    }

    pub fn drain(&mut self) {
        let st = self.get(self.stalled);
        let api = self.get(self.api);
        let r = self.invoke(st, api, &[]).unwrap();
        self.vm.heap_mut().drop_handle(r);
    }

    /// pi's `settle(id, ok, payload)` then a drain.
    pub fn deliver(&mut self, id: u32, ok: bool, payload: &str, now: f64) {
        self.now = self.now.max(now);
        let p = self.str_val(payload);
        let settle = self.get(self.settle);
        let api = self.get(self.api);
        let r = self
            .invoke(settle, api, &[Value::int(id as i32), Value::bool(ok), p])
            .unwrap();
        self.vm.heap_mut().drop_handle(r);
        self.drain();
    }
}

impl Session {
    /// A session over a raw image of a prepared session (`like` gives the handles).
    pub fn from_image(img: &[u8], like: &Session) -> Session {
        Session {
            vm: Vm::from_bytes(img.to_vec(), opts()).unwrap(),
            api: like.api,
            settle: like.settle,
            stalled: like.stalled,
            run: like.run,
            set_origin: like.set_origin,
            ev: Events::default(),
            now: like.now,
        }
    }
}
