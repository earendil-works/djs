//! Subset checker: programs it must accept and programs it must reject (with the
//! expected message and position).

const DECLS: &str = "\
/** @leaf */ declare function IsObject(v: Value): boolean;
/** @leaf */ declare function AsObj(v: Value): Obj;
/** @leaf */ declare function AsFn(v: Value): Fn;
/** @leaf */ declare function NewTypeError(m: Str): Obj;
/** @leaf */ declare function Fail(m: Str): never;
/** @leaf */ declare function Opt(a: Value, b?: Value): void;
/** @reentrant */ declare function Get(o: Obj, k: Value): Value;
/** @reentrant */ declare function Call(f: Value, t: Value, ...args: Value[]): Value;
";

fn compile(src: &str) -> Result<djs_bgen::Output, String> {
    djs_bgen::compile(
        ("intrinsics.d.ts", DECLS),
        &[("t.ts".to_string(), src.to_string())],
    )
}

fn accept(src: &str) {
    if let Err(e) = compile(src) {
        panic!("rejected:\n{src}\n{e}");
    }
}

/// Rejects with a message containing `want`; returns the rendered errors.
fn reject(src: &str, want: &str) -> String {
    match compile(src) {
        Ok(_) => panic!("accepted:\n{src}"),
        Err(e) => {
            assert!(
                e.contains(want),
                "{src}\n---\n{e}\ndoes not mention `{want}`"
            );
            e
        }
    }
}

#[test]
fn accepts() {
    accept("function F(x: Value): Value { return x; }");
    accept("function F(this: Value): Obj { return AsObj(this); }");
    accept("function F(o: Obj): Obj { const f: Fn = AsFn(o); return f; }"); // Fn widens to Obj
    accept("function F(): f64 { let k: i32 = 1; let x: f64 = k; x = x / 2; return x % 3; }");
    accept("function F(): u32 { let k: u32 = 0; k += 1; k = k * 2 - 1; k++; --k; return k; }");
    accept("function F(): i32 { const a: i32 = -5; return (a & 3) | (a << 2) ^ ~a >> 1; }");
    accept("function F(a: i32): u32 { return a >>> 0; }");
    accept("function F(): boolean { return 1 < 2 && !(3 >= 4) || 2.5 > 1; }");
    accept("function F(): Str { let s: Str = \"a\"; s += \"b\"; return s + \"c\"; }");
    accept("function F(x: Value): boolean { return x === undefined || x !== null; }");
    accept("function F(x: Value): Value { return IsObject(x) ? x : undefined; }");
    accept("function F(): f64 { return 4294967296; }"); // integer literals widen to f64
    accept("function F(): u32 { return 4294967295; }");
    accept(
        "function F(x: Value): void { Opt(x); Opt(x, x); Call(x, x); Call(x, x, 1, \"s\", x); }",
    );
    accept("function F(x: Value): Value { throw x; }");
    accept("function F(): Value { Fail(\"never returns\"); }"); // a never call terminates
    accept("function F(): Value { while (true) { if (IsObject(1)) { return 1; } } }");
    accept("function F(): Value { for (;;) { } }");
    accept("function F(): Value { try { return 1; } catch (e) { throw e; } }");
    accept("function F(): Value { try { Opt(1); } finally { return 2; } }");
    accept(
        "function F(): Value { if (IsObject(1)) { return 1; } else if (IsObject(2)) { return 2; } else { return 3; } }",
    );
    accept(
        "function F(): void { for (let i: i32 = 0; i < 3; i++) { if (i === 1) { continue; } break; } }",
    );
    accept("function F(): void { do { } while (false); }");
    accept("function F(): void { { const a: i32 = 1; } { const a: i32 = 2; } }"); // sibling blocks
    accept("function G(): Value { return F(1); }\nfunction F(x: Value): Value { return x; }"); // forward calls
    accept(
        "/** @builtin Foo.prototype.bar\n * @length 0\n * @native\n */ function F(this: Value, a?: Value): void {}",
    );
    accept("/** @intrinsic */ function F(a: Value): Value { return a; }");
}

#[test]
fn rejects_names_and_calls() {
    reject("function F(): Value { return y; }", "unknown name `y`");
    reject(
        "function F(): Value { return G(); }",
        "unknown function `G`",
    );
    reject(
        "function F(f: Value): Value { return f(); }",
        "Call(f, thisArg",
    );
    reject(
        "function F(): Value { return IsObject; }",
        "functions are not values",
    );
    reject(
        "function F(): void { Opt(); }",
        "`Opt` takes 1 to 2 arguments, found 0",
    );
    reject(
        "function F(): void { Opt(1, 2, 3); }",
        "takes 1 to 2 arguments",
    );
    reject("function F(): void { Call(1); }", "at least 2");
    reject(
        "function F(x: Value): void { const o: Obj = x; }",
        "expected Obj, found Value",
    );
    reject(
        "function F(o: Obj): void { const f: Fn = o; }",
        "expected Fn, found Obj",
    );
    reject(
        "function F(x: Value): void { Get(x, 1); }",
        "argument 1 of `Get`: expected Obj, found Value",
    );
    reject(
        "function F(): Value { return this; }",
        "`this: T` parameter",
    );
    reject(
        "function G(this: Value): void {}\nfunction F(): void { G(); }",
        "takes a `this` value",
    );
    reject(
        "function F(): void {}\nfunction F(): void {}",
        "already declared",
    );
    reject("function IsObject(): void {}", "already declared");
    reject(
        "function F(): void { const x: i32 = 1; x = 2; }",
        "`x` is a const",
    );
    reject(
        "function F(): void { const x: i32 = 1; x++; }",
        "`x` is a const",
    );
    reject("function F(): void { y = 2; }", "unknown local `y`");
    reject(
        "function F(a: Value): void { const a: i32 = 1; }",
        "shadows another local",
    );
    reject(
        "function F(): void { const x: i32 = 1; { const x: i32 = 2; } }",
        "shadows another local",
    );
    reject(
        "function F(): void { const Get: i32 = 1; }",
        "shadows a function",
    );
    reject(
        "function F(a: Value, a: Value): void {}",
        "duplicate parameter",
    );
    reject("function F(): void { 1 + 2; }", "must be a call");
}

#[test]
fn rejects_types_and_numbers() {
    reject(
        "function F(a: i32, b: u32): Value { return a + b; }",
        "mixes i32 and u32",
    );
    reject(
        "function F(a: i32, b: f64): Value { return a + b; }",
        "mixes i32 and f64",
    );
    reject(
        "function F(a: u32): Value { return a / 2; }",
        "differs between JS and Rust",
    );
    reject(
        "function F(a: i32): Value { return a % 2; }",
        "differs between JS and Rust",
    );
    reject("function F(a: u32): Value { return a & 1; }", "i32 only");
    reject("function F(a: f64): Value { return a | 0; }", "i32 only");
    reject(
        "function F(a: u32): Value { return -a; }",
        "unary `-` needs i32 or f64",
    );
    reject(
        "function F(): u32 { return -1; }",
        "expected u32, found integer literal",
    );
    reject("function F(): i32 { return 2147483648; }", "expected i32");
    reject(
        "function F(): i32 { return 1.5; }",
        "expected i32, found number literal",
    );
    reject(
        "function F(a: i32, b: u32): boolean { return a === b; }",
        "unrelated types i32 and u32",
    );
    reject(
        "function F(a: i32, b: u32): boolean { return a < b; }",
        "mixes i32 and u32",
    );
    reject(
        "function F(a: Str, b: Str): boolean { return a < b; }",
        "needs numbers",
    );
    reject(
        "function F(a: Str, b: f64): Value { return a + b; }",
        "Str + Str",
    );
    reject(
        "function F(a: Value): boolean { return !a; }",
        "no implicit truthiness",
    );
    reject(
        "function F(a: Value): void { if (a) {} }",
        "condition: expected boolean, found Value",
    );
    reject(
        "function F(a: i32): void { while (a) {} }",
        "condition: expected boolean",
    );
    reject(
        "function F(a: boolean, b: i32): boolean { return a && b; }",
        "needs booleans",
    );
    reject(
        "function F(a: boolean): Value { return a ? 1 : \"s\"; }",
        "unrelated types",
    );
    reject(
        "function F(): void { let s: Str = \"\"; s -= \"a\"; }",
        "needs numbers",
    );
    reject(
        "function F(): void { let k: u32 = 1; k /= 2; }",
        "differs between JS and Rust",
    );
    reject(
        "function F(): void { let s: Str = \"\"; s++; }",
        "numeric local",
    );
    reject("function F(a: void): void {}", "void or never");
    reject(
        "function F(): void { const x: void = Opt(1); }",
        "void or never",
    );
    reject(
        "function F(): Value { return Opt(1); }",
        "expected Value, found void",
    );
}

#[test]
fn rejects_control_flow_and_tags() {
    reject(
        "function F(a: boolean): Value { if (a) { return 1; } }",
        "can reach its end without returning a Value",
    );
    reject(
        "function F(): Value { while (true) { break; } }",
        "can reach its end",
    );
    reject(
        "function F(): Value { try { return 1; } catch (e) { } }",
        "can reach its end",
    );
    reject("function F(): void { return 1; }", "returns no value");
    reject("function F(): Value { return; }", "needs a Value value");
    reject("function F(): void { break; }", "outside a loop");
    reject(
        "function F(a: boolean): void { if (a) const x: i32 = 1; }",
        "inside a block",
    );
    reject(
        "/** @builtin bad path */ function F(): void {}",
        "dotted path",
    );
    reject(
        "/** @builtin A.b\n * @getter A.c */ function F(this: Value): void {}",
        "mix only with each other",
    );
    reject(
        "/** @extends Error */ function F(): void {}",
        "needs `@constructor`",
    );
    reject(
        "/** @constructor A.B */ function F(): void {}",
        "constructors are globals",
    );
    reject(
        "/** @builtin A.prototype[Symbol.x.y] */ function F(): void {}",
        "dotted path",
    );
    reject(
        "/** @builtin A.prototype[foo] */ function F(): void {}",
        "dotted path",
    );
    reject(
        "/** @value A.b\n * @attributes writable frozen */ function F(): Value { return 1; }",
        "unknown attribute `frozen`",
    );
    reject(
        "/** @value A.b */ function F(a: Value): Value { return a; }",
        "takes no parameters",
    );
    reject(
        "/** @builtin A.b */ function F(): Value { return new.target; }",
        "only available in `@constructor`",
    );
    reject("/** @length 2 */ function F(): void {}", "needs `@builtin`");
    reject(
        "/** @builtin A.b\n * @length x */ function F(): void {}",
        "expected a number",
    );
    reject("/** @frob */ function F(): void {}", "unknown tag `@frob`");
    reject(
        "/** @builtin A.b */ function F(): void {}\n/** @builtin A.b */ function G(): void {}",
        "installed twice",
    );
}

#[test]
fn errors_have_positions_and_are_all_reported() {
    let e = reject(
        "function F(): void {\n  const x: i32 = y;\n  const z: u32 = -1;\n}",
        "unknown name `y`",
    );
    assert!(e.contains("t.ts:2:18: error: unknown name `y`"), "{e}");
    assert!(
        e.contains("t.ts:3:18: error: initializer of `z`: expected u32"),
        "{e}"
    );
    assert!(
        e.contains("\n                   ^\n"),
        "caret under `y`:\n{e}"
    );
    assert!(e.ends_with("2 errors\n"), "{e}");
    // Parse errors stop at the first error of a file.
    let e = reject("function F(): void { o.x; }", "property access");
    assert!(e.contains("t.ts:1:23:"), "{e}");
}

#[test]
fn intrinsic_declarations_are_checked() {
    let bad = |decls: &str, want: &str| {
        let e = djs_bgen::compile(("intrinsics.d.ts", decls), &[])
            .err()
            .unwrap_or_default();
        assert!(e.contains(want), "{decls}\n---\n{e}");
    };
    bad(
        "declare function A(): void;",
        "needs `@leaf` or `@reentrant`",
    );
    bad(
        "/** @leaf @reentrant */ declare function A(): void;",
        "exactly one",
    );
    bad(
        "/** @pure */ declare function A(): void;",
        "unknown tag `@pure`",
    );
    bad(
        "/** @leaf */ declare function A(): void;\n/** @leaf */ declare function A(): void;",
        "declared twice",
    );
}

#[test]
fn length_defaults_to_required_params() {
    let out = compile(
        "/** @builtin A.f */ function F(this: Value, a: Value, b: Value, c?: Value): void {}\n\
         /** @builtin A.g\n * @length 5 */ function G(): void {}",
    )
    .unwrap();
    assert!(
        out.prelude.contains("%InstallMethod(\"A\", \"f\", F, 2);"),
        "{}",
        out.prelude
    );
    assert!(out.prelude.contains("%InstallMethod(\"A\", \"g\", G, 5);"));
}

#[test]
fn installation_forms() {
    let out = compile(
        "/** @builtin A.f */ function F(this: Value, a: Value, ...rest: Value[]): Obj { return rest; }\n\
         /** @constructor A\n * @length 1 */ function C(a?: Value): Value { return new.target; }\n\
         /** @constructor B\n * @extends A */ function D(): Value { return undefined; }\n\
         /** @getter A.prototype.x */ function G(this: Value): Value { return this; }\n\
         /** @setter A.prototype.x */ function S(this: Value, v: Value): void {}\n\
         /** @value A.K\n * @attributes writable configurable */ function K(): f64 { return 1.5; }\n\
         /** @builtin globalThis.f\n * @builtin A.g */ function P(a: Value): Value { return a; }\n\
         /** @builtin A.prototype[Symbol.iterator] */ function It(this: Value): Value { return this; }\n\
         function Q(): Value { return F2(1, 2, 3); }\n\
         function F2(a: Value, ...more: Value[]): Value { return more; }",
    )
    .unwrap();
    let p = &out.prelude;
    assert!(p.contains("function F(a, ...rest) {"), "{p}");
    assert!(p.contains("return new.target;"), "{p}");
    // Globals first, in source order; then the rest.
    let expected = "%InstallConstructor(\"globalThis\", \"A\", C, 1, undefined);\n\
        %InstallConstructor(\"globalThis\", \"B\", D, 0, \"A\");\n\
        %InstallMethod(\"globalThis\", \"f\", P, 1);\n\
        %InstallMethod(\"A\", \"f\", F, 1);\n\
        %InstallAccessor(\"A.prototype\", \"x\", G, false);\n\
        %InstallAccessor(\"A.prototype\", \"x\", S, true);\n\
        %InstallValue(\"A\", \"K\", K(), 5);\n\
        %InstallMethod(\"A\", \"g\", P, 1);\n\
        %InstallMethod(\"A.prototype\", %WellKnownSymbol(\"iterator\"), It, 0);\n";
    assert!(p.contains(expected), "{p}");
}
