//! Interpreter tests on hand-assembled bytecode (docs/compiler.md 7). End-to-end tests
//! through the compiler come with 0014.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};

use super::*;
use crate::bytecode::template::{TemplateParts, alloc_template};
use crate::bytecode::{CodeBuf, Handler, Label};
use crate::heap::{Atom, PropFlags, PropertyDescriptor};

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
    assert_eq!(
        r,
        "throw TypeError: not supported yet: conversion of an object to a primitive"
    );
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
