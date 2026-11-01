// Types and intrinsics of the builtin language (ARCH.md "The builtin language").
//
// The builtin sources (builtins/*.ts) are scripts that see these declarations as
// globals. djs-bgen reads this file for the intrinsic signatures and tags; the `type`
// aliases are only for tsc and editors (bgen's types are fixed, and bgen is stricter:
// see crates/djs-bgen/src/check.rs for the numeric rules).
//
// Every intrinsic is tagged:
// - @leaf: plain Rust that may throw but never runs user code.
// - @reentrant: may run user code, but in a single step: its Rust fast path either
//   finishes or calls exactly one user function (a getter, a setter, the callee of
//   Call) and the result completes the intrinsic (D1085). Calls to reentrant intrinsics
//   are resume points. Anything that needs several steps around user code (ToPrimitive,
//   GetMethod, LengthOfArrayLike, ...) is written in the builtin language instead.
// Intrinsics are called as `Name(...)` here and emitted as `%Name(...)`.

/** Any JS value. */
type Value = unknown;
/** An object. */
type Obj = object;
/** A callable object. Only AsFn produces one. */
type Fn = Obj & { readonly __callable: true };
/** A string (WTF-16). */
type Str = string;
/** A symbol. */
type Sym = symbol;
/** Numbers whose kind bgen tracks; the Rust backend keeps them unboxed. */
type i32 = number & { readonly __num?: "i32" };
type u32 = number & { readonly __num?: "u32" };
type f64 = number;

// ---- Type tests ---------------------------------------------------------------------

/** @leaf */
declare function IsObject(v: Value): boolean;
/** IsCallable ( argument ). @leaf */
declare function IsCallable(v: Value): boolean;
/** IsConstructor ( argument ). @leaf */
declare function IsConstructor(v: Value): boolean;
/** IsArray ( argument ), without proxies. @leaf */
declare function IsArray(v: Value): boolean;
/** @leaf */
declare function IsString(v: Value): boolean;
/** @leaf */
declare function IsNumber(v: Value): boolean;
/** @leaf */
declare function IsSymbol(v: Value): boolean;
/** @leaf */
declare function IsBoolean(v: Value): boolean;

// ---- Casts (an internal error if the value has another type; test first) ------------

/** @leaf */
declare function AsObj(v: Value): Obj;
/** @leaf */
declare function AsFn(v: Value): Fn;
/** @leaf */
declare function AsStr(v: Value): Str;
/** @leaf */
declare function AsNumber(v: Value): f64;
/** @leaf */
declare function AsSym(v: Value): Sym;
/** @leaf */
declare function AsBool(v: Value): boolean;

// ---- Conversions that never run user code --------------------------------------------

/** ToBoolean ( argument ). @leaf */
declare function ToBoolean(v: Value): boolean;
/** ToNumber of a primitive (TypeError for symbols); objects go through ToPrimitive first. @leaf */
declare function ToNumberPrimitive(v: Value): f64;
/** ToString of a primitive (TypeError for symbols); objects go through ToPrimitive first. @leaf */
declare function ToStringPrimitive(v: Value): Str;
/** ToObject ( argument ): TypeError for undefined and null. @leaf */
declare function ToObject(v: Value): Obj;
/** ToIntegerOrInfinity of a number. @leaf */
declare function ToIntegerOrInfinityNumber(n: f64): f64;
/** An integral f64 in u32 range to u32 (an internal error otherwise). @leaf */
declare function F64ToU32(n: f64): u32;
/** An integral f64 in i32 range to i32 (an internal error otherwise). @leaf */
declare function F64ToI32(n: f64): i32;
/** The length of a string in code units. @leaf */
declare function StrLength(s: Str): u32;

// ---- Properties ----------------------------------------------------------------------
// Keys are primitive property keys (strings, symbols, numbers); an object key must go
// through ToPropertyKey first.

/** Get ( O, P ): may call a getter. @reentrant */
declare function Get(o: Obj, key: Value): Value;
/** GetV ( V, P ): Get with a primitive base looked up on its prototype. @reentrant */
declare function GetV(v: Value, key: Value): Value;
/** Set ( O, P, V, true ): may call a setter; throws on failure. @reentrant */
declare function Set(o: Obj, key: Value, v: Value): void;
/** HasProperty ( O, P ): reentrant once proxies exist. @reentrant */
declare function HasProperty(o: Obj, key: Value): boolean;
/** CreateDataPropertyOrThrow ( O, P, V ): reentrant once proxies exist. @reentrant */
declare function CreateDataPropertyOrThrow(o: Obj, key: Value, v: Value): void;
/** DeletePropertyOrThrow ( O, P ): reentrant once proxies exist. @reentrant */
declare function DeletePropertyOrThrow(o: Obj, key: Value): void;

// ---- Calls ---------------------------------------------------------------------------

/** Call ( F, V, args ): TypeError if f is not callable. @reentrant */
declare function Call(f: Value, thisArg: Value, ...args: Value[]): Value;
/** Construct ( F, args ) with newTarget F: TypeError if f is not a constructor. @reentrant */
declare function Construct(f: Value, ...args: Value[]): Obj;

// ---- Objects, errors, symbols ----------------------------------------------------------

/** OrdinaryObjectCreate ( proto ). @leaf */
declare function ObjectCreate(proto: Value): Obj;
/** ArrayCreate ( length ): RangeError above 2**32 - 1. @leaf */
declare function ArrayCreate(length: f64): Obj;
/** A new TypeError of the current realm. @leaf */
declare function NewTypeError(message: Str): Obj;
/** A TypeError "<name> is not a constructor" with f's internal name, as QuickJS words it. @leaf */
declare function NewNotConstructorError(f: Value): Obj;
/** A new RangeError of the current realm. @leaf */
declare function NewRangeError(message: Str): Obj;
/** The well-known symbol Symbol.<name>. @leaf */
declare function WellKnownSymbol(name: Str): Sym;

// ---- Realm objects, own properties (0031) ------------------------------------------------

/**
 * A realm intrinsic object by name: ObjectPrototype, FunctionPrototype, ArrayPrototype,
 * ErrorPrototype and the native error prototypes (TypeErrorPrototype, ...). @leaf
 */
declare function RealmIntrinsic(name: Str): Obj;
/** An object with [[ErrorData]] and prototype `proto`. @leaf */
declare function ErrorCreate(proto: Obj): Obj;
/**
 * Defines `o.stack` (D1282) from the current frames, without the prelude frames at the
 * top: an Error constructor does not show itself. @leaf
 */
declare function CaptureStack(o: Obj): void;
/**
 * DefinePropertyOrThrow ( O, P, desc ) with a complete data descriptor; `attrs` are
 * writable 1, enumerable 2, configurable 4. @leaf
 */
declare function DefineDataPropertyOrThrow(o: Obj, key: Value, v: Value, attrs: i32): void;
/**
 * [[DefineOwnProperty]] with a descriptor record from ToPropertyDescriptor (an object
 * whose own data properties are the present fields). False if it was rejected. @leaf
 */
declare function DefineOwnPropertyFromRecord(o: Obj, key: Value, desc: Obj): boolean;
/** FromPropertyDescriptor ( [[GetOwnProperty]](o, key) ): undefined if absent. @leaf */
declare function GetOwnPropertyDescriptorObject(o: Obj, key: Value): Value;
/** HasOwnProperty ( O, P ). @leaf */
declare function HasOwnProperty(o: Obj, key: Value): boolean;
/**
 * The attributes of an own property: -1 if absent, else writable 1, enumerable 2,
 * configurable 4, accessor 8. @leaf
 */
declare function OwnPropertyAttributes(o: Obj, key: Value): i32;
/**
 * The own keys of o as a new array, in [[OwnPropertyKeys]] order: kind 0 strings,
 * 1 symbols, 2 both, 3 enumerable strings. @leaf
 */
declare function OwnPropertyKeys(o: Obj, kind: i32): Obj;

/** [[GetPrototypeOf]] ( ): an object or null. @leaf */
declare function GetPrototypeOf(o: Obj): Value;
/** [[SetPrototypeOf]] ( V ) with an object or null: false if refused. @leaf */
declare function SetPrototypeOf(o: Obj, proto: Value): boolean;
/** [[PreventExtensions]] ( ). @leaf */
declare function PreventExtensions(o: Obj): boolean;
/** [[IsExtensible]] ( ). @leaf */
declare function IsExtensible(o: Obj): boolean;
/** SetIntegrityLevel ( O, level ): sealed, or frozen if `frozen`. @leaf */
declare function SetIntegrityLevel(o: Obj, frozen: boolean): boolean;
/** TestIntegrityLevel ( O, level ): sealed, or frozen if `frozen`. @leaf */
declare function TestIntegrityLevel(o: Obj, frozen: boolean): boolean;
/** SameValue ( x, y ). @leaf */
declare function SameValue(x: Value, y: Value): boolean;
/**
 * The builtinTag of Object.prototype.toString: Array, Arguments, Function, Error,
 * Boolean, Number, String, Date, RegExp, else Object. @leaf
 */
declare function ObjectClassTag(o: Obj): Str;
/** A Boolean, Number, String or Symbol wrapper of the primitive v with prototype proto. @leaf */
declare function WrapPrimitive(v: Value, proto: Obj): Obj;
/**
 * The primitive of a wrapper object ([[BooleanData]] and so on), a primitive itself,
 * or undefined for other objects. @leaf
 */
declare function PrimitiveValue(v: Value): Value;

// ---- Functions (0031) -----------------------------------------------------------------

/** Call ( F, V, args ) with the arguments in a dense array. @reentrant */
declare function CallWithArray(f: Value, thisArg: Value, args: Obj): Value;
/** BoundFunctionCreate ( target, boundThis, boundArgs ), without name and length. @leaf */
declare function BoundFunctionCreate(target: Fn, boundThis: Value, args: Obj): Obj;
/** [[BoundTargetFunction]] of a bound function, or undefined for anything else. @leaf */
declare function BoundTargetFunction(f: Value): Value;
/** The source text of a function (natives: `function name() { [native code] }`). @leaf */
declare function FunctionToString(f: Value): Str;

// ---- Numbers, symbols (0031) ------------------------------------------------------------

/** Number::exponentiate ( base, exponent ). @leaf */
declare function NumberPow(base: f64, exponent: f64): f64;
/** SymbolDescriptiveString ( sym ): `Symbol(description)`. @leaf */
declare function SymbolDescriptiveString(s: Sym): Str;
/**
 * Compares two strings by code units (IsLessThan for strings): -1, 0 or 1. Also
 * localeCompare's order (no Intl). @leaf
 */
declare function CompareStrings(a: Str, b: Str): f64;
/** The code points of s as an array of strings (the string iterator's values). @leaf */
declare function StringCodePoints(s: Str): Obj;
/**
 * Sorts the dense array `list` in place by SortCompare without a comparefn (undefined
 * last, then ToString order; stable) if every element is a primitive other than a
 * symbol. Returns false, leaving it unchanged, otherwise. @leaf
 */
declare function SortPrimitives(list: Obj): boolean;
/**
 * Splice's element moves in one step on a plain dense Array (no holes, default
 * attributes, extensible, writable length, no index properties on the prototype chain):
 * o[start .. start + deleteCount) becomes itemCount undefined slots and `length`
 * follows. False, with nothing changed, for any other object. @leaf
 */
declare function ArraySpliceDense(o: Obj, start: f64, deleteCount: f64, itemCount: f64): boolean;
/** A new symbol; the description is a string or undefined. @leaf */
declare function NewSymbol(description: Value): Sym;
/** [[Description]] of a symbol: a string or undefined. @leaf */
declare function SymbolDescription(s: Sym): Value;
/** Symbol.for's GlobalSymbolRegistry lookup or insertion. @leaf */
declare function SymbolFor(key: Str): Sym;
/** Symbol.keyFor's registry lookup: the key, or undefined. @leaf */
declare function SymbolKeyFor(s: Sym): Value;
/**
 * Number::toString ( x, radix ) for an integral radix (RangeError outside 2..36),
 * Number.prototype.toString. @leaf
 */
declare function NumberToString(x: f64, radix: f64): Str;
/** Number.prototype.toFixed after ToIntegerOrInfinity(fractionDigits). @leaf */
declare function NumberToFixed(x: f64, digits: f64): Str;
/** Number.prototype.toExponential; digits is undefined or an integral Number. @leaf */
declare function NumberToExponential(x: f64, digits: Value): Str;
/** Number.prototype.toPrecision; precision is undefined or an integral Number. @leaf */
declare function NumberToPrecision(x: f64, precision: Value): Str;
/** The global parseFloat after ToString. @leaf */
declare function StringParseFloat(s: Str): f64;
/** The global parseInt after ToString; radix is ToInt32(radix). @leaf */
declare function StringParseInt(s: Str, radix: i32): f64;
/** ToInt32 of a number. @leaf */
declare function NumberToInt32(n: f64): i32;
/** ToUint32 of a number, as an f64. @leaf */
declare function NumberToUint32(n: f64): f64;

// ---- Math (0031): the libm functions (D1150) ---------------------------------------------

/** @leaf */
declare function F64Abs(x: f64): f64;
/** @leaf */
declare function F64Acos(x: f64): f64;
/** @leaf */
declare function F64Acosh(x: f64): f64;
/** @leaf */
declare function F64Asin(x: f64): f64;
/** @leaf */
declare function F64Asinh(x: f64): f64;
/** @leaf */
declare function F64Atan(x: f64): f64;
/** @leaf */
declare function F64Atanh(x: f64): f64;
/** @leaf */
declare function F64Atan2(y: f64, x: f64): f64;
/** @leaf */
declare function F64Cbrt(x: f64): f64;
/** @leaf */
declare function F64Hypot(x: f64, y: f64): f64;
/** @leaf */
declare function F64Ceil(x: f64): f64;
/** Math.clz32 of a number (ToUint32 inside). @leaf */
declare function F64Clz32(x: f64): f64;
/** @leaf */
declare function F64Cos(x: f64): f64;
/** @leaf */
declare function F64Cosh(x: f64): f64;
/** @leaf */
declare function F64Exp(x: f64): f64;
/** @leaf */
declare function F64Expm1(x: f64): f64;
/** @leaf */
declare function F64Floor(x: f64): f64;
/** Rounds to the nearest f32. @leaf */
declare function F64Fround(x: f64): f64;
/** Math.imul of two numbers (ToUint32 inside). @leaf */
declare function F64Imul(a: f64, b: f64): f64;
/** @leaf */
declare function F64Log(x: f64): f64;
/** @leaf */
declare function F64Log1p(x: f64): f64;
/** @leaf */
declare function F64Log10(x: f64): f64;
/** @leaf */
declare function F64Log2(x: f64): f64;
/** Math.round: half up, -0 kept. @leaf */
declare function F64Round(x: f64): f64;
/** @leaf */
declare function F64Sign(x: f64): f64;
/** @leaf */
declare function F64Sin(x: f64): f64;
/** @leaf */
declare function F64Sinh(x: f64): f64;
/** @leaf */
declare function F64Sqrt(x: f64): f64;
/** @leaf */
declare function F64Tan(x: f64): f64;
/** @leaf */
declare function F64Tanh(x: f64): f64;
/** @leaf */
declare function F64Trunc(x: f64): f64;
/** The next number in [0, 1) of the realm's generator (xorshift128+, seeded by the host). @leaf */
declare function MathRandom(): f64;

// ---- Used by the generated code (crates/djs-bgen/src/emit_js.rs) ------------------------

/**
 * Defines f as a method of the object at the dotted path `target` (from the global
 * object): writable, configurable, not enumerable, with `name` = key and `length`. The
 * key is a string or a symbol (`[Symbol.iterator]` paths).
 * @leaf
 */
declare function InstallMethod(target: Str, key: Value, f: Value, length: u32): void;
/**
 * `@constructor`: defines f as the global constructor `key` (writable, configurable, not
 * enumerable) with its prototype object: the realm's intrinsic prototype for that name
 * if there is one, else a new object inheriting from the parent's prototype (or
 * Object.prototype). Sets `f.prototype`, `prototype.constructor`, `name` and `length`;
 * with a parent (a dotted path), f's [[Prototype]] is the parent constructor.
 * @leaf
 */
declare function InstallConstructor(target: Str, key: Str, f: Value, length: u32, parent: Value): void;
/** `@getter`/`@setter`: one half of an accessor (configurable, not enumerable). @leaf */
declare function InstallAccessor(target: Str, key: Value, f: Value, isSetter: boolean): void;
/** `@value`: a data property with `attrs` (writable 1, enumerable 2, configurable 4). @leaf */
declare function InstallValue(target: Str, key: Value, v: Value, attrs: i32): void;
/** Registers f so that privileged code can call it as %name(...). @leaf */
declare function InstallIntrinsic(name: Str, f: Value): void;
