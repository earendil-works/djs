//! The Unicode tables handed over (D3350): a VM without them built in, whose host gives
//! them at their first need, ends every program with the value, fuel and heap of a VM that
//! has them: for every way a property escape compiles, and for `normalize` and
//! `localeCompare`.

use alloc::string::String;
use alloc::vec::Vec;

use super::*;
use crate::compiler::CompileOptions;

/// A VM from a base image made once per test thread, elastic (a host that hands over).
fn vm() -> Vm {
    extern crate std;
    std::thread_local! {
        static BASE: Vec<u8> = {
            let mut vm = Vm::new(VmOptions::default()).unwrap();
            vm.prepare_base_image().unwrap();
            vm.bytes().to_vec()
        };
    }
    let mut vm = BASE
        .with(|b| Vm::from_base(b, VmOptions::default()))
        .unwrap();
    vm.set_elastic_stack(true);
    vm
}

/// Runs `src` as a script in slices of `slice` fuel: its value, its fuel, its heap and how
/// often it stopped for the tables. `cold`: the VM starts without them and gets them when
/// it stops, as djs-wasm's host does.
fn run(src: &str, cold: bool, slice: u64) -> (String, u64, Vec<u8>, u32) {
    let tables = [
        (
            crate::unicode::PROPS,
            crate::unicode::builtin_props().unwrap(),
        ),
        (
            crate::unicode::NORM,
            crate::unicode::builtin_norm().unwrap(),
        ),
    ];
    let mut vm = vm();
    if cold {
        vm.drop_tables();
    }
    let mut stops = 0;
    // The families it stopped for (and only those).
    let give = |vm: &mut Vm| {
        let need = vm.needs_tables();
        for (family, bytes) in tables {
            if need & family != 0 {
                vm.set_tables(family, bytes);
            }
        }
        assert_eq!(vm.needs_tables(), 0);
    };
    let opts = CompileOptions::script("t.js");
    let f = loop {
        match vm.compile(src.as_bytes(), &opts) {
            Ok(f) => break f,
            Err(_) if vm.needs_tables() != 0 => {
                stops += 1;
                give(&mut vm);
            }
            Err(e) => panic!("{src}: {}", e.message),
        }
    };
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let mut fuel = 0;
    let mut budget = slice;
    let value = loop {
        let (st, used) = vm.run_metered(budget);
        fuel += used;
        match st {
            Status::Done(v) => break vm.display(v),
            Status::Threw(v) => break alloc::format!("throw {}", vm.display(v)),
            Status::OutOfFuel if vm.needs_tables() != 0 => {
                stops += 1;
                give(&mut vm);
                // Fuel 0: the run goes on with what the stopped slice had left.
                budget = 0;
            }
            Status::OutOfFuel => budget = slice,
            s => panic!("{s:?}"),
        }
    };
    (value, fuel, vm.bytes().to_vec(), stops)
}

#[test]
fn handed_over_tables_change_nothing() {
    // (source, how often it stops: at its compile, or at run time where it builds its
    // pattern or source from parts, or normalizes).
    let b = "const b = String.fromCharCode(92); ";
    let cases: Vec<(String, u32)> = [
        // A literal in the script, and one in a function compiled at its first call.
        (r"/\p{Lu}+/u.exec('abcDEFg')[0]", 1),
        (
            r"const f = (s) => /\p{Script=Greek}/u.test(s); f('α') + ',' + f('a')",
            1,
        ),
        (r"/^\p{RGI_Emoji}$/v.test('👨‍👩‍👧')", 1),
        // Errors stay errors.
        (
            r"try { new RegExp('\\p{Nope}', 'u') } catch (e) { String(e) }",
            1,
        ),
        // A source that only mentions the escape (the tables load for nothing).
        (r"'\\p{L} is text'.length", 1),
        (r"/a/.test('a')", 0),
        // Normalization: only strings that need the tables stop.
        (r"'e\u0301'.normalize() + 'abc'.normalize('NFD')", 1),
        (r"'é'.normalize('NFD').length + 'ﬁ'.normalize('NFKC')", 1),
        (
            r"['b', 'a\u0301', 'a', 'á'].sort((x, y) => x.localeCompare(y)).join()",
            1,
        ),
        (r"'résumé'.localeCompare('resume') + 'abc'.normalize()", 0),
        // Both families, one after the other.
        (r"/\p{L}/u.test('e\u0301'.normalize('NFD'))", 2),
    ]
    .into_iter()
    .map(|(s, n)| (String::from(s), n))
    .chain(
        [
            // Run-time compiles of sources and patterns made at run time: new RegExp,
            // eval, Function, a function inside eval, RegExp through String methods.
            "new RegExp(b + 'p{Nd}+', 'u').exec('x123y')[0]",
            "eval('/' + b + 'P{L}/u.test(\"1\")')",
            "new Function('s', 'return /' + b + 'p{Emoji}/u.test(s)')('😀')",
            "eval('(() => /[' + b + 'p{L}--' + b + 'p{Ll}]/v.exec(\"aB\")[0])')()",
            "'a1B2'.replace(new RegExp(b + 'p{Lu}', 'gu'), '_')",
            "try { new RegExp(b + 'p{Nope}', 'u') } catch (e) { String(e) }",
        ]
        .into_iter()
        .map(|s| (alloc::format!("{b}{s}"), 1)),
    )
    .collect();
    for (src, stops) in &cases {
        let src = src.as_str();
        let warm = run(src, false, u64::MAX);
        assert_eq!(warm.3, 0, "{src}");
        for slice in [u64::MAX, 7] {
            let cold = run(src, true, slice);
            assert_eq!(cold.0, warm.0, "{src}");
            assert_eq!(cold.1, warm.1, "fuel: {src}");
            assert!(cold.2 == warm.2, "heap: {src}");
            assert_eq!(cold.3, *stops, "stops: {src}");
        }
    }
}

#[test]
fn without_a_host_a_property_escape_is_a_syntax_error() {
    let mut vm = vm();
    vm.drop_tables();
    vm.set_elastic_stack(false);
    let opts = CompileOptions::script("t.js");
    let e = vm.compile(br"/\p{L}/u", &opts).unwrap_err();
    assert_eq!(vm.needs_tables(), 0);
    assert_eq!(e.message, crate::builtins::regexp::NO_PROPS);
}
