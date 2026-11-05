# Opcodes

Every instruction of the bytecode, generated from the `ops!` table in
`crates/djs-core/src/bytecode/ops.rs`: the encoded byte, the immediates, the encoded
size, the stack effect and the fuel cost class. The emitter, decoder, disassembler and
the stack-depth checker are generated from the same table, so this page cannot disagree
with them.

<!-- gen:ops-all -->
| opcode | byte | immediates | size | pop → push | cost | description |
| --- | ---: | --- | ---: | --- | --- | --- |
| `undefined` | 0 | – | 1 | 0 → 1 | Basic | `-> undefined` |
| `null` | 1 | – | 1 | 0 → 1 | Basic | `-> null` |
| `push_true` | 2 | – | 1 | 0 → 1 | Basic | `-> true` |
| `push_false` | 3 | – | 1 | 0 → 1 | Basic | `-> false` |
| `push_i8` | 4 | I8 | 2 | 0 → 1 | Basic | `-> n`: a small integer constant. |
| `push_i32` | 5 | I32 | 5 | 0 → 1 | Basic | `-> n`: an integer constant. |
| `push_const` | 6 | Const | 5 | 0 → 1 | Basic | `-> v`: a constant pool entry (non-int32 numbers, strings). |
| `push_atom` | 7 | Atom | 5 | 0 → 1 | Basic | `-> s`: the string of an atom. |
| `object` | 8 | – | 1 | 0 → 1 | Alloc | `-> {}` |
| `array_from` | 9 | Count | 3 | imm0 → 1 | Alloc | `v1 .. vn -> array`: an array literal without holes or spread. |
| `array_push` | 10 | – | 1 | 2 → 1 | Basic | `array v -> array` |
| `array_hole` | 11 | – | 1 | 1 → 1 | Basic | `array -> array`: append a hole. |
| `array_spread` | 12 | – | 1 | 2 → 1 | Access | `array iterable -> array`: append the elements of an iterable (spread). |
| `fclosure` | 13 | Child | 5 | 0 → 1 | Alloc | `-> function`: create a closure of child template `child`. |
| `regexp` | 14 | – | 1 | 2 → 1 | Alloc | `pattern flags -> regexp` |
| `get_template_object` | 15 | Const | 5 | 0 → 1 | Alloc | `-> template object`: tagged templates. `CONSTS[idx]` is the site's object once created; before that, the index of `n`, `n` cooked strings (or `undefined`) and `n` raw strings, from which the first run creates and caches it. |
| `special_object` | 16 | U8 | 2 | 0 → 1 | Alloc | `-> v`: `arguments`, `new.target`, home object, ... (function entry only). |
| `rest` | 17 | Arg | 3 | 0 → 1 | Alloc | `-> array`: the actual arguments from `index` on (a rest parameter). |
| `drop` | 18 | – | 1 | 1 → 0 | Basic | `a ->` |
| `dup` | 19 | – | 1 | 1 → 2 | Basic | `a -> a a` |
| `dup2` | 20 | – | 1 | 2 → 4 | Basic | `a b -> a b a b` |
| `swap` | 21 | – | 1 | 2 → 2 | Basic | `a b -> b a` |
| `nip` | 22 | – | 1 | 2 → 1 | Basic | `a b -> b` |
| `insert2` | 23 | – | 1 | 2 → 3 | Basic | `a b -> b a b` |
| `insert3` | 24 | – | 1 | 3 → 4 | Basic | `a b c -> c a b c` |
| `insert4` | 25 | – | 1 | 4 → 5 | Basic | `a b c d -> d a b c d` |
| `rot3l` | 26 | – | 1 | 3 → 3 | Basic | `x a b -> a b x` |
| `rot3r` | 27 | – | 1 | 3 → 3 | Basic | `a b x -> x a b` |
| `pick` | 28 | U8 | 2 | 1+imm0 → 2+imm0 | Basic | `x a1 .. an -> x a1 .. an x`: copy the value under the top `n` (`pick 0` is `dup`). Destructuring reaches its iterator under assignment-target references. |
| `poke` | 29 | U8 | 2 | 2+imm0 → 1+imm0 | Basic | `x a1 .. an v -> v a1 .. an`: overwrite the value under the top `n` + 1. |
| `name_get` | 30 | NameRef | 5 | 0 → 1 | Free | `-> v`: read a name. |
| `name_put` | 31 | NameRef | 5 | 1 → 0 | Free | `v ->`: assign a name. |
| `name_set` | 32 | NameRef | 5 | 1 → 1 | Free | `v -> v`: assign a name, keep the value. |
| `name_init` | 33 | NameRef | 5 | 1 → 0 | Free | `v ->`: initialize a declaration (no TDZ check, const allowed). |
| `name_typeof` | 34 | NameRef | 5 | 0 → 1 | Free | `-> v`: read a name for `typeof` (undeclared globals give `undefined`). |
| `enter_scope` | 35 | Scope | 3 | 0 → 0 | Free | Scope entry: TDZ setup, Boxes, hoisted block functions. |
| `leave_scope` | 36 | Scope | 3 | 0 → 0 | Free | Scope exit. |
| `iter_scope` | 37 | Scope | 3 | 0 → 0 | Free | Start of a `for (let ...)` iteration: fresh copies of the scope's captured bindings (`rebox`, CreatePerIterationEnvironment). |
| `label` | 38 | LabelId | 5 | 0 → 0 | Free | A label position (symbolic labels). |
| `get_loc` | 39 | Loc | 3 | 0 → 1 | Basic | `-> v` |
| `put_loc` | 40 | Loc | 3 | 1 → 0 | Basic | `v ->` |
| `set_loc` | 41 | Loc | 3 | 1 → 1 | Basic | `v -> v` |
| `get_loc_chk` | 42 | Loc | 3 | 0 → 1 | Basic | `-> v`, ReferenceError if uninitialized (TDZ). |
| `put_loc_chk` | 43 | Loc | 3 | 1 → 0 | Basic | `v ->`, ReferenceError if uninitialized. |
| `set_loc_chk` | 44 | Loc | 3 | 1 → 1 | Basic | `v -> v`, ReferenceError if uninitialized. |
| `set_loc_uninit` | 45 | Loc | 3 | 0 → 0 | Basic | Store the uninitialized marker. |
| `inc_loc` | 46 | Loc | 3 | 0 → 0 | Basic | `loc = loc + 1` (`i++;`, `++i;` on an unchecked local): `inc` in place. An object takes `%ToNumeric` into the local first, then the instruction runs again. |
| `dec_loc` | 47 | Loc | 3 | 0 → 0 | Basic | `loc = loc - 1`, as `inc_loc`. |
| `get_arg` | 48 | Arg | 3 | 0 → 1 | Basic | `-> v` |
| `put_arg` | 49 | Arg | 3 | 1 → 0 | Basic | `v ->` |
| `set_arg` | 50 | Arg | 3 | 1 → 1 | Basic | `v -> v` |
| `get_box` | 51 | Loc | 3 | 0 → 1 | Basic | `-> v`: the value of the Box in local `loc`. |
| `put_box` | 52 | Loc | 3 | 1 → 0 | Basic | `v ->` |
| `set_box` | 53 | Loc | 3 | 1 → 1 | Basic | `v -> v` |
| `get_box_chk` | 54 | Loc | 3 | 0 → 1 | Basic | `-> v` with a TDZ check. |
| `put_box_chk` | 55 | Loc | 3 | 1 → 0 | Basic | `v ->` with a TDZ check. |
| `set_box_chk` | 56 | Loc | 3 | 1 → 1 | Basic | `v -> v` with a TDZ check. |
| `new_box` | 57 | Loc | 3 | 0 → 0 | Alloc | `loc = new Box(uninitialized)` |
| `new_box_init` | 58 | Loc | 3 | 1 → 0 | Alloc | `v ->`: `loc = new Box(v)`. |
| `rebox` | 59 | Loc | 3 | 0 → 0 | Alloc | `loc = new Box(current value)`: per-iteration bindings. |
| `get_cap` | 60 | Cap | 3 | 0 → 1 | Basic | `-> v`: the value of the closure's capture `cap`. |
| `put_cap` | 61 | Cap | 3 | 1 → 0 | Basic | `v ->` |
| `set_cap` | 62 | Cap | 3 | 1 → 1 | Basic | `v -> v` |
| `get_cap_chk` | 63 | Cap | 3 | 0 → 1 | Basic | `-> v` with a TDZ check. |
| `put_cap_chk` | 64 | Cap | 3 | 1 → 0 | Basic | `v ->` with a TDZ check. |
| `set_cap_chk` | 65 | Cap | 3 | 1 → 1 | Basic | `v -> v` with a TDZ check. |
| `get_builtin` | 66 | Builtin | 3 | 0 → 1 | Basic | `-> f`: entry `idx` of the realm's prelude function table (privileged code: a prelude function another one refers to). |
| `put_builtin` | 67 | Builtin, Count | 5 | 1 → 0 | Alloc | `f ->`: stores entry `idx` of the prelude function table, which has `len` entries (allocated by the first store). |
| `get_global` | 68 | Atom, Cache | 7 | 0 → 1 | Access | `-> v`: global lexical record, then the global object; ReferenceError if missing. |
| `get_global_typeof` | 69 | Atom, Cache | 7 | 0 → 1 | Access | `-> v`: like `get_global`, but `undefined` if missing (`typeof`). |
| `put_global` | 70 | Atom, Cache | 7 | 1 → 0 | Access | `v ->`: ReferenceError if missing, TypeError for a global const. |
| `init_global_lex` | 71 | Atom | 5 | 1 → 0 | Access | `v ->`: initialize a Script top-level let/const/class. |
| `global_decls` | 72 | Const | 5 | 0 → 0 | Access | Script GlobalDeclarationInstantiation from the declaration list in constant `idx`. |
| `throw_const_assign` | 73 | Atom | 5 | 0 → 0 | Basic | TypeError `'x' is read-only`. |
| `get_field` | 74 | Atom, Cache | 7 | 1 → 1 | Access | `obj -> v` |
| `get_method` | 75 | Atom, Cache | 7 | 1 → 2 | Access | `obj -> f obj`: callee and `this` for a method call. |
| `put_field` | 76 | Atom, Cache | 7 | 2 → 0 | Access | `obj v ->` |
| `get_elem` | 77 | – | 1 | 2 → 1 | Access | `obj key -> v` |
| `get_elem2` | 78 | – | 1 | 2 → 3 | Access | `obj key -> obj key' v`: `get_elem` keeping the reference for a compound assignment or update, with `key'` = ToPrimitive(key) (one conversion for the get and the put, after the RequireObjectCoercible of `obj`). |
| `to_propkey` | 79 | – | 1 | 1 → 1 | Basic | `key -> key'`: ToPrimitive(key, string) now, for a computed key whose definition comes after other code (`{ [k]: v }`: ToPropertyKey before `v`). Primitives stay. |
| `get_elem_method` | 80 | – | 1 | 2 → 2 | Access | `obj key -> f obj` |
| `put_elem` | 81 | – | 1 | 3 → 0 | Access | `obj key v ->` |
| `define_field` | 82 | Atom, Cache | 7 | 2 → 1 | Access | `obj v -> obj`: CreateDataProperty (literals). |
| `define_elem` | 83 | – | 1 | 3 → 1 | Access | `obj key v -> obj`: computed keys. |
| `define_getter` | 84 | Atom | 5 | 2 → 1 | Access | `obj f -> obj` |
| `define_setter` | 85 | Atom | 5 | 2 → 1 | Access | `obj f -> obj` |
| `define_getter_elem` | 86 | – | 1 | 3 → 1 | Access | `obj key f -> obj` |
| `define_setter_elem` | 87 | – | 1 | 3 → 1 | Access | `obj key f -> obj` |
| `define_method` | 88 | Atom | 5 | 2 → 1 | Access | `obj f -> obj`: also sets the home object. |
| `define_method_elem` | 89 | – | 1 | 3 → 1 | Access | `obj key f -> obj` |
| `set_proto` | 90 | – | 1 | 2 → 1 | Basic | `obj v -> obj`: a literal `__proto__: v` sets the prototype when `v` is an object or null (other values are ignored). |
| `set_function_name` | 91 | U8 | 2 | 2 → 2 | Basic | `key f -> key f`: SetFunctionName(f, ToPropertyKey(key), prefix) for an anonymous function under a computed key (`prefix`: 0 none, 1 "get", 2 "set"). |
| `copy_data_props` | 92 | U8 | 2 | 2 → 1 | Access | `obj src -> obj`: object spread / rest (flags select the variant). |
| `delete` | 93 | – | 1 | 2 → 1 | Access | `obj key -> bool` |
| `in` | 94 | – | 1 | 2 → 1 | Access | `key obj -> bool` |
| `private_symbol` | 95 | Atom, U8 | 6 | 0 → 1 | Alloc | `-> sym`: a new private name (a private symbol described `#x`). `flags`: `heap::cells::flags::SYM_PRIVATE_METHOD` (a method or accessor) and `SYM_PRIVATE_READONLY` (no setter). |
| `get_private_field` | 96 | – | 1 | 2 → 1 | Access | `obj sym -> v`: `obj.#x` (own properties only, never a proxy trap; a getter is called). |
| `put_private_field` | 97 | – | 1 | 3 → 0 | Access | `obj sym v ->`: `obj.#x = v` (a setter is called). |
| `define_private_field` | 98 | U8 | 2 | 3 → 1 | Access | `obj sym v -> obj`: adds a private field (`kind` 0) or method (1), ignoring extensibility; TypeError if `obj` has it already. |
| `define_private_accessor` | 99 | – | 1 | 4 → 1 | Access | `obj sym getter setter -> obj`: adds a private accessor (`undefined` for a missing half). |
| `private_in` | 100 | – | 1 | 2 → 1 | Access | `sym obj -> bool`: `#x in obj`. |
| `get_super` | 101 | – | 1 | 1 → 1 | Access | `o -> o.[[GetPrototypeOf]]()`: the base of a super property (from the home object), or the super constructor (from the active function). |
| `get_super_field` | 102 | Atom | 5 | 2 → 1 | Access | `this base -> v`: `super.x`, base.[[Get]](x, this). |
| `put_super_field` | 103 | Atom | 5 | 3 → 0 | Access | `this base v ->`: `super.x = v`, base.[[Set]](x, v, this). |
| `get_super_elem` | 104 | – | 1 | 3 → 1 | Access | `this base key -> v` |
| `put_super_elem` | 105 | – | 1 | 4 → 0 | Access | `this base key v ->` |
| `set_home_object` | 106 | – | 1 | 2 → 2 | Basic | `home f -> home f`: sets the home object of `f` (class field initializers). |
| `define_class` | 107 | U8 | 2 | 3 → 2 | Alloc | `parent protoParent ctor -> ctor proto`: ClassDefinitionEvaluation's prototype and constructor setup. `flags` bit 0: there is a heritage (`parent` and `protoParent`, checked by %ClassHeritage, are used; otherwise ignored). |
| `define_class_method` | 108 | Atom, U8 | 6 | 2 → 1 | Access | `obj f -> obj`: a class method (`kind` 0), getter (1) or setter (2): non-enumerable, and sets the home object. |
| `define_class_method_elem` | 109 | U8 | 2 | 3 → 1 | Access | `obj key f -> obj` |
| `define_class_field` | 110 | – | 1 | 3 → 1 | Access | `obj key v -> obj`: a public class field (CreateDataPropertyOrThrow). |
| `require_object_coercible` | 111 | – | 1 | 1 → 1 | Basic | `v -> v`: TypeError for `null`/`undefined` (destructuring). |
| `neg` | 112 | – | 1 | 1 → 1 | Basic | `a -> -a` |
| `plus` | 113 | – | 1 | 1 → 1 | Basic | `a -> +a` (ToNumber) |
| `bnot` | 114 | – | 1 | 1 → 1 | Basic | `a -> ~a` |
| `not` | 115 | – | 1 | 1 → 1 | Basic | `a -> !a` |
| `typeof` | 116 | – | 1 | 1 → 1 | Basic | `a -> typeof a` |
| `inc` | 117 | – | 1 | 1 → 1 | Basic | `a -> a + 1` (numeric) |
| `dec` | 118 | – | 1 | 1 → 1 | Basic | `a -> a - 1` (numeric) |
| `to_numeric` | 119 | – | 1 | 1 → 1 | Basic | `a -> ToNumeric(a)` (postfix `++`/`--`) |
| `to_string` | 120 | – | 1 | 1 → 1 | Basic | `a -> ToString(a)` (template substitutions) |
| `add` | 121 | – | 1 | 2 → 1 | Basic | `a b -> a + b` |
| `sub` | 122 | – | 1 | 2 → 1 | Basic | `a b -> a - b` |
| `mul` | 123 | – | 1 | 2 → 1 | Basic | `a b -> a * b` |
| `div` | 124 | – | 1 | 2 → 1 | Basic | `a b -> a / b` |
| `mod` | 125 | – | 1 | 2 → 1 | Basic | `a b -> a % b` |
| `pow` | 126 | – | 1 | 2 → 1 | Basic | `a b -> a ** b` |
| `shl` | 127 | – | 1 | 2 → 1 | Basic | `a b -> a << b` |
| `sar` | 128 | – | 1 | 2 → 1 | Basic | `a b -> a >> b` |
| `shr` | 129 | – | 1 | 2 → 1 | Basic | `a b -> a >>> b` |
| `band` | 130 | – | 1 | 2 → 1 | Basic | `a b -> a & b` |
| `bor` | 131 | – | 1 | 2 → 1 | Basic | `a b -> a \| b` |
| `bxor` | 132 | – | 1 | 2 → 1 | Basic | `a b -> a ^ b` |
| `lt` | 133 | – | 1 | 2 → 1 | Basic | `a b -> a < b` |
| `le` | 134 | – | 1 | 2 → 1 | Basic | `a b -> a <= b` |
| `gt` | 135 | – | 1 | 2 → 1 | Basic | `a b -> a > b` |
| `ge` | 136 | – | 1 | 2 → 1 | Basic | `a b -> a >= b` |
| `eq` | 137 | – | 1 | 2 → 1 | Basic | `a b -> a == b` |
| `ne` | 138 | – | 1 | 2 → 1 | Basic | `a b -> a != b` |
| `strict_eq` | 139 | – | 1 | 2 → 1 | Basic | `a b -> a === b` |
| `strict_ne` | 140 | – | 1 | 2 → 1 | Basic | `a b -> a !== b` |
| `instanceof` | 141 | – | 1 | 2 → 1 | Access | `a b -> a instanceof b` |
| `is_nullish` | 142 | – | 1 | 1 → 1 | Basic | `v -> v is null or undefined` |
| `is_undefined` | 143 | – | 1 | 1 → 1 | Basic | `v -> v === undefined` |
| `concat` | 144 | Count | 3 | imm0 → 1 | Alloc | `s1 .. sn -> s`: join template literal parts in one allocation. |
| `jump` | 145 | Target | 5 | 0 → 0 | Basic | Unconditional jump. |
| `jump_if_false` | 146 | Target | 5 | 1 → 0 | Basic | `v ->`: jump if ToBoolean(v) is false. |
| `jump_if_true` | 147 | Target | 5 | 1 → 0 | Basic | `v ->`: jump if ToBoolean(v) is true. |
| `throw` | 148 | – | 1 | 1 → 0 | Basic | `v ->`: throw `v`. |
| `return` | 149 | – | 1 | 1 → 0 | Basic | `v ->`: return `v`. |
| `return_undef` | 150 | – | 1 | 0 → 0 | Basic | Return `undefined`. |
| `call` | 151 | Argc | 3 | 2+imm0 → 1 | Call | `f this a1 .. an -> r` |
| `construct` | 152 | Argc | 3 | 1+imm0 → 1 | Call | `f a1 .. an -> r`: `new f(...)`, new.target = f. |
| `call_spread` | 153 | – | 1 | 3 → 1 | Call | `f this args -> r`: call with an argument array (spread). |
| `construct_spread` | 154 | – | 1 | 2 → 1 | Call | `f args -> r` |
| `super_call` | 155 | Argc | 3 | 2+imm0 → 1 | Call | `f a1 .. an newTarget -> r`: `super(...)` in a derived constructor: constructs `f` (the super constructor) with that new.target. |
| `super_call_spread` | 156 | – | 1 | 3 → 1 | Call | `f args newTarget -> r`: `super(...args)` with an argument array (spread). |
| `call_intrinsic` | 157 | Intrinsic, Argc | 5 | imm1 → 1 | Call | `a1 .. an -> r`: privileged code only. |
| `generator_start` | 158 | Loc | 3 | 0 → 0 | Alloc | Generator function entry: creates the generator object, stores it in local `generator`, suspends the frame (to resume at the next instruction) and returns the object to the caller. |
| `yield` | 159 | Loc | 3 | 1 → 2 | Alloc | `v -> received isReturn`: suspends the generator in local `generator`, handing `v` to the resumer. Resumed by `next(x)`: `x false`; by `return(x)`: `x true` (the code returns `x`, running finally blocks); `throw(e)` throws `e` here. |
| `yield_star` | 160 | Loc | 3 | 1 → 2 | Alloc | `innerResult -> received isReturn`: `yield` for `yield*` in a sync generator: the resumer hands `innerResult` (the inner iterator's result object) on as it is, without reading `value` or wrapping it (GeneratorYield ( innerResult )). |
| `async_start` | 161 | Loc | 3 | 0 → 0 | Alloc | Async function entry: creates the async state (with the result promise) in local `state`. The function runs on synchronously. |
| `await` | 162 | Loc | 3 | 1 → 1 | Alloc | `v -> result`: suspends the async function of local `state` until `v` settles, then pushes its value or throws its reason here. The first await returns the result promise to the caller. |
| `async_return` | 163 | Loc | 3 | 1 → 0 | Call | `v ->`: return from an async function: resolves the result promise with `v` and returns the promise. |
| `async_reject` | 164 | Loc | 3 | 1 → 0 | Call | `e ->`: an exception leaves an async function: rejects the result promise with `e` and returns the promise (the target of the body's catch-all handler). |
| `get_iterator` | 165 | – | 1 | 1 → 1 | Access | `obj -> iterator` |
| `iter_next` | 166 | Target | 5 | 2 → 1 | Call | `it next -> r`: `r = next.call(it)`, an iterator result. When `next` is the original `next` of an Array or String Iterator and that step runs no user code, `r` is the next value itself and the instruction jumps (no result object). |
| `iter_next_loc` | 167 | Loc, Target | 7 | 0 → 1 | Call | `-> r`: `iter_next` of the iterator in local `loc` and its `next` in `loc + 1` (a loop's or pattern's temporaries): one instruction for for-of's step. |
| `iter_check_object` | 168 | – | 1 | 1 → 1 | Basic | `v -> v`: TypeError if `v` is not an object (iterator results). |
| `iterator_close` | 169 | – | 1 | 1 → 0 | Access | `iterator ->`: call `iterator.return()` if present. |
| `for_in_start` | 170 | – | 1 | 1 → 1 | Alloc | `obj -> for-in iterator` (a snapshot of the enumerable keys) |
| `for_in_next` | 171 | – | 1 | 1 → 3 | Access | `it -> it key done` |
| `check_ctor` | 172 | – | 1 | 0 → 0 | Basic | TypeError if a class constructor is called without `new`. |
| `check_this` | 173 | – | 1 | 1 → 1 | Basic | `v -> v`: a derived constructor's `this`: ReferenceError while uninitialized (`undefined`, before `super()`). |
| `check_this_uninit` | 174 | – | 1 | 1 → 0 | Basic | `v ->`: before `super()` binds `this`: ReferenceError if it is bound already. |
| `check_ctor_return` | 175 | – | 1 | 1 → 1 | Basic | `v -> v`: a derived constructor's return value: TypeError unless an object or `undefined`. |
| `throw_error` | 176 | Atom, U8 | 6 | 0 → 0 | Basic | Throws a new error with message `atom`; `kind` 0: TypeError, 1: ReferenceError. |
| `debugger` | 177 | – | 1 | 0 → 0 | Free | `debugger` (no effect). |
| `nop` | 178 | – | 1 | 0 → 0 | Free | No operation. |
| `get_loc0` | 179 | – | 1 | 0 → 1 | Basic | `get_loc 0` |
| `get_loc1` | 180 | – | 1 | 0 → 1 | Basic | `get_loc 1` |
| `get_loc2` | 181 | – | 1 | 0 → 1 | Basic | `get_loc 2` |
| `get_loc3` | 182 | – | 1 | 0 → 1 | Basic | `get_loc 3` |
| `put_loc0` | 183 | – | 1 | 1 → 0 | Basic | `put_loc 0` |
| `put_loc1` | 184 | – | 1 | 1 → 0 | Basic | `put_loc 1` |
| `put_loc2` | 185 | – | 1 | 1 → 0 | Basic | `put_loc 2` |
| `put_loc3` | 186 | – | 1 | 1 → 0 | Basic | `put_loc 3` |
| `get_arg0` | 187 | – | 1 | 0 → 1 | Basic | `get_arg 0` |
| `get_arg1` | 188 | – | 1 | 0 → 1 | Basic | `get_arg 1` |
| `get_arg2` | 189 | – | 1 | 0 → 1 | Basic | `get_arg 2` |
| `get_arg3` | 190 | – | 1 | 0 → 1 | Basic | `get_arg 3` |
| `set_loc0` | 191 | – | 1 | 1 → 1 | Basic | `set_loc 0` |
| `set_loc1` | 192 | – | 1 | 1 → 1 | Basic | `set_loc 1` |
| `set_loc2` | 193 | – | 1 | 1 → 1 | Basic | `set_loc 2` |
| `set_loc3` | 194 | – | 1 | 1 → 1 | Basic | `set_loc 3` |
| `push0` | 195 | – | 1 | 0 → 1 | Basic | `push_i8 0` |
| `push1` | 196 | – | 1 | 0 → 1 | Basic | `push_i8 1` |
| `push2` | 197 | – | 1 | 0 → 1 | Basic | `push_i8 2` |
| `push3` | 198 | – | 1 | 0 → 1 | Basic | `push_i8 3` |
| `call0` | 199 | – | 1 | 2 → 1 | Call | `call 0` |
| `call1` | 200 | – | 1 | 3 → 1 | Call | `call 1` |
| `call2` | 201 | – | 1 | 4 → 1 | Call | `call 2` |
| `call3` | 202 | – | 1 | 5 → 1 | Call | `call 3` |
| `get_loc8` | 203 | Loc8 | 2 | 0 → 1 | Basic | `get_loc` of a local below 256. |
| `put_loc8` | 204 | Loc8 | 2 | 1 → 0 | Basic | `put_loc` of a local below 256. |
| `set_loc8` | 205 | Loc8 | 2 | 1 → 1 | Basic | `set_loc` of a local below 256. |
| `get_arg8` | 206 | Arg8 | 2 | 0 → 1 | Basic | `get_arg` of an argument below 256. |
| `push_const8` | 207 | Const8 | 2 | 0 → 1 | Basic | `push_const` of a constant below 256. |
| `call_intrinsic8` | 208 | Intrinsic8 | 2 | 0 → 1 | Call | `a1 .. an -> r`: `call_intrinsic id, n` of an intrinsic below 256 with a fixed arity `n` (its declared parameters: privileged calls pass exactly those). |
| `get_builtin_callee8` | 209 | Builtin8 | 2 | 0 → 2 | Basic | `-> f undefined`: `get_builtin idx; undefined`, a prelude function as the callee of a call with `this` undefined. |
| `jump8` | 210 | Target8 | 2 | 0 → 0 | Basic | `jump` with an 8-bit offset. |
| `jump_if_false8` | 211 | Target8 | 2 | 1 → 0 | Basic | `jump_if_false` with an 8-bit offset. |
| `jump_if_true8` | 212 | Target8 | 2 | 1 → 0 | Basic | `jump_if_true` with an 8-bit offset. |
| `lazy` | 213 | – | 1 | 0 → 0 | Alloc | The whole code of a lazy template: compiles the function at its first call and continues in the compiled template, in the same frame. |
<!-- /gen -->
