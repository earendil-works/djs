//! Interpreter tests on hand-assembled bytecode (docs/compiler.md 7). End-to-end tests
//! through the compiler come with 0014.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};

use super::*;
use crate::bytecode::template::{TemplateParts, alloc_template};
use crate::bytecode::{CodeBuf, Handler, Label};
use crate::heap::{Atom, PropFlags, PropertyDescriptor};
use crate::interp::run::STACK_TRACE_LIMIT;

const LOG: u32 = 1;

/// A function under construction: code plus the parts of its template.
#[derive(Default)]
struct F {
    b: CodeBuf,
    consts: Vec<Value>,
    children: Vec<Ref>,
    captures: Vec<u32>,
    handlers: Vec<(Label, Label, Label, u32)>,
    names: Vec<Atom>,
    params: u16,
    locals: u16,
}

impl F {
    fn new() -> F {
        F::default()
    }

    fn build(self, vm: &mut Vm) -> Ref {
        let code = self.b.finish().unwrap();
        let handlers: Vec<Handler> = self
            .handlers
            .iter()
            .map(|&(s, e, t, depth)| Handler {
                start: code.label_pos(s).unwrap(),
                end: code.label_pos(e).unwrap(),
                target: code.label_pos(t).unwrap(),
                depth,
            })
            .collect();
        let parts = TemplateParts {
            params: self.params,
            locals: self.locals,
            captures: &self.captures,
            code: &code.code,
            consts: &self.consts,
            children: &self.children,
            handlers: &handlers,
            names: &self.names,
            ..Default::default()
        };
        alloc_template(vm.heap_mut(), &parts).unwrap()
    }

    fn string(&mut self, vm: &mut Vm, s: &str) -> u32 {
        let v = Value::string(vm.heap_mut().alloc_str(s).unwrap());
        self.consts.push(v);
        self.consts.len() as u32 - 1
    }
}

fn vm_with(opts: VmOptions) -> Vm {
    let mut vm = Vm::new(opts).unwrap();
    let console = vm.new_object().unwrap();
    let log = vm.new_host_function(LOG, "log").unwrap();
    vm.define(console, "log", log).unwrap();
    vm.define_global("console", Value::object(console)).unwrap();
    vm
}

fn vm() -> Vm {
    vm_with(VmOptions::default())
}

/// The outcome of a run: the completion (as displayed, or `throw ...`) and console output.
#[derive(Debug, PartialEq)]
struct Outcome {
    result: String,
    log: Vec<String>,
}

/// Runs until the call finishes, serving `console.log` and rebuilding the VM from its
/// image whenever it runs out of fuel when `suspend` is set.
fn finish(mut vm: Vm, fuel: u64, suspend: bool) -> (Vm, Outcome) {
    let mut log = Vec::new();
    loop {
        match vm.run(fuel) {
            Status::Done(v) => {
                let result = vm.display(v);
                return (vm, Outcome { result, log });
            }
            Status::Threw(v) => {
                let result = format!("throw {}", vm.display(v));
                return (vm, Outcome { result, log });
            }
            Status::OutOfFuel => {
                if suspend {
                    let opts = vm.options();
                    vm = Vm::from_bytes(vm.bytes().to_vec(), opts).unwrap();
                }
            }
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, LOG);
                let info = vm.host_call(call_id).unwrap();
                let line: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                log.push(line.join(" "));
                if suspend {
                    let opts = vm.options();
                    vm = Vm::from_bytes(vm.bytes().to_vec(), opts).unwrap();
                }
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle => panic!("idle"),
        }
    }
}

/// Builds `main` with `f`, calls it, and runs it to the end in several modes, checking
/// they all agree: plenty of fuel, one unit of fuel at a time with suspension through the
/// image, and GC stress.
fn run_main(build: impl Fn(&mut Vm) -> Ref) -> Outcome {
    let mut outcomes = Vec::new();
    for (fuel, suspend, gc_stress) in [(1 << 40, false, false), (1, true, false), (7, false, true)]
    {
        let mut vm = vm_with(VmOptions {
            exec: ExecOptions {
                gc_stress,
                ..Default::default()
            },
            ..Default::default()
        });
        let t = build(&mut vm);
        let main = vm.instantiate(t).unwrap();
        vm.call(main, Value::UNDEFINED, &[]).unwrap();
        let (vm, out) = finish(vm, fuel, suspend);
        vm.heap().verify().unwrap();
        outcomes.push(out);
    }
    assert_eq!(outcomes[0], outcomes[1], "suspension changed the outcome");
    assert_eq!(outcomes[0], outcomes[2], "gc stress changed the outcome");
    outcomes.remove(0)
}

fn result_of(build: impl Fn(&mut Vm) -> Ref) -> String {
    run_main(build).result
}

#[test]
fn arithmetic() {
    let r = result_of(|vm| {
        let mut f = F::new();
        f.b.push_i32(2);
        f.b.push_i32(3);
        f.b.add();
        f.b.push_i8(4);
        f.b.mul();
        f.b.return_();
        f.build(vm)
    });
    assert_eq!(r, "20");
    let r = result_of(|vm| {
        let mut f = F::new();
        f.b.push_i32(i32::MAX);
        f.b.push_i8(1);
        f.b.add();
        f.b.push_i8(3);
        f.b.div();
        f.b.return_();
        f.build(vm)
    });
    assert_eq!(r, "715827882.6666666");
}

/// One binary operator on two constants.
fn binop(a: Value, b: Value, op: fn(&mut CodeBuf)) -> String {
    let mut vm = vm();
    let mut f = F::new();
    f.consts = vec![a, b];
    f.b.push_const(0);
    f.b.push_const(1);
    op(&mut f.b);
    f.b.return_();
    let t = f.build(&mut vm);
    let main = vm.instantiate(t).unwrap();
    vm.call(main, Value::UNDEFINED, &[]).unwrap();
    finish(vm, 1 << 30, false).1.result
}

#[test]
fn operators() {
    let n = Value::number;
    let i = Value::int;
    type Case = (Value, Value, fn(&mut CodeBuf), &'static str);
    let cases: Vec<Case> = vec![
        (n(0.1), n(0.2), CodeBuf::add, "0.30000000000000004"),
        (i(7), i(-2), CodeBuf::mod_, "1"),
        (i(-7), i(2), CodeBuf::mod_, "-1"),
        (n(5.5), i(2), CodeBuf::mod_, "1.5"),
        (i(2), i(10), CodeBuf::pow, "1024"),
        (i(2), n(0.5), CodeBuf::pow, "1.4142135623730951"),
        (i(1), n(f64::INFINITY), CodeBuf::pow, "NaN"),
        (i(1), i(0), CodeBuf::div, "Infinity"),
        (i(-1), i(28), CodeBuf::shl, "-268435456"),
        (i(-16), i(2), CodeBuf::sar, "-4"),
        (i(-16), i(28), CodeBuf::shr, "15"),
        (i(-1), i(0), CodeBuf::shr, "4294967295"),
        (i(6), i(3), CodeBuf::band, "2"),
        (i(6), i(3), CodeBuf::bor, "7"),
        (i(6), i(3), CodeBuf::bxor, "5"),
        (i(1), i(2), CodeBuf::lt, "true"),
        (i(2), i(2), CodeBuf::le, "true"),
        (i(1), i(2), CodeBuf::gt, "false"),
        (n(f64::NAN), i(2), CodeBuf::ge, "false"),
        (n(f64::NAN), i(2), CodeBuf::lt, "false"),
        (i(1), n(1.0), CodeBuf::strict_eq, "true"),
        (n(f64::NAN), n(f64::NAN), CodeBuf::strict_ne, "true"),
        (Value::NULL, Value::UNDEFINED, CodeBuf::eq, "true"),
        (Value::NULL, Value::UNDEFINED, CodeBuf::strict_eq, "false"),
        (Value::TRUE, i(1), CodeBuf::eq, "true"),
        (Value::TRUE, i(2), CodeBuf::ne, "true"),
    ];
    for (a, b, op, want) in cases {
        assert_eq!(binop(a, b, op), want, "{a:?} {b:?}");
    }
}

#[test]
fn strings() {
    let r = run_main(|vm| {
        let mut f = F::new();
        let a = f.string(vm, "ab");
        let n = f.string(vm, "42");
        let long = f.string(vm, &"x".repeat(40));
        f.b.push_const(a);
        f.b.push_i8(1);
        f.b.add(); // "ab1"
        f.b.push_const(long);
        f.b.add(); // a rope
        f.b.get_field(crate::heap::atoms::LENGTH);
        f.b.push_const(n);
        f.b.add(); // 43 + "42" -> "4342"
        f.b.push_const(n);
        f.b.push_i8(42);
        f.b.eq(); // "42" == 42
        f.b.add(); // "4342true"
        f.b.push_const(a);
        f.b.push_const(n);
        f.b.lt(); // "ab" < "42": false
        f.b.add();
        f.b.null();
        f.b.typeof_();
        f.b.add();
        f.b.return_();
        f.build(vm)
    });
    assert_eq!(r.result, "4342truefalseobject");
}

#[test]
fn loops_and_fuel() {
    // sum = 0; for (i = 0; i < 100; i++) sum += i; return sum
    let build = |vm: &mut Vm| {
        let mut f = F::new();
        f.locals = 2;
        let (top, end) = (f.b.new_label(), f.b.new_label());
        f.b.push_i8(0);
        f.b.put_loc(0);
        f.b.push_i8(0);
        f.b.put_loc(1);
        f.b.place_label(top);
        f.b.get_loc(1);
        f.b.push_i8(100);
        f.b.lt();
        f.b.jump_if_false(end);
        f.b.get_loc(0);
        f.b.get_loc(1);
        f.b.add();
        f.b.put_loc(0);
        f.b.get_loc(1);
        f.b.inc();
        f.b.put_loc(1);
        f.b.jump(top);
        f.b.place_label(end);
        f.b.get_loc(0);
        f.b.return_();
        f.build(vm)
    };
    assert_eq!(result_of(build), "4950");
    // Out of fuel: resumes exactly where it stopped.
    let mut vm = vm();
    let t = build(&mut vm);
    let main = vm.instantiate(t).unwrap();
    vm.call(main, Value::UNDEFINED, &[]).unwrap();
    let mut slices = 0;
    loop {
        match vm.run(50) {
            Status::OutOfFuel => slices += 1,
            Status::Done(v) => {
                assert_eq!(v, Value::int(4950));
                break;
            }
            s => panic!("{s:?}"),
        }
    }
    assert!(slices > 20, "{slices}");
}

#[test]
fn console_log_is_a_host_call() {
    let out = run_main(|vm| {
        let mut f = F::new();
        let console = vm.heap_mut().atom_str("console").unwrap();
        let log = vm.heap_mut().atom_str("log").unwrap();
        let hi = f.string(vm, "hi");
        f.b.get_global(console);
        f.b.get_method(log);
        f.b.push_const(hi);
        f.b.push_i8(42);
        f.b.null();
        f.b.call(3);
        f.b.drop();
        f.b.get_global(console);
        f.b.get_method(log);
        f.b.object();
        f.b.push_i8(1);
        f.b.define_field(crate::heap::atoms::VALUE);
        f.b.push_i8(1);
        f.b.push_i8(2);
        f.b.array_from(2);
        f.b.call(2);
        f.b.return_();
        f.build(vm)
    });
    assert_eq!(out.log, ["hi 42 null", "{\"value\":1} [1,2]"]);
    assert_eq!(out.result, "undefined");
}

#[test]
fn host_call_results_and_errors() {
    let mut vm = vm();
    let host = vm.new_host_function(7, "h").unwrap();
    vm.define_global("h", host).unwrap();
    let h = vm.heap_mut().atom_str("h").unwrap();
    // try { return h(1) + 1 } catch (e) { return e }
    let mut f = F::new();
    let (s, e, t) = (f.b.new_label(), f.b.new_label(), f.b.new_label());
    f.b.place_label(s);
    f.b.get_global(h);
    f.b.undefined();
    f.b.push_i8(1);
    f.b.call(1);
    f.b.push_i8(1);
    f.b.add();
    f.b.return_();
    f.b.place_label(e);
    f.b.place_label(t);
    f.b.return_();
    f.handlers.push((s, e, t, 0));
    let tpl = f.build(&mut vm);
    let main = vm.instantiate(tpl).unwrap();
    for (result, want) in [
        (Ok(Value::int(41)), Value::int(42)),
        (Err(Value::int(-1)), Value::int(-1)),
    ] {
        vm.call(main, Value::UNDEFINED, &[]).unwrap();
        let Status::HostCall { call_id, fn_id } = vm.run(1000) else {
            panic!()
        };
        assert_eq!(fn_id, 7);
        assert_eq!(vm.host_call(call_id).unwrap().args, vec![Value::int(1)]);
        // Running again while parked reports the same call.
        assert_eq!(vm.run(1000), Status::HostCall { call_id, fn_id });
        assert_eq!(vm.call(main, Value::UNDEFINED, &[]), Err(VmError::Busy));
        assert_eq!(
            vm.resolve_host_call(call_id + 1, Ok(Value::NULL)),
            Err(VmError::UnknownHostCall)
        );
        vm.resolve_host_call(call_id, result).unwrap();
        assert_eq!(vm.run(1000), Status::Done(want));
    }
    // Calling a host function directly from the embedder.
    vm.call(host, Value::NULL, &[Value::TRUE]).unwrap();
    let Status::HostCall { call_id, .. } = vm.run(10) else {
        panic!()
    };
    assert_eq!(vm.host_call(call_id).unwrap().this, Value::NULL);
    vm.resolve_host_call(call_id, Ok(Value::int(5))).unwrap();
    assert_eq!(vm.run(10), Status::Done(Value::int(5)));
}

fn error_of(build: impl Fn(&mut Vm, &mut F)) -> String {
    result_of(|vm| {
        let mut f = F::new();
        build(vm, &mut f);
        f.b.return_undef();
        f.build(vm)
    })
}

#[test]
fn runtime_errors() {
    let r = error_of(|vm, f| {
        let a = vm.heap_mut().atom_str("nope").unwrap();
        f.b.get_global(a);
    });
    assert_eq!(r, "throw ReferenceError: 'nope' is not defined");
    let r = error_of(|vm, f| {
        let a = vm.heap_mut().atom_str("nope").unwrap();
        f.b.push_i8(1);
        f.b.put_global(a);
    });
    assert_eq!(r, "throw ReferenceError: 'nope' is not defined");
    let r = error_of(|_, f| {
        f.b.undefined();
        f.b.get_field(crate::heap::atoms::NAME);
    });
    assert_eq!(
        r,
        "throw TypeError: cannot read property 'name' of undefined"
    );
    let r = error_of(|_, f| {
        f.b.null();
        f.b.push_i8(0);
        f.b.get_elem();
    });
    assert_eq!(r, "throw TypeError: cannot read property '0' of null");
    let r = error_of(|_, f| {
        f.b.undefined();
        f.b.push_i8(1);
        f.b.put_field(crate::heap::atoms::NAME);
    });
    assert_eq!(
        r,
        "throw TypeError: cannot set property 'name' of undefined"
    );
    let r = error_of(|_, f| {
        f.b.push_i8(1);
        f.b.undefined();
        f.b.call(0);
    });
    assert_eq!(r, "throw TypeError: not a function");
    let r = error_of(|vm, f| {
        let c = vm.heap_mut().atom_str("c").unwrap();
        f.b.throw_const_assign(c);
    });
    assert_eq!(r, "throw TypeError: 'c' is read-only");
    let r = error_of(|_, f| {
        f.b.push_i8(1);
        f.b.push_i8(2);
        f.b.in_();
    });
    assert_eq!(r, "throw TypeError: invalid 'in' operand");
    let r = error_of(|_, f| {
        f.b.push_i8(1);
        f.b.push_i8(2);
        f.b.instanceof();
    });
    assert_eq!(r, "throw TypeError: invalid 'instanceof' right operand");
    let r = error_of(|vm, f| {
        f.locals = 1;
        f.names = vec![vm.heap_mut().atom_str("x").unwrap()];
        f.b.set_loc_uninit(0);
        f.b.get_loc_chk(0);
    });
    assert_eq!(r, "throw ReferenceError: x is not initialized");
    let r = error_of(|_, f| {
        f.b.object();
        f.b.push_i8(1);
        f.b.add();
    });
    // %ToPrimitive now finds Object.prototype.toString (0031): no error, and error_of
    // reports the (discarded) completion.
    assert_eq!(r, "undefined");
    let r = error_of(|_, f| {
        f.b.push_i8(1);
        f.b.throw();
    });
    assert_eq!(r, "throw 1");
}

#[test]
fn objects_and_globals() {
    let r = result_of(|vm| {
        let mut f = F::new();
        let x = vm.heap_mut().atom_str("x").unwrap();
        let g = vm.heap_mut().atom_str("counter").unwrap();
        // globalThis.counter = 1; counter = counter + 1; o = {}; o.x = counter; o[5] = 3;
        // return [o.x, o[5], typeof o, "x" in o, counter]
        f.locals = 1;
        f.b.get_global(crate::heap::atoms::GLOBAL_THIS);
        f.b.push_i8(1);
        f.b.put_field(g);
        f.b.get_global(g);
        f.b.push_i8(1);
        f.b.add();
        f.b.put_global(g);
        f.b.object();
        f.b.put_loc(0);
        f.b.get_loc(0);
        f.b.get_global(g);
        f.b.put_field(x);
        f.b.get_loc(0);
        f.b.push_i8(5);
        f.b.push_i8(3);
        f.b.put_elem();
        f.b.get_loc(0);
        f.b.get_field(x);
        f.b.get_loc(0);
        f.b.push_i8(5);
        f.b.get_elem();
        f.b.get_loc(0);
        f.b.typeof_();
        f.b.push_atom(x);
        f.b.get_loc(0);
        f.b.in_();
        f.b.get_global_typeof(g);
        f.b.array_from(5);
        f.b.return_();
        f.build(vm)
    });
    assert_eq!(r, "[2,3,\"object\",true,2]");
}

/// `function mul(a, b) { return a * b }`
fn mul_template(vm: &mut Vm) -> Ref {
    let mut f = F::new();
    f.params = 2;
    f.b.get_arg(0);
    f.b.get_arg(1);
    f.b.mul();
    f.b.return_();
    f.build(vm)
}

#[test]
fn js_calls_and_argument_padding() {
    let r = result_of(|vm| {
        let child = mul_template(vm);
        let mut f = F::new();
        f.children = vec![child];
        f.locals = 1;
        f.b.fclosure(0);
        f.b.put_loc(0);
        f.b.get_loc(0);
        f.b.undefined();
        f.b.push_i8(6);
        f.b.push_i8(7);
        f.b.call(2); // 42
        f.b.get_loc(0);
        f.b.undefined();
        f.b.push_i8(6);
        f.b.call(1); // NaN
        f.b.get_loc(0);
        f.b.undefined();
        f.b.push_i8(2);
        f.b.push_i8(3);
        f.b.push_i8(4);
        f.b.call(3); // 6, extra argument ignored
        f.b.array_from(3);
        f.b.return_();
        f.build(vm)
    });
    assert_eq!(r, "[42,null,6]");
}

#[test]
fn closures_share_boxes() {
    // function counter() { let n = 10; return () => ++n }
    let mut vm = vm();
    let mut inner = F::new();
    inner.captures = vec![0];
    inner.b.get_cap(0);
    inner.b.inc();
    inner.b.set_cap(0);
    inner.b.return_();
    let inner = inner.build(&mut vm);
    let mut outer = F::new();
    outer.locals = 1;
    outer.children = vec![inner];
    outer.b.new_box(0);
    outer.b.push_i8(10);
    outer.b.put_box(0);
    outer.b.fclosure(0);
    outer.b.return_();
    let outer = outer.build(&mut vm);
    let counter = vm.instantiate(outer).unwrap();
    vm.call(counter, Value::UNDEFINED, &[]).unwrap();
    let Status::Done(f) = vm.run(100) else {
        panic!()
    };
    let hf = vm.heap_mut().new_handle(f).unwrap();
    for want in [11, 12, 13] {
        vm.collect().unwrap();
        let f = vm.heap().get_handle(hf);
        vm.call(f, Value::UNDEFINED, &[]).unwrap();
        assert_eq!(vm.run(100), Status::Done(Value::int(want)));
    }
    vm.heap().verify().unwrap();
}

#[test]
fn accessors_run_through_continuations() {
    // o has a getter (a JS closure returning this.v * 2) and a setter (stores v + 1).
    let r = run_main(|vm| {
        let v = vm.heap_mut().atom_str("v").unwrap();
        let p = vm.heap_mut().atom_str("p").unwrap();
        let mut getter = F::new();
        getter.b.get_global(crate::heap::atoms::GLOBAL_THIS);
        getter.b.get_field(v);
        getter.b.push_i8(2);
        getter.b.mul();
        getter.b.return_();
        let getter = getter.build(vm);
        let mut setter = F::new();
        setter.params = 1;
        setter.b.get_global(crate::heap::atoms::GLOBAL_THIS);
        setter.b.get_arg(0);
        setter.b.push_i8(1);
        setter.b.add();
        setter.b.put_field(v);
        setter.b.push_i8(99);
        setter.b.return_(); // the setter's result is discarded
        let setter = setter.build(vm);
        let gf = vm.instantiate(getter).unwrap();
        let sf = vm.instantiate(setter).unwrap();
        let g = vm.global();
        vm.heap_mut()
            .create_data_property(g, v, Value::int(5))
            .unwrap();
        vm.heap_mut()
            .define_own_property(g, p, &PropertyDescriptor::accessor(gf, sf, PropFlags::ALL))
            .unwrap();
        // return [p, (p = 20, p), v]
        let mut f = F::new();
        f.b.get_global(p);
        f.b.get_global(crate::heap::atoms::GLOBAL_THIS);
        f.b.push_i8(20);
        f.b.put_field(p);
        f.b.get_global(p);
        f.b.get_global(v);
        f.b.array_from(3);
        f.b.return_();
        f.build(vm)
    });
    assert_eq!(r.result, "[10,42,21]");
}

#[test]
fn exceptions_unwind_frames() {
    // function thrower() { null.x }   main: try { thrower() } catch (e) { return "caught " + e.message }
    let r = result_of(|vm| {
        let mut t = F::new();
        t.b.null();
        t.b.get_field(crate::heap::atoms::NAME);
        t.b.return_();
        let t = t.build(vm);
        let mut f = F::new();
        f.children = vec![t];
        let caught = f.string(vm, "caught ");
        let (s, e, h) = (f.b.new_label(), f.b.new_label(), f.b.new_label());
        f.b.place_label(s);
        f.b.push_i8(1); // an operand below the call, dropped by the unwind
        f.b.fclosure(0);
        f.b.undefined();
        f.b.call(0);
        f.b.return_();
        f.b.place_label(e);
        f.b.place_label(h);
        f.b.get_field(crate::heap::atoms::MESSAGE);
        f.b.push_const(caught);
        f.b.swap();
        f.b.add();
        f.b.return_();
        f.handlers.push((s, e, h, 0));
        f.build(vm)
    });
    assert_eq!(r, "caught cannot read property 'name' of null");
}

#[test]
fn finally_with_gosub() {
    // try { return 1 } finally { log("fin") }
    let out = run_main(|vm| {
        let mut f = F::new();
        let console = vm.heap_mut().atom_str("console").unwrap();
        let log = vm.heap_mut().atom_str("log").unwrap();
        let fin_s = f.string(vm, "fin");
        let fin = f.b.new_label();
        f.b.push_i8(1);
        f.b.gosub(fin);
        f.b.return_();
        f.b.place_label(fin);
        f.b.get_global(console);
        f.b.get_method(log);
        f.b.push_const(fin_s);
        f.b.call(1);
        f.b.drop();
        f.b.ret();
        f.build(vm)
    });
    assert_eq!(out.log, ["fin"]);
    assert_eq!(out.result, "1");
}

#[test]
fn stack_overflow() {
    let mut vm = vm_with(VmOptions {
        exec: ExecOptions {
            max_stack: 10_000,
            ..Default::default()
        },
        ..Default::default()
    });
    // function f() { return f() }
    let name = vm.heap_mut().atom_str("f").unwrap();
    let mut f = F::new();
    f.b.get_global(name);
    f.b.undefined();
    f.b.call(0);
    f.b.return_();
    let t = f.build(&mut vm);
    let func = vm.instantiate(t).unwrap();
    vm.define_global("f", func).unwrap();
    vm.call(func, Value::UNDEFINED, &[]).unwrap();
    let (vm, out) = finish(vm, 1 << 30, false);
    assert_eq!(out.result, "throw InternalError: stack overflow");
    // The VM is usable again.
    let mut vm = vm;
    let t = mul_template(&mut vm);
    let m = vm.instantiate(t).unwrap();
    vm.call(m, Value::UNDEFINED, &[Value::int(3), Value::int(4)])
        .unwrap();
    assert_eq!(vm.run(100), Status::Done(Value::int(12)));
}

#[test]
fn garbage_is_collected_while_running() {
    // A loop that allocates a lot of garbage strings; the heap must stay small.
    let mut vm = vm();
    let mut f = F::new();
    f.locals = 1;
    let s = f.string(&mut vm, &"y".repeat(50));
    let (top, end) = (f.b.new_label(), f.b.new_label());
    f.b.push_i32(0);
    f.b.put_loc(0);
    f.b.place_label(top);
    f.b.get_loc(0);
    f.b.push_i32(20_000);
    f.b.lt();
    f.b.jump_if_false(end);
    f.b.push_const(s);
    f.b.get_loc(0);
    f.b.add();
    f.b.drop();
    f.b.get_loc(0);
    f.b.inc();
    f.b.put_loc(0);
    f.b.jump(top);
    f.b.place_label(end);
    f.b.get_loc(0);
    f.b.return_();
    let t = f.build(&mut vm);
    let main = vm.instantiate(t).unwrap();
    vm.call(main, Value::UNDEFINED, &[]).unwrap();
    assert_eq!(vm.run(1 << 30), Status::Done(Value::int(20_000)));
    // 20k ropes of ~100 bytes each would be 2 MB without collections.
    assert!(
        vm.heap().arena().len() < 1 << 20,
        "{}",
        vm.heap().arena().len()
    );
}

#[test]
fn image_while_parked() {
    // Parked on a host call: snapshot, restore elsewhere, resolve there.
    let mut vm = vm();
    let mut f = F::new();
    let console = vm.heap_mut().atom_str("console").unwrap();
    let log = vm.heap_mut().atom_str("log").unwrap();
    f.b.get_global(console);
    f.b.get_method(log);
    f.b.push_i8(1);
    f.b.call(1);
    f.b.push_i8(1);
    f.b.add();
    f.b.return_();
    let t = f.build(&mut vm);
    let main = vm.instantiate(t).unwrap();
    vm.call(main, Value::UNDEFINED, &[]).unwrap();
    let Status::HostCall { call_id, .. } = vm.run(100) else {
        panic!()
    };
    let mut moved = Vm::from_bytes(vm.bytes().to_vec(), VmOptions::default()).unwrap();
    drop(vm);
    assert_eq!(moved.state(), RunState::HostCall);
    moved
        .resolve_host_call(call_id, Ok(Value::int(41)))
        .unwrap();
    assert_eq!(moved.run(100), Status::Done(Value::int(42)));
}

#[test]
fn abort_returns_to_idle() {
    // An endless loop, aborted after running out of fuel.
    let mut vm = vm();
    let mut f = F::new();
    let top = f.b.new_label();
    f.b.place_label(top);
    f.b.push_i8(1);
    f.b.drop();
    f.b.jump(top);
    let t = f.build(&mut vm);
    let spin = vm.instantiate(t).unwrap();
    let hs = vm.heap_mut().new_handle(spin).unwrap();
    vm.call(spin, Value::UNDEFINED, &[]).unwrap();
    assert_eq!(vm.run(100), Status::OutOfFuel);
    assert_eq!(vm.call(spin, Value::UNDEFINED, &[]), Err(VmError::Busy));
    vm.abort();
    assert_eq!(vm.state(), RunState::Idle);
    assert_eq!(vm.run(100), Status::Idle);
    // Usable again.
    let t = mul_template(&mut vm);
    let m = vm.instantiate(t).unwrap();
    vm.call(m, Value::UNDEFINED, &[Value::int(6), Value::int(7)])
        .unwrap();
    assert_eq!(vm.run(100), Status::Done(Value::int(42)));

    // Aborting while parked on a host call drops the call.
    let host = vm.new_host_function(9, "h").unwrap();
    vm.call(host, Value::UNDEFINED, &[]).unwrap();
    let Status::HostCall { call_id, .. } = vm.run(10) else {
        panic!()
    };
    vm.abort();
    assert!(vm.host_call(call_id).is_none());
    assert_eq!(
        vm.resolve_host_call(call_id, Ok(Value::NULL)),
        Err(VmError::UnknownHostCall)
    );
    vm.collect().unwrap();
    vm.heap().verify().unwrap();
    // The aborted closure is still usable through its handle.
    let spin = vm.heap().get_handle(hs);
    vm.call(spin, Value::UNDEFINED, &[]).unwrap();
    assert_eq!(vm.run(10), Status::OutOfFuel);
}

/// Runs a Script through the compiler, collecting console.log output.
fn run_js(src: &str) -> Outcome {
    let mut vm = vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("t.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    finish(vm, 1 << 40, false).1
}

#[test]
fn error_stack_traces() {
    // Positions checked against qjs.
    let out = run_js(
        "function inner(x) { return x.y.z; }\n\
         const arrow = () => { const r = inner({}); return r; };\n\
         function outer() { const r = arrow(); return r; }\n\
         try { outer(); } catch (e) { console.log(e.stack); }\n\
         try { null.x } catch (e) { console.log(e.stack); }\n\
         const anon = function () { return nope; };\n\
         try { anon(); } catch (e) { console.log(e.stack); }\n\
         function deep(n) { if (n === 0) return null.x; const r = deep(n - 1); return r; }\n\
         try { deep(20); } catch (e) { console.log(e.stack); }\n",
    );
    assert_eq!(
        out.log[0],
        "    at inner (t.js:1:31)\n    at arrow (t.js:2:38)\n    at outer (t.js:3:35)\n    at <eval> (t.js:4:12)\n"
    );
    assert_eq!(out.log[1], "    at <eval> (t.js:5:11)\n");
    assert_eq!(
        out.log[2],
        "    at anon (t.js:6:35)\n    at <eval> (t.js:7:11)\n"
    );
    let deep = &out.log[3];
    assert_eq!(deep.lines().count(), STACK_TRACE_LIMIT as usize, "{deep}");
}

/// `Vm::new` with and without the prelude. Run with
/// `cargo test -p djs-core --release vm_new_cost -- --ignored --nocapture`.
#[test]
#[ignore]
fn vm_new_cost() {
    extern crate std;
    use std::time::Instant;
    for prelude in [false, true] {
        let opts = VmOptions {
            prelude,
            ..Default::default()
        };
        let n = 200;
        let t = Instant::now();
        let mut size = 0;
        for _ in 0..n {
            let vm = Vm::new(opts).unwrap();
            size = vm.bytes().len();
        }
        let per = t.elapsed().as_secs_f64() / n as f64;
        std::println!(
            "Vm::new prelude={prelude}: {:.1} µs, heap {size} bytes",
            per * 1e6
        );
    }
    // What a base snapshot costs: the image after a collect, restored with full
    // verification, and a plain copy.
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    vm.collect().unwrap();
    let image = vm.bytes().to_vec();
    let n = 2000;
    let t = Instant::now();
    for _ in 0..n {
        let vm = Vm::from_bytes(image.clone(), VmOptions::default()).unwrap();
        std::hint::black_box(vm.bytes().len());
    }
    let per = t.elapsed().as_secs_f64() / n as f64;
    let t = Instant::now();
    for _ in 0..n {
        std::hint::black_box(image.clone());
    }
    let copy = t.elapsed().as_secs_f64() / n as f64;
    std::println!(
        "image {} bytes: from_bytes {:.1} µs, copy {:.1} µs",
        image.len(),
        per * 1e6,
        copy * 1e6
    );
}

#[test]
fn method_call_through_a_getter() {
    // o.m is an accessor whose getter returns a function; `o.m()` calls that function
    // with this = o (get_method through the Replace continuation).
    let mut vm = vm();
    let src = "function getter() { return function () { return this.n + 1; }; }\n\
               function make() { return { n: 41 }; }";
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("g.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    assert!(matches!(vm.run(1 << 30), Status::Done(_)));
    let g = vm.global();
    let k = vm.heap_mut().atom_str("getter").unwrap();
    let getter = vm.heap().get_own_property(g, k).unwrap().value().unwrap();
    let o = vm.new_object().unwrap();
    let n = vm.heap_mut().atom_str("n").unwrap();
    vm.heap_mut()
        .create_data_property(o, n, Value::int(41))
        .unwrap();
    let m = vm.heap_mut().atom_str("m").unwrap();
    vm.heap_mut()
        .define_own_property(
            o,
            m,
            &PropertyDescriptor::accessor(getter, Value::UNDEFINED, PropFlags::ALL),
        )
        .unwrap();
    vm.define_global("obj", Value::object(o)).unwrap();
    let out = {
        let f = vm
            .compile(
                b"console.log(obj.m(), obj['m'](), typeof obj.m)",
                &crate::compiler::CompileOptions::script("g2.js"),
            )
            .unwrap();
        let this = Value::object(vm.global());
        vm.call(f, this, &[]).unwrap();
        finish(vm, 1 << 30, true).1
    };
    assert_eq!(out.log, ["42 42 function"]);
}

#[test]
fn to_primitive_through_a_host_valueof() {
    // valueOf is a host function: the slow path parks on it and the resolved value
    // replaces the operand.
    let mut vm = vm();
    let host = vm.new_host_function(5, "valueOf").unwrap();
    let o = vm.new_object().unwrap();
    vm.define(o, "valueOf", host).unwrap();
    vm.define_global("o", Value::object(o)).unwrap();
    let f = vm
        .compile(
            b"console.log(o + 1, o * 3)",
            &crate::compiler::CompileOptions::script("h.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut log = Vec::new();
    loop {
        match vm.run(1 << 30) {
            Status::HostCall { call_id, fn_id: 5 } => {
                // `o` re-read from the global: the round trips below may move it, so a
                // raw value from before goes stale (embedders hold handles instead).
                let key = vm.heap_mut().atom_str("o").unwrap();
                let g = vm.global();
                let o_now = match vm.heap().get_own_property(g, key) {
                    Some(crate::heap::Property::Data { value, .. }) => value,
                    _ => panic!("global o"),
                };
                assert_eq!(vm.host_call(call_id).unwrap().this, o_now);
                // Through an image round trip, for good measure.
                let opts = vm.options();
                vm = Vm::from_bytes(vm.bytes().to_vec(), opts).unwrap();
                vm.resolve_host_call(call_id, Ok(Value::int(10))).unwrap();
            }
            Status::HostCall { call_id, .. } => {
                let info = vm.host_call(call_id).unwrap();
                let parts: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                log.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Done(_) => break,
            s => panic!("{s:?}"),
        }
    }
    assert_eq!(log, ["11 30"]);
}

/// Runs `src` as a script and returns what it evaluates to (`return` value of a body).
fn eval(vm: &mut Vm, src: &str) -> String {
    let opts = crate::compiler::CompileOptions::async_body("t.js", &[]);
    let f = vm.compile(src.as_bytes(), &opts).unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    match vm.run(1 << 30) {
        Status::Done(v) => vm.display(v),
        Status::Threw(e) => panic!("threw: {}", vm.display(e)),
        s => panic!("{s:?}"),
    }
}

/// A base image as djs-prelude makes it: a fresh VM with the prelude, collected.
fn base_image() -> Vec<u8> {
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    vm.collect().unwrap();
    vm.bytes().to_vec()
}

#[test]
fn from_base_gives_independent_vms() {
    let base = base_image();
    let mut a = Vm::from_base(&base, VmOptions::default()).unwrap();
    let mut b = Vm::from_base(&base, VmOptions::default()).unwrap();
    assert_eq!(
        eval(&mut a, "globalThis.x = 1; return [1, 2, 3].length + 1"),
        "4"
    );
    // The second VM does not see the first one's global.
    assert_eq!(eval(&mut b, "return typeof globalThis.x"), "undefined");
    // A VM from the base behaves like one that compiled its prelude.
    let mut c = Vm::new(VmOptions::default()).unwrap();
    let probe = "return typeof JSON + typeof Math + typeof Array + typeof String + typeof Object \
                 + typeof [].push + typeof ''.slice + [1, 2, 3].length";
    assert_eq!(eval(&mut b, probe), eval(&mut c, probe));
    // Host functions are bound after the copy, per VM.
    let h = a.new_host_function(7, "f").unwrap();
    a.define_global("f", h).unwrap();
    assert_eq!(eval(&mut b, "return typeof f"), "undefined");
}

#[test]
fn images_carry_the_build_hash() {
    let vm = Vm::new(VmOptions::default()).unwrap();
    let at = crate::heap::cells::Roots::BUILD.offset as usize;
    assert_eq!(vm.bytes()[at..at + 4], BUILD_HASH.to_le_bytes());
    let mut other = vm.bytes().to_vec();
    other[at] ^= 1;
    let expected = BUILD_HASH;
    let found = expected ^ 1;
    assert_eq!(
        Vm::from_bytes(other.clone(), VmOptions::default()).err(),
        Some(LoadError::BuildMismatch { found, expected })
    );
    assert_eq!(
        Vm::from_base(&other, VmOptions::default()).err(),
        Some(LoadError::BuildMismatch { found, expected })
    );
    let msg = alloc::format!("{}", LoadError::BuildMismatch { found, expected });
    assert!(msg.contains("another djs build"), "{msg}");
    // Not an image at all.
    assert_eq!(
        Vm::from_base(b"not an image", VmOptions::default()).err(),
        Some(LoadError::BadHeader)
    );
    // A bare heap (no VM) has no build hash.
    let bare = crate::heap::Heap::new().unwrap();
    assert!(matches!(
        Vm::from_bytes(bare.bytes().to_vec(), VmOptions::default()),
        Err(LoadError::BuildMismatch { found: 0, .. })
    ));
}

#[test]
fn from_base_needs_an_idle_vm_and_room() {
    // A VM suspended mid-call is a valid image for from_bytes, but not a base.
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    let opts = crate::compiler::CompileOptions::async_body("t.js", &[]);
    let f = vm.compile(b"for (;;) {}", &opts).unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    assert!(matches!(vm.run(1000), Status::OutOfFuel));
    let busy = vm.bytes().to_vec();
    assert!(Vm::from_bytes(busy.clone(), VmOptions::default()).is_ok());
    assert_eq!(
        Vm::from_base(&busy, VmOptions::default()).err(),
        Some(LoadError::BadHeader)
    );
    let small = VmOptions {
        memory_limit: 4096,
        ..VmOptions::default()
    };
    assert_eq!(
        Vm::from_base(&base_image(), small).err(),
        Some(LoadError::TooLarge)
    );
}

#[test]
fn math_random_is_seeded_per_vm_and_survives_snapshots() {
    let base = base_image();
    let seeded = |seed| VmOptions {
        random_seed: seed,
        ..Default::default()
    };
    let draw = "return [Math.random(), Math.random(), Math.random()].join()";
    let mut a = Vm::from_base(&base, seeded(42)).unwrap();
    let mut b = Vm::from_base(&base, seeded(42)).unwrap();
    let mut c = Vm::from_base(&base, seeded(43)).unwrap();
    let first = eval(&mut a, draw);
    assert_eq!(first, eval(&mut b, draw));
    assert_ne!(first, eval(&mut c, draw));
    // Vm::new applies the seed too.
    let mut d = Vm::new(seeded(42)).unwrap();
    assert_eq!(first, eval(&mut d, draw));
    // A snapshot continues the sequence instead of restarting it.
    let mut e = Vm::from_bytes(a.bytes().to_vec(), seeded(42)).unwrap();
    assert_eq!(eval(&mut a, draw), eval(&mut e, draw));
    let r = eval(&mut e, "const r = Math.random(); return r >= 0 && r < 1");
    assert_eq!(r, "true");
}

/// `acc = ""; for (k in <global name>) { acc += k + ","; [delete obj.<del> on the first
/// step] } return acc`, hand-assembled (the compiler side is 0029).
fn for_in_keys(vm: &mut Vm, target: &str, delete_on_first: Option<&str>) -> Value {
    let t = vm.heap_mut().atom_str(target).unwrap();
    let mut f = F::new();
    f.locals = 2;
    let comma = f.string(vm, ",");
    let empty = f.string(vm, "");
    let (top, end, skip) = (f.b.new_label(), f.b.new_label(), f.b.new_label());
    f.b.push_const(empty);
    f.b.put_loc(0);
    f.b.push_i8(0);
    f.b.put_loc(1);
    f.b.get_global(t);
    f.b.for_in_start();
    f.b.place_label(top);
    f.b.for_in_next();
    f.b.jump_if_true(end);
    f.b.get_loc(0);
    f.b.swap();
    f.b.add();
    f.b.push_const(comma);
    f.b.add();
    f.b.put_loc(0);
    if let Some(d) = delete_on_first {
        let d = vm.heap_mut().atom_str(d).unwrap();
        f.b.get_loc(1);
        f.b.jump_if_true(skip);
        f.b.get_global(t);
        f.b.push_atom(d);
        f.b.delete();
        f.b.drop();
        f.b.push_true();
        f.b.put_loc(1);
        f.b.place_label(skip);
    }
    f.b.jump(top);
    f.b.place_label(end);
    f.b.drop();
    f.b.drop();
    f.b.get_loc(0);
    f.b.return_();
    let tpl = f.build(vm);
    let main = vm.instantiate(tpl).unwrap();
    vm.call(main, Value::UNDEFINED, &[]).unwrap();
    let Status::Done(v) = vm.run(1 << 30) else {
        panic!()
    };
    v
}

#[test]
fn for_in_order_shadowing_and_deletion() {
    let mut vm = vm();
    let src = "var proto = { inherited: 1, shadowed: 2, gone: 3 };\n\
               var obj = { __proto__: proto, b: 1, 2: 0, a: 1, 1: 0 };";
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("fi.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    assert!(matches!(vm.run(1 << 30), Status::Done(_)));
    // A non-enumerable own `shadowed` hides the prototype's, and a symbol key is skipped.
    let g = vm.global();
    let obj_key = vm.heap_mut().atom_str("obj").unwrap();
    let obj = vm
        .heap()
        .get_own_property(g, obj_key)
        .unwrap()
        .value()
        .unwrap()
        .as_object()
        .unwrap();
    let sh = vm.heap_mut().atom_str("shadowed").unwrap();
    vm.heap_mut()
        .define_own_property(
            obj,
            sh,
            &PropertyDescriptor::data(Value::NULL, PropFlags::WRITABLE),
        )
        .unwrap();
    let sym = vm.heap_mut().new_symbol(Value::UNDEFINED).unwrap();
    let sk = vm.heap().symbol_atom(sym);
    vm.heap_mut()
        .create_data_property(obj, sk, Value::NULL)
        .unwrap();
    let v = for_in_keys(&mut vm, "obj", None);
    assert_eq!(vm.display(v), "1,2,b,a,inherited,gone,");
    // A key deleted during the loop is not visited.
    let v = for_in_keys(&mut vm, "obj", Some("a"));
    assert_eq!(vm.display(v), "1,2,b,inherited,gone,");
    // Strings give their indexes; null and numbers give nothing.
    let xyz = Value::string(vm.heap_mut().alloc_str("xyz").unwrap());
    vm.define_global("s", xyz).unwrap();
    vm.define_global("n", Value::NULL).unwrap();
    let v = for_in_keys(&mut vm, "s", None);
    assert_eq!(vm.display(v), "0,1,2,");
    let v = for_in_keys(&mut vm, "n", None);
    assert_eq!(vm.display(v), "");
}

#[test]
fn vm_call_never_collects() {
    // With GC stress, frame entry is a safepoint, but not for the entry call: the
    // embedder's raw values must stay valid until `run`.
    let mut vm = vm_with(VmOptions {
        exec: ExecOptions {
            gc_stress: true,
            ..Default::default()
        },
        ..Default::default()
    });
    let f = vm
        .compile(
            b"return 1 + 1",
            &crate::compiler::CompileOptions::async_body("c.js", &[]),
        )
        .unwrap();
    for _ in 0..100 {
        vm.heap_mut().alloc_str("garbage before the call").unwrap();
    }
    let len = vm.heap().arena().len();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    assert_eq!(vm.heap().arena().len(), len, "Vm::call must not collect");
    assert_eq!(
        vm.heap().cell_kind(f.as_object().unwrap().offset()),
        crate::heap::CellKind::Object
    );
    assert_eq!(vm.run(1 << 20), Status::Done(Value::int(2)));
    assert!(
        vm.heap().arena().len() < len,
        "run collects at its first safepoint"
    );
}

#[test]
fn date_uses_the_host_clock_and_offset() {
    let base = base_image();
    let mut vm = Vm::from_base(&base, VmOptions::default()).unwrap();
    // No clock set: the epoch (the VM never reads a system clock).
    assert_eq!(eval(&mut vm, "return Date.now()"), "0");
    vm.set_now(1_646_177_400_000.0);
    assert_eq!(
        eval(&mut vm, "return new Date().toISOString()"),
        "2022-03-01T23:30:00.000Z"
    );
    // UTC+2, checked against qjs with TZ=Etc/GMT-2.
    vm.set_time_zone_offset(120);
    let src = "const d = new Date(Date.UTC(2022, 2, 1, 23, 30));
        return [d.toString(), d.getDate(), d.getHours(), d.getTimezoneOffset(), d.toLocaleString(),
          new Date(2022, 0, 1).getTime(), Date.parse('2022-03-01T10:00'), Date.parse('2022-03-01'),
          new Date(2022, 2, 1).toISOString()].join('|')";
    assert_eq!(
        eval(&mut vm, src),
        "Wed Mar 02 2022 01:30:00 GMT+0200|2|1|-120|03/02/2022, 01:30:00 AM|1640988000000|\
         1646121600000|1646092800000|2022-02-28T22:00:00.000Z"
    );
}

const TOOL: u32 = 20;

/// Runs a Script with `console.log` and an async host function `tool(x)`, which the
/// "embedder" completes with `x.toUpperCase()` (or rejects for "fail"), resolving the
/// pending calls one at a time in reverse order whenever the VM is idle. With `suspend`,
/// every stop continues in a VM restored from the image; `gc_stress` collects at every
/// safepoint.
fn run_tools(src: &str, fuel: u64, suspend: bool, gc_stress: bool) -> Outcome {
    let mut vm = vm_with(VmOptions {
        exec: ExecOptions {
            gc_stress,
            ..Default::default()
        },
        ..Default::default()
    });
    let tool = vm.new_async_host_function(TOOL, "tool").unwrap();
    vm.define_global("tool", tool).unwrap();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("t.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut log = Vec::new();
    let mut pending: Vec<u32> = Vec::new();
    let reload = |vm: Vm| {
        let opts = vm.options();
        Vm::from_bytes(vm.bytes().to_vec(), opts).unwrap()
    };
    loop {
        let status = vm.run(fuel);
        for id in vm.new_host_calls() {
            let info = vm.host_call(id).unwrap();
            assert!(info.is_async);
            assert_eq!(info.fn_id, TOOL);
            pending.push(id);
        }
        match status {
            Status::Done(_) | Status::Threw(_) => {
                assert!(pending.is_empty());
                let result = match status {
                    Status::Done(v) => vm.display(v),
                    Status::Threw(e) => format!("throw {}", vm.display(e)),
                    _ => unreachable!(),
                };
                for r in vm.take_unhandled_rejections() {
                    log.push(format!("unhandled {}", vm.display(r)));
                }
                vm.heap().verify().unwrap();
                return Outcome { result, log };
            }
            Status::OutOfFuel => {}
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, LOG);
                let info = vm.host_call(call_id).unwrap();
                assert!(!info.is_async);
                let line: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                log.push(line.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle => {
                assert_eq!(vm.pending_host_calls(), pending);
                let id = pending.pop().expect("idle without pending calls");
                let arg = vm.host_call(id).unwrap().args[0];
                let s = vm.display(arg);
                let result = if s == "fail" {
                    Err(vm.new_error(ErrorKind::TypeError, "tool failed").unwrap())
                } else {
                    let v = vm.heap_mut().alloc_str(&s.to_uppercase()).unwrap();
                    Ok(Value::string(v))
                };
                log.push(format!("resolve {s}"));
                vm.resolve_host_call(id, result).unwrap();
            }
        }
        if suspend {
            vm = reload(vm);
        }
    }
}

fn tools_outcome(src: &str) -> Outcome {
    let plain = run_tools(src, 1 << 40, false, false);
    assert_eq!(
        plain,
        run_tools(src, 1, true, false),
        "suspension changed it"
    );
    assert_eq!(
        plain,
        run_tools(src, 13, true, true),
        "gc stress changed it"
    );
    plain
}

#[test]
fn async_host_calls_run_concurrently() {
    let out = tools_outcome(
        r#"
        const a = tool("a"), b = tool("b"), c = tool("c");
        console.log(Object.prototype.toString.call(a), a === b);
        Promise.all([a, b, c]).then(v => console.log("all", v.join()));
        b.then(v => console.log("b", v));
        Promise.resolve().then(() => console.log("microtask first"));
        console.log("sync end");
        "#,
    );
    assert_eq!(out.result, "undefined");
    assert_eq!(
        out.log,
        [
            "[object Promise] false",
            "sync end",
            "microtask first",
            "resolve c",
            "resolve b",
            "b B",
            "resolve a",
            "all A,B,C",
        ]
    );
}

#[test]
fn async_host_call_errors_and_thenables() {
    let out = tools_outcome(
        r#"
        tool("fail").catch(e => console.log("caught", e.name, e.message));
        tool("x").then(v => tool(v + "y")).then(v => console.log("chained", v));
        tool("fail");
        "#,
    );
    assert_eq!(
        out.log,
        [
            "resolve fail",
            "resolve x",
            "resolve Xy",
            "chained XY",
            "resolve fail",
            "caught TypeError tool failed",
            "unhandled TypeError: tool failed",
        ]
    );
}

#[test]
fn async_host_function_as_the_entry_callee() {
    let mut vm = vm();
    let tool = vm.new_async_host_function(TOOL, "tool").unwrap();
    vm.call(tool, Value::UNDEFINED, &[Value::int(1)]).unwrap();
    assert_eq!(vm.run(100), Status::Idle);
    let [id] = vm.new_host_calls()[..] else {
        panic!()
    };
    assert!(vm.new_host_calls().is_empty(), "reported once");
    vm.resolve_host_call(id, Ok(Value::int(5))).unwrap();
    let Status::Done(p) = vm.run(1000) else {
        panic!()
    };
    assert_eq!(vm.promise_state(p), Some(Some(Ok(Value::int(5)))));
    assert_eq!(vm.promise_state(Value::int(5)), None);
}

#[test]
fn image_with_pending_host_calls_and_jobs() {
    // Two async calls pending; one resolved but its job not run yet: the image carries
    // the pending call, the queued job and the reactions.
    let mut vm = vm();
    let tool = vm.new_async_host_function(TOOL, "tool").unwrap();
    vm.define_global("tool", tool).unwrap();
    let src = r#"
        const log = [];
        tool(1).then(v => log.push("one " + v));
        tool(2).then(v => { log.push("two " + v); console.log(log.join()); });
    "#;
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("t.js"),
        )
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    assert_eq!(vm.run(1 << 20), Status::Idle);
    let ids = vm.new_host_calls();
    assert_eq!(ids.len(), 2);
    vm.resolve_host_call(ids[0], Ok(Value::int(10))).unwrap();
    vm.collect().unwrap();
    let mut moved = Vm::from_bytes(vm.bytes().to_vec(), VmOptions::default()).unwrap();
    drop(vm);
    assert_eq!(moved.pending_host_calls(), [ids[1]]);
    assert!(moved.new_host_calls().is_empty(), "already reported");
    let info = moved.host_call(ids[1]).unwrap();
    assert_eq!(info.args, [Value::int(2)]);
    assert_eq!(moved.run(1 << 20), Status::Idle);
    moved.resolve_host_call(ids[1], Ok(Value::int(20))).unwrap();
    let (moved, out) = finish(moved, 1 << 20, true);
    assert_eq!(out.log, ["one 10,two 20"]);
    assert!(moved.pending_host_calls().is_empty());
    moved.heap().verify().unwrap();
}
