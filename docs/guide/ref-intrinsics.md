# Intrinsics

The intrinsics are the operations the builtin language calls as `%Name(...)`: Rust
functions (`leaf`: never runs user code; `reentrant`: may call one user function whose
result completes it) and JS builtins tagged `@intrinsic`. The id is the immediate of
`call_intrinsic` and part of the bytecode format. Generated from `INTRINSICS` in
`crates/djs-core/src/bytecode/intrinsics.rs`, with the descriptions from
[`builtins/intrinsics.d.ts`](../../builtins/intrinsics.d.ts).

<!-- gen:intrinsics-all -->
| id | name | kind | parameters | description |
| ---: | --- | --- | --- | --- |
| 0 | `IsObject` | leaf | 1 |  |
| 1 | `IsCallable` | leaf | 1 | IsCallable ( argument ). |
| 2 | `IsConstructor` | leaf | 1 | IsConstructor ( argument ). |
| 3 | `IsArray` | leaf | 1 | IsArray ( argument ), 7.2.2, through proxies: TypeError for a revoked proxy. |
| 4 | `IsString` | leaf | 1 |  |
| 5 | `IsNumber` | leaf | 1 |  |
| 6 | `IsSymbol` | leaf | 1 |  |
| 7 | `IsBoolean` | leaf | 1 |  |
| 8 | `ArgumentCount` | leaf | 0 | How many arguments the calling builtin got (its frame's count, before padding), for an optional parameter whose absence differs from undefined (reduce's initialValue) without a rest array. |
| 9 | `Throw` | leaf | 1 | Throws `v`: `throw` as an expression, for helpers that the bytecode backend inlines into their callers as one expression. |
| 10 | `AsObj` | leaf | 1 |  |
| 11 | `AsFn` | leaf | 1 |  |
| 12 | `AsStr` | leaf | 1 |  |
| 13 | `AsNumber` | leaf | 1 |  |
| 14 | `AsSym` | leaf | 1 |  |
| 15 | `AsBool` | leaf | 1 |  |
| 16 | `ToBoolean` | leaf | 1 | ToBoolean ( argument ). |
| 17 | `ToNumberPrimitive` | leaf | 1 | ToNumber of a primitive (TypeError for symbols); objects go through ToPrimitive first. |
| 18 | `ToStringPrimitive` | leaf | 1 | ToString of a primitive (TypeError for symbols); objects go through ToPrimitive first. |
| 19 | `ToObject` | leaf | 1 | ToObject ( argument ): TypeError for undefined and null. |
| 20 | `ToIntegerOrInfinityNumber` | leaf | 1 | ToIntegerOrInfinity of a number. |
| 21 | `F64ToU32` | leaf | 1 | An integral f64 in u32 range to u32 (an internal error otherwise). |
| 22 | `F64ToI32` | leaf | 1 | An integral f64 in i32 range to i32 (an internal error otherwise). |
| 23 | `StrLength` | leaf | 1 | The length of a string in code units. |
| 24 | `Get` | reentrant | 2 | Get ( O, P ): may call a getter. |
| 25 | `GetV` | reentrant | 2 | GetV ( V, P ): Get with a primitive base looked up on its prototype. |
| 26 | `Set` | reentrant | 3 | Set ( O, P, V, true ): may call a setter; throws on failure. |
| 27 | `HasProperty` | reentrant | 2 | HasProperty ( O, P ): a proxy's `has` trap. |
| 28 | `CreateDataPropertyOrThrow` | reentrant | 3 | CreateDataPropertyOrThrow ( O, P, V ): a proxy's `defineProperty` trap. |
| 29 | `DeletePropertyOrThrow` | reentrant | 2 | DeletePropertyOrThrow ( O, P ): a proxy's `deleteProperty` trap. |
| 30 | `Call` | reentrant | 2 + rest | Call ( F, V, args ): TypeError if f is not callable. |
| 31 | `Construct` | reentrant | 1 + rest | Construct ( F, args ) with newTarget F: TypeError if f is not a constructor. |
| 32 | `ObjectCreate` | leaf | 1 | OrdinaryObjectCreate ( proto ). |
| 33 | `ArrayCreate` | leaf | 1 | ArrayCreate ( length ): RangeError above 2**32 - 1. |
| 34 | `NewTypeError` | leaf | 1 | A new TypeError of the current realm. |
| 35 | `NewNotConstructorError` | leaf | 1 | A TypeError "<name> is not a constructor" with f's internal name, as QuickJS words it. |
| 36 | `NewSyntaxError` | leaf | 1 | A new SyntaxError of the current realm. |
| 37 | `NewReferenceError` | leaf | 1 | A new ReferenceError of the current realm. |
| 38 | `NewRangeError` | leaf | 1 | A new RangeError of the current realm. |
| 39 | `WellKnownSymbol` | leaf | 1 | The well-known symbol Symbol.<name>. |
| 40 | `RealmIntrinsic` | leaf | 1 | A realm intrinsic object by name: ObjectPrototype, FunctionPrototype, ArrayPrototype, ErrorPrototype and the native error prototypes (TypeErrorPrototype, ...). |
| 41 | `ConstructorPrototype` | leaf | 2 | GetPrototypeFromConstructor where it runs no user code: newTarget's `prototype` if that is an object, else the realm's prototype `fallback` (named as for RealmIntrinsic), also for an undefined newTarget; undefined when reading `prototype` would call a getter or a proxy trap. |
| 42 | `ErrorCreate` | leaf | 1 | An object with [[ErrorData]] and prototype `proto`. |
| 43 | `DOMExceptionCreate` | leaf | 3 | A DOMException: an object with [[ErrorData]], its name and message in slots. |
| 44 | `DOMExceptionField` | leaf | 2 | Slot i (0 name, 1 message) of a DOMException; undefined if v is not one. |
| 45 | `DOMExceptionCode` | leaf | 1 | The legacy code of a DOMException name, 0 if it has none. |
| 46 | `DOMExceptionConstant` | leaf | 1 | The name of the legacy code constant i, 1 to 25 (`INDEX_SIZE_ERR`, ...). |
| 47 | `Btoa` | leaf | 1 | btoa's base64 of the code units of s; undefined if one is above 0xFF. |
| 48 | `Atob` | leaf | 1 | atob's forgiving-base64 decoding of s into a string of bytes; undefined if invalid. |
| 49 | `CaptureStack` | leaf | 1 | Records `o.stack` from the current frames, without the prelude frames at the top: an Error constructor does not show itself. With Error.prepareStackTrace it is pending until FinishErrorStack. |
| 50 | `ErrorStack` | leaf | 1 | The stack an Error object recorded, a pending one too, or undefined for any other value. |
| 51 | `StackTraceValue` | leaf | 1 | The stack trace of the current frames (the calling builtin left out): a string, or a pending stack with Error.prepareStackTrace. With a function `below`, only the frames below its topmost call, none if it is not on the stack (Error.captureStackTrace). |
| 52 | `IsPendingStack` | leaf | 1 | Whether v is a stack waiting for Error.prepareStackTrace. |
| 53 | `SetErrorStack` | leaf | 2 | Sets the stack slot of an Error (an own `stack` of anything else). |
| 54 | `CallSiteSlot` | leaf | 2 | Slot i of a CallSite (0 function, 1 name, 2 file, 3 line, 4 column, 5 native, 6 constructor); TypeError "CallSite object expected" for anything else. |
| 55 | `DefineDataPropertyOrThrow` | reentrant | 4 | DefinePropertyOrThrow ( O, P, desc ) with a complete data descriptor; `attrs` are writable 1, enumerable 2, configurable 4. |
| 56 | `DefineOwnPropertyFromRecord` | reentrant | 3 | [[DefineOwnProperty]] with a descriptor record from ToPropertyDescriptor (an object whose own data properties are the present fields). False if it was rejected. |
| 57 | `GetOwnPropertyDescriptorObject` | reentrant | 2 | FromPropertyDescriptor ( [[GetOwnProperty]](o, key) ): undefined if absent. |
| 58 | `HasOwnProperty` | reentrant | 2 | HasOwnProperty ( O, P ). |
| 59 | `OwnPropertyAttributes` | reentrant | 2 | The attributes of an own property: -1 if absent, else writable 1, enumerable 2, configurable 4, accessor 8. |
| 60 | `OwnPropertyKeys` | reentrant | 2 | The own keys of o as a new array, in [[OwnPropertyKeys]] order: kind 0 strings, 1 symbols, 2 both, 3 enumerable strings. |
| 61 | `GetPrototypeOf` | reentrant | 1 | [[GetPrototypeOf]] ( ): an object or null. |
| 62 | `SetPrototypeOf` | reentrant | 2 | [[SetPrototypeOf]] ( V ) with an object or null: false if refused. |
| 63 | `PreventExtensions` | reentrant | 1 | [[PreventExtensions]] ( ). |
| 64 | `IsExtensible` | reentrant | 1 | [[IsExtensible]] ( ). |
| 65 | `SetIntegrityLevel` | reentrant | 2 | SetIntegrityLevel ( O, level ): sealed, or frozen if `frozen`. |
| 66 | `TestIntegrityLevel` | reentrant | 2 | TestIntegrityLevel ( O, level ): sealed, or frozen if `frozen`. |
| 67 | `SameValue` | leaf | 2 | SameValue ( x, y ). |
| 68 | `ObjectClassTag` | leaf | 1 | The builtinTag of Object.prototype.toString: Array, Arguments, Function, Error, Boolean, Number, String, Date, RegExp, else Object. |
| 69 | `WrapPrimitive` | leaf | 2 | A Boolean, Number, String or Symbol wrapper of the primitive v with prototype proto. |
| 70 | `PrimitiveValue` | leaf | 1 | The primitive of a wrapper object ([[BooleanData]] and so on), a primitive itself, or undefined for other objects. |
| 71 | `ConstructWithArray` | reentrant | 3 | Construct ( F, args, newTarget ) with the arguments in a dense array. |
| 72 | `ConstructWithThis` | reentrant | 4 | [[Construct]] of an ordinary (non-builtin) function f with `thisArg` as the new object (OrdinaryCreateFromConstructor done by the caller) and new.target. |
| 73 | `GetWithReceiver` | reentrant | 3 | [[Get]] ( P, Receiver ): a getter runs with `receiver` as this. |
| 74 | `SetWithReceiver` | reentrant | 4 | OrdinarySet ( O, P, V, Receiver ): false when refused; a setter runs with receiver. |
| 75 | `DeleteProperty` | reentrant | 2 | [[Delete]] ( P ): false for a non-configurable property. |
| 76 | `IsProxy` | leaf | 1 | Whether v is a Proxy exotic object (revoked or not). |
| 77 | `NewProxy` | leaf | 2 | A new proxy (ProxyCreate's object, 10.5.14): callable/constructor as target is. |
| 78 | `ProxyTarget` | leaf | 1 | [[ProxyTarget]] of a proxy: TypeError "revoked proxy" once revoked. |
| 79 | `ProxyHandler` | leaf | 1 | [[ProxyHandler]] of a proxy: TypeError "revoked proxy" once revoked. |
| 80 | `ProxyRevoke` | leaf | 1 | Revokes a proxy (target and handler become null). |
| 81 | `CollectionCreate` | leaf | 2 | A new empty collection: class 0 Map, 1 Set, 2 WeakMap, 3 WeakSet. |
| 82 | `CollectionClass` | leaf | 1 | The class of a collection object (0 Map, 1 Set, 2 WeakMap, 3 WeakSet), -1 otherwise. |
| 83 | `CollectionGet` | leaf | 2 | The value of key (SameValueZero), or undefined. |
| 84 | `CollectionHas` | leaf | 2 | Whether key is present. |
| 85 | `CollectionSet` | leaf | 3 | Adds or replaces key (-0 becomes +0); a Set passes true as the value. |
| 86 | `CollectionDelete` | leaf | 2 | Removes key; false if it was absent. |
| 87 | `CollectionClear` | leaf | 1 | Removes every entry. |
| 88 | `CollectionSize` | leaf | 1 | The number of entries. |
| 89 | `CollectionIterator` | leaf | 2 | A Map Iterator or Set Iterator over m (kind 0 keys, 1 values, 2 entries): it visits entries added while iterating and skips deleted ones. |
| 90 | `CollectionIteratorNext` | leaf | 2 | %MapIteratorPrototype%.next / %SetIteratorPrototype%.next: the iterator result. TypeError "Map Iterator object expected" (or Set) for anything else. |
| 91 | `IsOriginalCollectionAdder` | leaf | 2 | Whether `adder` is the installed Map.prototype.set (kind 0) or Set.prototype.add (1): the constructors then store directly. |
| 92 | `IsArrayConstructor` | leaf | 1 | Whether `v` is the realm's %Array%: Array.from then makes its array itself, as Construct(%Array%, ..) would (its `prototype` cannot change). |
| 93 | `CollectionIteratorStep` | leaf | 1 | Advances an iterator to the next entry; false when done. |
| 94 | `CollectionIteratorKey` | leaf | 1 | The key of the entry the last step reached. |
| 95 | `CollectionIteratorValue` | leaf | 1 | The value of the entry the last step reached. |
| 96 | `CanBeHeldWeakly` | leaf | 1 | CanBeHeldWeakly ( v ): an object, or a symbol not in the Symbol.for registry. |
| 97 | `DateNow` | leaf | 0 | The host's clock (ms since the epoch; Vm::set_now). |
| 98 | `DateCreate` | leaf | 2 | A new Date object with prototype proto and time value t. |
| 99 | `DateValue` | leaf | 1 | thisTimeValue: the time value of a Date, TypeError "not a Date object" otherwise. |
| 100 | `DateSetValue` | leaf | 2 | Stores time value t in a Date (TypeError "not a Date object" otherwise); returns t. |
| 101 | `DateFromFields` | leaf | 2 | A time value from the Numbers year, month[, date, hours, minutes, seconds, ms] in a dense array (the constructor's and Date.UTC's form: years 0-99 are 1900-1999). |
| 102 | `DateGetField` | leaf | 3 | Field `field` of t (0 year, 1 month, 2 date, 3 hours, 4 minutes, 5 seconds, 6 ms, 7 weekday), local or UTC; NaN for an invalid date. |
| 103 | `DateSetFields` | leaf | 4 | A setter's new time value: fields first.. of t replaced by the Numbers in args (a dense array), local or UTC (qjs's set_date_field). |
| 104 | `DateFormat` | leaf | 3 | Formats t: fmt 0 toUTCString, 1 toString, 2 toISOString (RangeError for NaN), 3 toLocale*; part 1 date, 2 time, 3 both. "Invalid Date" for NaN. |
| 105 | `DateParse` | leaf | 1 | Date.parse of a string (ISO and the formats qjs's toString/toUTCString emit). |
| 106 | `DateTimezoneOffset` | leaf | 1 | getTimezoneOffset of t: minutes (UTC - local), NaN for an invalid date. |
| 107 | `ArrayBufferCreate` | leaf | 3 | AllocateArrayBuffer: byteLength zeroed bytes; max is the maxByteLength of a resizable buffer, true for an immutable one, or undefined. RangeError "invalid array buffer length" (or "... max length"). |
| 108 | `BufferInfo` | leaf | 2 | A field of an ArrayBuffer: 0 byteLength, 1 maxByteLength, 2 resizable, 3 detached, 4 immutable. TypeError "ArrayBuffer object expected" for anything else. |
| 109 | `BufferDetach` | leaf | 1 | DetachArrayBuffer. |
| 110 | `BufferResize` | leaf | 2 | Resizes a resizable buffer in place (new bytes are 0). |
| 111 | `BufferTransfer` | leaf | 3 | A new buffer with the first bytes of b (max: a Number for resizable, true for immutable), b detached; an immutable b is a TypeError. |
| 112 | `BufferCopy` | leaf | 5 | Copies count bytes from src at from to dst at to (memmove). |
| 113 | `SortNumbers` | leaf | 1 | Sorts a dense array of Numbers numerically in place (a typed array's default order: -0 before +0, NaN last; stable), or one of BigInts by value. |
| 114 | `DataViewCreate` | leaf | 4 | A DataView over buffer (byteLength -1: length-tracking). |
| 115 | `DataViewField` | leaf | 2 | A slot of a DataView: 0 buffer, 1 byteOffset, 2 byteLength (-1: length-tracking). TypeError "DataView object expected" for anything else. |
| 116 | `DataViewGet` | leaf | 4 | The element of kind at byte `at` of the view's buffer (bounds checked by the caller): a Number, or a BigInt for kinds 10 and 11. |
| 117 | `DataViewSet` | leaf | 5 | Writes v (a Number, or a BigInt for kinds 10 and 11) as kind at byte `at` of the view's buffer. |
| 118 | `IsDataView` | leaf | 1 | Whether v is a DataView. |
| 119 | `RealmConstructor` | leaf | 1 | The constructor installed as `name` (the realm's %Name%, whatever the global is now). |
| 120 | `IsTypedArray` | leaf | 1 | Whether v is a typed array. |
| 121 | `TypedArrayCreate` | leaf | 5 | A typed array of `kind` over buffer (length -1: length-tracking). |
| 122 | `TypedArrayField` | leaf | 2 | A field of a typed array: 0 kind, 1 buffer, 2 byteOffset, 3 length, 4 byteLength (2-4 are 0 when out of bounds), 5 out of bounds or detached, 6 length-tracking, 7 [[ByteOffset]] (also when out of bounds). TypeError "not a TypedArray" for anything else. |
| 123 | `TypedArrayGet` | leaf | 2 | Element i (undefined if not a valid index). |
| 124 | `TypedArraySet` | leaf | 3 | Writes v (a Number, or a BigInt for a BigInt array) to element i (nothing if not a valid index). |
| 125 | `TypedArrayCopy` | leaf | 5 | Copies count elements from src (at from) to dst (at to), converting; overlaps ok. |
| 126 | `TypedArrayKindProto` | leaf | 1 | The prototype of the typed array constructor for kind (the realm's). |
| 127 | `Base64Encode` | leaf | 3 | The bytes of a Uint8Array as base64 (TypeError if detached or out of bounds). |
| 128 | `HexEncode` | leaf | 1 | The bytes of a Uint8Array as lowercase hex (TypeError if detached). |
| 129 | `Base64Decode` | leaf | 4 | FromBase64 ( string, alphabet, lastChunkHandling, maxLength ): a record object { read, bytes (an ArrayBuffer), error } (last: 0 loose, 1 strict, 2 stop-before-partial). |
| 130 | `HexDecode` | leaf | 2 | FromHex ( string, maxLength ): the same record. |
| 131 | `CallWithArray` | reentrant | 3 | Call ( F, V, args ) with the arguments in a dense array. |
| 132 | `BoundFunctionCreate` | leaf | 3 | BoundFunctionCreate ( target, boundThis, boundArgs ), without name and length. |
| 133 | `BoundTargetFunction` | leaf | 1 | [[BoundTargetFunction]] of a bound function, or undefined for anything else. |
| 134 | `FunctionToString` | leaf | 1 | The source text of a function (natives: `function name() { [native code] }`). |
| 135 | `NumberPow` | leaf | 2 | Number::exponentiate ( base, exponent ). |
| 136 | `SymbolDescriptiveString` | leaf | 1 | SymbolDescriptiveString ( sym ): `Symbol(description)`. |
| 137 | `CompareStrings` | leaf | 2 | Compares two strings by code units (IsLessThan for strings): -1, 0 or 1. Also localeCompare's order (no Intl). |
| 138 | `StringCodePoints` | leaf | 1 | The code points of s as an array of strings (the string iterator's values). |
| 139 | `SortPrimitives` | leaf | 1 | Sorts the dense array `list` in place by SortCompare without a comparefn (undefined last, then ToString order; stable) if every element is a primitive other than a symbol. Returns false, leaving it unchanged, otherwise. |
| 140 | `SortStart` | leaf | 1 | The state of a stable bottom-up merge sort of the dense array `list` (made by SortList, never seen by user code) by SortCompare: its undefined elements move to the end of the list (they sort last and are never compared), the others into the state. |
| 141 | `SortStep` | leaf | 2 | Places the pending comparison's element by its result v, a Number (the right one if v > 0), then merges on to the next comparison: true if there is one (SortOperand reads it), false once the list is sorted (written back into it). |
| 142 | `SortOperand` | leaf | 2 | The left (0) or right (1) value of the pending comparison of SortStep. |
| 143 | `ArraySpliceDense` | leaf | 4 | Splice's element moves in one step on a plain dense Array (no holes, default attributes, extensible, writable length, no index properties on the prototype chain): o[start .. start + deleteCount) becomes itemCount undefined slots and `length` follows. False, with nothing changed, for any other object. |
| 144 | `ArrayCanPopDense` | leaf | 1 | Whether `ArrayPopDense(o)` applies: an Array with only dense elements, a writable length, its last element present and no index properties on its prototype chain, where pop's Get, DeletePropertyOrThrow and Set observe nothing else. |
| 145 | `ArrayPopDense` | leaf | 1 | Array.prototype.pop of an array `ArrayCanPopDense` accepts. |
| 146 | `ArrayFillDense` | leaf | 4 | fill's Sets on an Array while each is a plain store (an existing dense element, or an append or hole fill of an extensible Array without index properties on its prototype chain), from k up to fin: the index the general loop continues from. |
| 147 | `ArrayPushDense` | leaf | 2 | push(v)'s Set and length update in one step when the Set is a plain append to the Array `o` (extensible, writable length, no holes at its end, no index properties on its prototype chain). The new length, or -1 with nothing changed. |
| 148 | `ArraySearchDense` | leaf | 5 | The search loops of indexOf (mode 0: `from` up to len, strict equality), lastIndexOf (1: `from` down to 0) and includes (2: up to len, SameValueZero) over an Array whose elements in the range are all present in dense storage, where HasProperty and Get observe nothing: the index found or -1. -2, for the general loop, for anything else (a hole, a shorter array, another object). |
| 149 | `ArraySliceDense` | leaf | 4 | slice's copy loop: `a[0 .. fin - k) = o[k .. fin)` when every element of `o` in the range is present in dense storage and `a` is an extensible Array without index properties (a fresh ArraySpeciesCreate result), so HasProperty, Get and CreateDataPropertyOrThrow observe nothing. False, with nothing changed, otherwise. |
| 150 | `EachDense` | reentrant | 7 | The dense part of the loops of the callback builtins. From `k`, while `o` is an Array whose element k is present in its dense store (so HasProperty and Get observe nothing), calls `f(o[k], k, o)` with `thisArg` and completes the step by `mode`: - 0 forEach; - 1 map: CreateDataPropertyOrThrow(a, k, result); - 2 filter: for a truthy result, CreateDataPropertyOrThrow(a, to, o[k]), with `to` the length of `a`, a fresh Array (the receiver's species is plain); - 3 some, 4 every: a truthy (some) or falsy (every) result decides; - 5 find: a truthy result decides, and o[k] goes to element 0 of the Array `a`; - 6 findIndex: a truthy result decides; - 7 reduce: calls `f(acc, o[k], k, o)` with `this` undefined; the accumulator starts as `thisArg`, and each result goes to element 0 of the Array `a` (so `a` holds the accumulator once the returned k is past the starting one); - 8 Array.from of an array-like: calls `f(o[k], k)` and does CreateDataPropertyOrThrow(a, k, result); `o` is any ordinary object (or array or function) whose element k, along its prototype chain, is a data property or absent (undefined), so Get observes nothing. One frame for `f`, entered again per element. Returns the k the builtin's own loop goes on from (`len`: done; `k` itself when `f` is not a plain closure or `a` is not as required), or -1 - k when the result at k decided. |
| 151 | `ArrayJoinDense` | leaf | 3 | join over o[0 .. len) when every element is present in dense storage and a primitive other than a symbol (ToString runs no user code): the string, or undefined for the general loop. |
| 152 | `EnumerableOwnDataProperties` | leaf | 2 | EnumerableOwnProperties ( o, kind ) for kind 1 (values) and 2 (entries) in one step when no enumerable string-keyed own property is an accessor and `o` is not exotic (no Proxy): the array, or undefined for the general loop. |
| 153 | `TypedArraySetFromDense` | leaf | 2 | Writes values[0 .. length) into the typed array `o` (at least that long) when `values` is an Array whose elements are all present in dense storage and primitives of `o`'s content type (ToNumber / ToBigInt run no user code). False, with nothing written, otherwise. |
| 154 | `StrLocaleCompare` | leaf | 2 | localeCompare without Intl (as qjs): the difference of the first differing code points, else -1, 0 or 1 by length. |
| 155 | `StrCharCodeAt` | leaf | 2 | The code unit at i as a Number, NaN out of range. |
| 156 | `StrCodePointAt` | leaf | 2 | The code point at i (a surrogate pair combined), undefined out of range. |
| 157 | `StrSubstring` | leaf | 3 | s[start .. end) in code units; the indexes are integers, clamped by the caller. |
| 158 | `StrIndexOf` | leaf | 3 | The first index ≥ from where search occurs, or -1. |
| 159 | `StrLastIndexOf` | leaf | 3 | The last index ≤ from where search occurs, or -1. |
| 160 | `StrToCase` | leaf | 2 | Full Unicode upper- or lowercase (SpecialCasing, Final_Sigma; no locales). |
| 161 | `StrTrim` | leaf | 2 | Trims WhiteSpace and LineTerminators: where 0 both ends, 1 start, 2 end. |
| 162 | `StrIsWellFormed` | leaf | 1 | IsStringWellFormedUnicode. |
| 163 | `StrToWellFormed` | leaf | 1 | Lone surrogates replaced by U+FFFD. |
| 164 | `StrRepeat` | leaf | 2 | s repeated n times (an integer ≥ 0; RangeError past the string length limit). |
| 165 | `StrFromCodeUnits` | leaf | 1 | A string of the code units ToUint16(n) of the Numbers in a dense array. |
| 166 | `StrFromCodePoints` | leaf | 1 | A string of the (valid) code points in a dense array. |
| 167 | `StrNormalize` | leaf | 2 | String.prototype.normalize for form 0 NFC, 1 NFD, 2 NFKC, 3 NFKD: the normalized string; undefined only when djs is built without the `unicode-normalization` feature and the string would need its tables. |
| 168 | `NewSymbol` | leaf | 1 | A new symbol; the description is a string or undefined. |
| 169 | `SymbolDescription` | leaf | 1 | [[Description]] of a symbol: a string or undefined. |
| 170 | `SymbolFor` | leaf | 1 | Symbol.for's GlobalSymbolRegistry lookup or insertion. |
| 171 | `SymbolKeyFor` | leaf | 1 | Symbol.keyFor's registry lookup: the key, or undefined. |
| 172 | `NumberToString` | leaf | 2 | Number::toString ( x, radix ) for an integral radix (RangeError outside 2..36), Number.prototype.toString. |
| 173 | `NumberToFixed` | leaf | 2 | Number.prototype.toFixed after ToIntegerOrInfinity(fractionDigits). |
| 174 | `NumberToExponential` | leaf | 2 | Number.prototype.toExponential; digits is undefined or an integral Number. |
| 175 | `NumberToPrecision` | leaf | 2 | Number.prototype.toPrecision; precision is undefined or an integral Number. |
| 176 | `StringParseFloat` | leaf | 1 | The global parseFloat after ToString. |
| 177 | `StringParseInt` | leaf | 2 | The global parseInt after ToString; radix is ToInt32(radix). |
| 178 | `UriEncode` | leaf | 2 | Encode ( string, extraUnescaped ), 19.2.6.5, after ToString: encodeURI keeps the reserved characters (`component` false), encodeURIComponent escapes them. URIError for a lone surrogate. |
| 179 | `UriDecode` | leaf | 2 | Decode ( string, preserveEscapeSet ), 19.2.6.6, after ToString: decodeURI keeps the escapes of reserved characters (`component` false). URIError for a malformed escape or UTF-8 sequence. |
| 180 | `StrEscape` | leaf | 1 | escape ( string ), B.2.1.1, after ToString. |
| 181 | `StrUnescape` | leaf | 1 | unescape ( string ), B.2.1.2, after ToString. |
| 182 | `CompileEval` | leaf | 1 | PerformEval's parse (19.2.1.1) for indirect eval: the source as a Script closure, to be called with `this` = the global object. SyntaxError for invalid source. Charges fuel per source byte. |
| 183 | `CompileFunction` | leaf | 3 | CreateDynamicFunction's parse (20.2.1.1.1): `kind` 0 function, 1 generator, 2 async, 3 async generator; `params` joined with ","; a Script closure whose call returns the new function. SyntaxError for invalid source. Charges fuel per source byte. |
| 184 | `GlobalObject` | leaf | 0 | The realm's global object. |
| 185 | `EnqueueMicrotask` | leaf | 1 | Queues the job MicrotaskJob(callback) (queueMicrotask). |
| 186 | `NumberToInt32` | leaf | 1 | ToInt32 of a number. |
| 187 | `NumberToUint32` | leaf | 1 | ToUint32 of a number, as an f64. |
| 188 | `F64Abs` | leaf | 1 |  |
| 189 | `F64Acos` | leaf | 1 |  |
| 190 | `F64Acosh` | leaf | 1 |  |
| 191 | `F64Asin` | leaf | 1 |  |
| 192 | `F64Asinh` | leaf | 1 |  |
| 193 | `F64Atan` | leaf | 1 |  |
| 194 | `F64Atanh` | leaf | 1 |  |
| 195 | `F64Atan2` | leaf | 2 |  |
| 196 | `F64Cbrt` | leaf | 1 |  |
| 197 | `F64Hypot` | leaf | 2 |  |
| 198 | `F64Ceil` | leaf | 1 |  |
| 199 | `F64Clz32` | leaf | 1 | Math.clz32 of a number (ToUint32 inside). |
| 200 | `F64Cos` | leaf | 1 |  |
| 201 | `F64Cosh` | leaf | 1 |  |
| 202 | `F64Exp` | leaf | 1 |  |
| 203 | `F64Expm1` | leaf | 1 |  |
| 204 | `F64Floor` | leaf | 1 |  |
| 205 | `F64Fround` | leaf | 1 | Rounds to the nearest f32. |
| 206 | `F64F16round` | leaf | 1 | Rounds to the nearest binary16 value (Math.f16round). |
| 207 | `F64SumPrecise` | leaf | 1 | The exactly rounded sum of the Numbers of the dense Array list (Math.sumPrecise). |
| 208 | `F64Imul` | leaf | 2 | Math.imul of two numbers (ToUint32 inside). |
| 209 | `F64Log` | leaf | 1 |  |
| 210 | `F64Log1p` | leaf | 1 |  |
| 211 | `F64Log10` | leaf | 1 |  |
| 212 | `F64Log2` | leaf | 1 |  |
| 213 | `F64Round` | leaf | 1 | Math.round: half up, -0 kept. |
| 214 | `F64Sign` | leaf | 1 |  |
| 215 | `F64Sin` | leaf | 1 |  |
| 216 | `F64Sinh` | leaf | 1 |  |
| 217 | `F64Sqrt` | leaf | 1 |  |
| 218 | `F64Tan` | leaf | 1 |  |
| 219 | `F64Tanh` | leaf | 1 |  |
| 220 | `F64Trunc` | leaf | 1 |  |
| 221 | `MathRandom` | leaf | 0 | The next number in [0, 1) of the realm's generator (xorshift128+, seeded by the host). |
| 222 | `CreateArrayIterator` | leaf | 2 | A new Array Iterator over o (kind 0: keys, 1: values, 2: entries), 23.1.5.1. |
| 223 | `ArrayIteratorTarget` | leaf | 1 | The object an Array Iterator iterates, or undefined once it is exhausted. TypeError "Array Iterator object expected" if it is not an Array Iterator. |
| 224 | `ArrayIteratorIndex` | leaf | 1 | The next index of an Array Iterator. |
| 225 | `ArrayIteratorSetIndex` | leaf | 2 |  |
| 226 | `ArrayIteratorFinish` | leaf | 1 | Marks an Array Iterator exhausted (its target becomes undefined). |
| 227 | `ArrayIteratorKind` | leaf | 1 | The kind of an Array Iterator (0: keys, 1: values, 2: entries). |
| 228 | `CreateStringIterator` | leaf | 1 | A String Iterator over s (CreateStringIterator, 22.1.5.1): an Array Iterator object of kind 3 with %StringIteratorPrototype%; ArrayIteratorIndex/SetIndex/Finish apply. |
| 229 | `StringIteratorTarget` | leaf | 1 | The string a String Iterator iterates, or undefined once it is exhausted. TypeError "String Iterator object expected" otherwise. |
| 230 | `StrCodePointString` | leaf | 2 | The code point at index i of s as a string (one or two code units). |
| 231 | `HasOriginalIteration` | leaf | 1 | Whether v is an Array or a string whose iteration is the original Symbol.iterator method and `next` (data properties only): iterating it then equals reading it directly. |
| 232 | `IteratorFastReady` | leaf | 2 | Whether the next step of `iterator` with `next` gives a value without running user code: `next` is the original one of an Array or String Iterator and the value is at hand. Then `IteratorFastValue` returns it; otherwise call `next`. |
| 233 | `IteratorFastValue` | leaf | 2 | The next value of an iterator `IteratorFastReady` accepts (it advances). |
| 234 | `IteratorFastDrain` | leaf | 4 | The values `IteratorFastValue` would give one by one for a keys or values Array Iterator, stored with CreateDataPropertyOrThrow into the Array `arr` at n, n + 1, ... in one step, while they are plain dense stores: the next n. Stops before a value only `next` can read (a hole), at the end (the next step finishes the iterator) and where a store is not plain; gives n back unchanged for anything else. |
| 235 | `CreateIterResultObject` | leaf | 2 | CreateIterResultObject ( value, done ), 7.4.14. |
| 236 | `CreateArrayFromPair` | leaf | 2 | A new Array [a, b]. |
| 237 | `PromiseCreate` | leaf | 1 | A new pending Promise with prototype proto. |
| 238 | `IsPromise` | leaf | 1 | Whether v is a Promise object (has [[PromiseState]]). |
| 239 | `IntrinsicPromise` | leaf | 0 | The realm's %Promise% constructor. |
| 240 | `PromiseState` | leaf | 1 | [[PromiseState]]: 0 pending, 1 fulfilled, 2 rejected. |
| 241 | `PromiseResult` | leaf | 1 | [[PromiseResult]]. |
| 242 | `FulfillPromise` | leaf | 2 | FulfillPromise ( promise, value ), 27.2.1.4: reactions become jobs. No-op unless pending. |
| 243 | `RejectPromise` | leaf | 2 | RejectPromise ( promise, reason ), 27.2.1.7, with the rejection tracker. No-op unless pending. |
| 244 | `PromiseClaimResolve` | leaf | 1 | Sets the [[AlreadyResolved]] of p's own resolving functions (and internal capability); false if it was already set. |
| 245 | `PerformPromiseThen` | leaf | 4 | PerformPromiseThen ( promise, onFulfilled, onRejected, resultCapability ), 27.2.5.4.1, without the result: handlers that are not callable count as empty. |
| 246 | `PromiseCombinatorRun` | leaf | 5 | The elements of PerformPromiseCombinator (Promise.all kind 0, allSettled 1, any 2, race 3) that an Array values iterator gives without user code (as `IteratorFastValue` would), one after another while their steps run no user code either: `fixed` is the Record [C, promiseResolve, capability, values, remaining, resolveFn, rejectFn], C %Promise% with the original `resolve`, and the element a Promise of %Promise% or a value that fulfills one without a `then` call, whose `then`, `constructor` and %Promise%[@@species] are the original ones. Each is done as the loop body would (values slot, element Record, functions, `then`); returns the next index. |
| 247 | `EnqueuePromiseResolveThenableJob` | leaf | 3 | Queues NewPromiseResolveThenableJob ( promise, thenable, then ), 27.2.2.2. |
| 248 | `NewRecord` | leaf | 1 | A new internal Record with n slots, all undefined. |
| 249 | `RecordGet` | leaf | 2 | Slot i of a Record, or of an object that keeps its state in slots (IteratorHelper). |
| 250 | `RecordSet` | leaf | 3 |  |
| 251 | `NewBuiltinClosure` | leaf | 3 | A function (name "", `length`) that calls the builtin `kind` (the table in interp/run/intrinsics.rs: 0 PromiseResolveFunction, 1 PromiseRejectFunction, 2 GetCapabilitiesExecutor, 3 PromiseAllResolveElement, 4 PromiseAllSettledResolveElement, 5 PromiseAllSettledRejectElement, 6 PromiseAnyRejectElement, 7 PromiseThenFinally, 8 PromiseCatchFinally, 9 PromiseValueThunk, 10 PromiseThrower, 11 AsyncGeneratorAwaitReturnFulfilled, 12 AsyncGeneratorAwaitReturnRejected, 13 AsyncFromSyncIteratorUnwrap, 14 AsyncFromSyncIteratorCloseThrow, 15 ProxyRevokeFunction, 16 FromAsyncFulfilled, 17 FromAsyncRejected, 18 AsyncDisposeFulfilled, 19 AsyncDisposeRejected, 20 DecoratorAddInitializer, 21 DecoratorAccessGet, 22 DecoratorAccessSet, 23 DecoratorAccessHas, 24 DecoratorParamCall) with `this` = data and its own arguments. |
| 252 | `GeneratorState` | leaf | 1 | The state of a generator object (not an async state), or -1 for anything else. |
| 253 | `GeneratorSetState` | leaf | 2 |  |
| 254 | `GeneratorResume` | reentrant | 3 | Resumes a suspended generator or async function (state 0 or 1) in a frame of its own: mode 0 continues with value (a generator at a yield gets `value false`, an async function the value), 1 throws value at the suspension point, 2 (generators at a yield) returns value from there (`value true`). The result is what the activation hands back when it next suspends or completes. |
| 255 | `IsAsyncFunctionState` | leaf | 1 | Whether v is the state of an async function call. |
| 256 | `AsyncFunctionPromise` | leaf | 1 | The result promise of an async function call. |
| 257 | `AsyncGeneratorState` | leaf | 1 | The state of an async generator, or -1 for anything else. |
| 258 | `AsyncGeneratorEnqueue` | leaf | 2 | Appends a request to the generator's queue. |
| 259 | `AsyncGeneratorFirstRequest` | leaf | 1 | The oldest request, or undefined. |
| 260 | `AsyncGeneratorDequeue` | leaf | 1 | Removes and returns the oldest request (undefined if none). |
| 261 | `CreateAsyncFromSyncIterator` | leaf | 2 | CreateAsyncFromSyncIterator ( syncIteratorRecord ), 27.1.6.1: a Record [iterator, next] with %AsyncFromSyncIteratorPrototype%. |
| 262 | `CreateIteratorHelper` | leaf | 3 | A new IteratorHelper object of `kind` (0: Iterator.from's wrapper, with %WrapForValidIteratorPrototype%; else a helper, with %IteratorHelperPrototype%): slot 0 kind, 1 state 0, 2 iterator, 3 next, the others undefined. |
| 263 | `ThisIteratorHelper` | leaf | 2 | v if it is an iterator helper (wrap false) or Iterator.from's wrapper (wrap true), else TypeError "Iterator Helper object expected" / "Iterator Wrap object expected". |
| 264 | `WeakRefCreate` | leaf | 2 | A new WeakRef with prototype proto and target (AddToKeptObjects ( target ) too). |
| 265 | `WeakRefDeref` | leaf | 1 | WeakRefDeref of v (AddToKeptObjects of a live target): the target, or undefined once it died. TypeError "WeakRef object expected" for anything else. |
| 266 | `FinalizationRegistryCreate` | leaf | 2 | A new FinalizationRegistry with prototype proto and cleanupCallback. |
| 267 | `ThisFinalizationRegistry` | leaf | 1 | v if it is a FinalizationRegistry, else TypeError "FinalizationRegistry object expected". |
| 268 | `FinalizationRegistryRegister` | leaf | 4 | Appends a cell (token undefined for none). |
| 269 | `FinalizationRegistryUnregister` | leaf | 2 | Removes the cells registered with token; whether there were any. |
| 270 | `FinalizationRegistryNextDead` | leaf | 1 | The index of the first cell whose target died, or -1. |
| 271 | `FinalizationRegistryTake` | leaf | 2 | The held value of dead cell i, removing the cell. |
| 272 | `FinalizationRegistrySetQueued` | leaf | 2 | Marks whether a cleanup job for r is queued. |
| 273 | `DisposableStackCreate` | leaf | 2 | A new DisposableStack (isAsync false) or AsyncDisposableStack with prototype proto: slot 0 isAsync, 1 state 0 (pending), 2 a new empty Array of resources. |
| 274 | `ThisDisposableStack` | leaf | 2 | v if it is a DisposableStack (isAsync false) or an AsyncDisposableStack, else TypeError "DisposableStack object expected" / "AsyncDisposableStack object expected". |
| 275 | `JsonParse` | leaf | 2 | JSON.parse of text without a reviver; qjs's SyntaxError for invalid JSON. With withSource, an Array [value, record] where record is the JSON Parse Record tree for the reviver's source text access: [value, source, elements, entries]. |
| 276 | `JsonRawJSON` | leaf | 1 | JSON.rawJSON's work after ToString: a frozen, null-prototype RawJSON object with the own property rawJSON = text; SyntaxError "invalid rawJSON string" unless text is a JSON primitive without leading or trailing whitespace. |
| 277 | `IsRawJSON` | leaf | 1 | Whether v has [[IsRawJSON]] (a JSON.rawJSON object). |
| 278 | `JsonStringifyFast` | leaf | 2 | JSON.stringify(v, undefined, space) with gap already computed, for plain data: the string, undefined for an undefined result, or null when the value needs the full algorithm (accessors, toJSON, wrappers, exotic objects; nothing observable happened). TypeError "circular reference" and RangeError "Maximum call stack size exceeded" are final. |
| 279 | `JsonStringifyFastAt` | leaf | 4 | JsonStringifyFast for an object the full algorithm reached, after its toJSON and replacer (its own toJSON is not looked at again), nested in the objects of the algorithm's `stack` (a cycle back to one throws; they set the indentation), with an array replacer's PropertyList or undefined: the string, or null when it is not plain data. |
| 280 | `JsonQuote` | leaf | 1 | QuoteJSONString ( value ), 25.5.2.3. |
| 281 | `CreateDataProperty` | reentrant | 3 | CreateDataProperty ( O, P, V ): false if refused. |
| 282 | `IsBigInt` | leaf | 1 |  |
| 283 | `ToBigIntPrimitive` | leaf | 1 | ToBigInt of a primitive: TypeError for undefined, null, numbers and symbols; SyntaxError for a bad string. |
| 284 | `NumberToBigInt` | leaf | 1 | NumberToBigInt ( number ), 21.2.1.1.1: RangeError unless integral. |
| 285 | `BigIntToString` | leaf | 2 | The digits of a BigInt in radix 2..36. |
| 286 | `BigIntAsIntN` | leaf | 2 | BigInt.asIntN on an index and a BigInt. |
| 287 | `BigIntAsUintN` | leaf | 2 | BigInt.asUintN on an index and a BigInt. |
| 288 | `BigIntToNumber` | leaf | 1 | The Number value of a BigInt (rounded to nearest). |
| 289 | `IsRegExpObject` | leaf | 1 | Whether v is a RegExp object (has [[RegExpMatcher]]). |
| 290 | `RegExpCreate` | leaf | 3 | RegExpAlloc + RegExpInitialize after the ToStrings: a new RegExp with prototype proto (%RegExp.prototype% for undefined) and lastIndex 0. qjs's SyntaxError for a bad pattern or flags. Regex literals compile to this. The pattern may be a RegExp object instead of its source: with the same flags the new one shares its program. |
| 291 | `RegExpSource` | leaf | 1 | [[OriginalSource]] of a RegExp object. |
| 292 | `RegExpFlags` | leaf | 1 | [[OriginalFlags]] of a RegExp object, in canonical order. |
| 293 | `RegExpHasFlag` | leaf | 2 | Whether a RegExp object has the flag letter `flag`. |
| 294 | `RegExpEscapedSource` | leaf | 1 | EscapeRegExpPattern of [[OriginalSource]] (`(?:)` when empty). |
| 295 | `RegExpExecRaw` | leaf | 4 | RegExpBuiltinExec ( R, S ) once lastIndex is Get(R, "lastIndex") as a Number (ToLength is applied here): the match array or null, and lastIndex updated for g and y. Pass undefined as state: when the fuel runs out mid-match the interpreter suspends here and keeps the matcher's state in that argument. InternalError "out of memory in regexp execution" if the backtracking stack overflows. |
| 296 | `RegExpTestRaw` | leaf | 4 | RegExpExecRaw without the match array: whether it matched (lastIndex is updated the same way). Suspendable like RegExpExecRaw. |
| 297 | `RegExpExecOwn` | leaf | 4 | RegExpExec ( R, S ) when its lookups read only data: R is a RegExp without an own `exec`, whose prototype is %RegExp.prototype% with the original `exec`, and whose own lastIndex is a Number. Then the match array or null (with `test`, true or false), lastIndex updated; else undefined, with nothing done. Suspendable like RegExpExecRaw (pass undefined as state). |
| 298 | `IsOriginalRegExpExec` | leaf | 1 | Whether v is the original RegExp.prototype.exec (the installed builtin). |
| 299 | `CreateRegExpStringIterator` | leaf | 4 | A RegExp String Iterator (CreateRegExpStringIterator, 22.2.9.1). |
| 300 | `RegExpStringIteratorSlot` | leaf | 2 | Slot i of a RegExp String Iterator: 0 the regexp, 1 the string, 2 global, 3 unicode, 4 done. TypeError "RegExp String Iterator object expected" for anything else. |
| 301 | `RegExpStringIteratorFinish` | leaf | 1 | Marks a RegExp String Iterator done. |
| 302 | `RegExpEscapeString` | leaf | 1 | RegExp.escape of a string (EncodeForRegExpEscape, qjs's lowercase hex). |
| 303 | `IsPristineRegExp` | leaf | 1 | Whether v is a RegExp whose protocol lookups (exec, flags and the flag getters, constructor, Symbol.species, Symbol.match, lastIndex) are all original data, so that skipping them is unobservable. |
| 304 | `RegExpSplitRaw` | leaf | 4 | RegExp.prototype[Symbol.split] of a pristine RegExp on a non-empty string with 0 < lim: the result array. Suspendable like RegExpExecRaw (pass undefined as state). |
| 305 | `RegExpReplaceRaw` | leaf | 4 | RegExp.prototype[Symbol.replace] of a pristine global RegExp with a string replacement template: the result, and lastIndex 0. Suspendable like RegExpExecRaw. |
| 306 | `IsDenseArray` | leaf | 1 | Whether v is an Array whose elements 0 .. length-1 are all present in its dense store (no holes): reading them with Get then runs no user code. |
| 307 | `AllNumbers` | leaf | 1 | Whether every element of a dense array (no holes) is a Number. |
| 308 | `AllCodePoints` | leaf | 1 | Whether every element of a dense array is an integral Number in [0, 0x10FFFF]. |
| 309 | `InstallMethod` | leaf | 4 | Defines f as a method of the object at the dotted path `target` (from the global object): writable, configurable, not enumerable, with `name` = key and `length`. The key is a string or a symbol (`[Symbol.iterator]` paths). |
| 310 | `InstallConstructor` | leaf | 5 | `@constructor`: defines f as the global constructor `key` (writable, configurable, not enumerable) and sets its `name` and `length`; with a parent (a dotted path), f's [[Prototype]] is the parent constructor. Its statics follow, then InstallPrototype. |
| 311 | `InstallPrototype` | leaf | 3 | The `prototype` of the constructor `key`, installed after its statics (qjs's order): the realm's intrinsic prototype for that name if there is one, else a new object inheriting from the parent's prototype (or Object.prototype). Sets `f.prototype`. |
| 312 | `InstallPrototypeConstructor` | leaf | 1 | `prototype.constructor` of the constructor `key`, installed among the prototype's methods where qjs's order puts it. |
| 313 | `InstallAccessor` | leaf | 4 | `@getter`/`@setter`: one half of an accessor (configurable, not enumerable). |
| 314 | `InstallValue` | leaf | 4 | `@value`: a data property with `attrs` (writable 1, enumerable 2, configurable 4). |
| 315 | `InstallIntrinsic` | leaf | 2 | Registers f so that privileged code can call it as %name(...). |
| 316 | `MarkNative` | leaf | 2 | Makes the builtin f native builtin `index` of the generated table: with native builtins on, its calls run the Rust state machine. |
| 317 | `MarkFast` | leaf | 1 | Gives the builtin f its leaf fast path (`@fast`): with native builtins on, a call first tries the Rust function of the same name in builtins/fast.rs. |
| 318 | `ArraySpeciesCreate` | builtin (JS) | 2 |  |
| 319 | `ArrayConstructor` | builtin (JS) | 0 + rest |  |
| 320 | `ArraySpecies` | builtin (JS) | 0 |  |
| 321 | `FindViaPredicate` | builtin (JS) | 6 |  |
| 322 | `EveryOrSome` | builtin (JS) | 4 |  |
| 323 | `SortList` | builtin (JS) | 2 |  |
| 324 | `CollectPresent` | builtin (JS) | 3 |  |
| 325 | `ArraySetLength` | builtin (JS) | 3 |  |
| 326 | `AsyncGeneratorStep` | builtin (JS) | 3 |  |
| 327 | `AsyncGeneratorAwaitReturnFulfilled` | builtin (JS) | 1 |  |
| 328 | `AsyncGeneratorAwaitReturnRejected` | builtin (JS) | 1 |  |
| 329 | `GetAsyncIterator` | builtin (JS) | 1 |  |
| 330 | `AsyncIteratorReturnMethod` | builtin (JS) | 1 |  |
| 331 | `AsyncIteratorCloseThrow` | builtin (JS) | 1 |  |
| 332 | `AsyncFromSyncIteratorUnwrap` | builtin (JS) | 1 |  |
| 333 | `AsyncFromSyncIteratorCloseThrow` | builtin (JS) | 1 |  |
| 334 | `ToBigInt` | builtin (JS) | 1 |  |
| 335 | `ToPrimitive` | builtin (JS) | 2 |  |
| 336 | `ToNumber` | builtin (JS) | 1 |  |
| 337 | `ToString` | builtin (JS) | 1 |  |
| 338 | `LengthOfArrayLike` | builtin (JS) | 1 |  |
| 339 | `ToPropertyKey` | builtin (JS) | 1 |  |
| 340 | `GetPrototypeFromConstructorSlow` | builtin (JS) | 2 |  |
| 341 | `ToNumeric` | builtin (JS) | 1 |  |
| 342 | `DatePrototypeValueOf` | builtin (JS) | 0 |  |
| 343 | `DatePrototypeToISOString` | builtin (JS) | 0 |  |
| 344 | `DatePrototypeToJSON` | builtin (JS) | 1 |  |
| 345 | `DatePrototypeToPrimitive` | builtin (JS) | 1 |  |
| 346 | `DecoratorStart` | builtin (JS) | 2 |  |
| 347 | `ApplyDecorators` | builtin (JS) | 6 |  |
| 348 | `DecoratorRunInits` | builtin (JS) | 3 |  |
| 349 | `DecoratorRunExtras` | builtin (JS) | 3 |  |
| 350 | `DecoratorAddInitializer` | builtin (JS) | 1 |  |
| 351 | `DecoratorAccessGet` | builtin (JS) | 1 |  |
| 352 | `DecoratorAccessSet` | builtin (JS) | 2 |  |
| 353 | `DecoratorAccessHas` | builtin (JS) | 1 |  |
| 354 | `ApplyLegacyDecorators` | builtin (JS) | 4 |  |
| 355 | `DecoratorParam` | builtin (JS) | 2 |  |
| 356 | `DecoratorParamCall` | builtin (JS) | 2 |  |
| 357 | `DecoratorMetadata` | builtin (JS) | 2 |  |
| 358 | `AsyncDisposeFulfilled` | builtin (JS) | 1 |  |
| 359 | `AsyncDisposeRejected` | builtin (JS) | 1 |  |
| 360 | `UsingCreate` | builtin (JS) | 0 |  |
| 361 | `UsingAdd` | builtin (JS) | 2 |  |
| 362 | `AwaitUsingAdd` | builtin (JS) | 2 |  |
| 363 | `UsingDisposeError` | builtin (JS) | 2 |  |
| 364 | `UsingDispose` | builtin (JS) | 1 |  |
| 365 | `UsingDisposeStep` | builtin (JS) | 1 |  |
| 366 | `UsingAwaitValue` | builtin (JS) | 1 |  |
| 367 | `CallPrepareStackTrace` | builtin (JS) | 4 |  |
| 368 | `ThrowWithStack` | builtin (JS) | 1 |  |
| 369 | `MicrotaskJob` | builtin (JS) | 1 |  |
| 370 | `FromAsyncFulfilled` | builtin (JS) | 1 |  |
| 371 | `FromAsyncRejected` | builtin (JS) | 1 |  |
| 372 | `InstanceofOperator` | builtin (JS) | 2 |  |
| 373 | `ClassHeritage` | builtin (JS) | 1 |  |
| 374 | `ThrowTypeError` | builtin (JS) | 1 |  |
| 375 | `FunctionPrototypeBehavior` | builtin (JS) | 0 |  |
| 376 | `AsyncAwait` | builtin (JS) | 2 |  |
| 377 | `AsyncFunctionReturn` | builtin (JS) | 2 |  |
| 378 | `GeneratorFunctionConstructor` | builtin (JS) | 0 + rest |  |
| 379 | `AsyncGeneratorFunctionConstructor` | builtin (JS) | 0 + rest |  |
| 380 | `AsyncFunctionConstructor` | builtin (JS) | 0 + rest |  |
| 381 | `DynamicImport` | builtin (JS) | 2 |  |
| 382 | `GetIterator` | builtin (JS) | 1 |  |
| 383 | `IteratorClose` | builtin (JS) | 1 |  |
| 384 | `IteratorCloseThrow` | builtin (JS) | 1 |  |
| 385 | `IteratorRest` | builtin (JS) | 2 |  |
| 386 | `ArrayPrototypeValues` | builtin (JS) | 0 |  |
| 387 | `ArrayIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 388 | `StringPrototypeIterator` | builtin (JS) | 0 |  |
| 389 | `StringIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 390 | `SerializeJSONProperty` | builtin (JS) | 7 |  |
| 391 | `JSONStackIncludes` | builtin (JS) | 2 |  |
| 392 | `JSONJoin` | builtin (JS) | 7 |  |
| 393 | `SerializeJSONObject` | builtin (JS) | 6 |  |
| 394 | `SerializeJSONArray` | builtin (JS) | 6 |  |
| 395 | `AddEntriesFromIterable` | builtin (JS) | 3 |  |
| 396 | `MapPrototypeSet` | builtin (JS) | 2 |  |
| 397 | `MapIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 398 | `ResolvePromise` | builtin (JS) | 2 |  |
| 399 | `PromiseResolveFunction` | builtin (JS) | 1 |  |
| 400 | `PromiseRejectFunction` | builtin (JS) | 1 |  |
| 401 | `GetCapabilitiesExecutor` | builtin (JS) | 2 |  |
| 402 | `PromiseReactionJob` | builtin (JS) | 4 |  |
| 403 | `PromiseResolveThenableJob` | builtin (JS) | 3 |  |
| 404 | `PromiseConstructor` | builtin (JS) | 1 |  |
| 405 | `PromiseSpecies` | builtin (JS) | 0 |  |
| 406 | `PromiseStaticResolve` | builtin (JS) | 1 |  |
| 407 | `PromiseAllResolveElement` | builtin (JS) | 1 |  |
| 408 | `PromiseAllSettledResolveElement` | builtin (JS) | 1 |  |
| 409 | `PromiseAllSettledRejectElement` | builtin (JS) | 1 |  |
| 410 | `PromiseAnyRejectElement` | builtin (JS) | 1 |  |
| 411 | `PromisePrototypeThen` | builtin (JS) | 2 |  |
| 412 | `PromiseThenFinally` | builtin (JS) | 1 |  |
| 413 | `PromiseCatchFinally` | builtin (JS) | 1 |  |
| 414 | `PromiseValueThunk` | builtin (JS) | 0 |  |
| 415 | `PromiseThrower` | builtin (JS) | 0 |  |
| 416 | `HostCallSettle` | builtin (JS) | 3 |  |
| 417 | `ProxyRevokeFunction` | builtin (JS) | 0 |  |
| 418 | `ProxyGetPrototypeOf` | builtin (JS) | 1 |  |
| 419 | `ProxySetPrototypeOf` | builtin (JS) | 2 |  |
| 420 | `ProxyIsExtensible` | builtin (JS) | 1 |  |
| 421 | `ProxyPreventExtensions` | builtin (JS) | 1 |  |
| 422 | `ProxyGetOwnPropertyDescriptorObject` | builtin (JS) | 2 |  |
| 423 | `ProxyHasOwnProperty` | builtin (JS) | 2 |  |
| 424 | `ProxyOwnPropertyAttributes` | builtin (JS) | 2 |  |
| 425 | `ProxyDefineOwnPropertyFromRecord` | builtin (JS) | 3 |  |
| 426 | `ProxyDefineDataPropertyOrThrow` | builtin (JS) | 4 |  |
| 427 | `ProxyCreateDataProperty` | builtin (JS) | 3 |  |
| 428 | `ProxyCreateDataPropertyOrThrow` | builtin (JS) | 3 |  |
| 429 | `ProxyHas` | builtin (JS) | 2 |  |
| 430 | `ProxyGet` | builtin (JS) | 3 |  |
| 431 | `ProxySet` | builtin (JS) | 4 |  |
| 432 | `ProxySetOrThrow` | builtin (JS) | 4 |  |
| 433 | `ProxySetOnReceiver` | builtin (JS) | 3 |  |
| 434 | `ProxyDeleteProperty` | builtin (JS) | 2 |  |
| 435 | `ProxyDeletePropertyOrThrow` | builtin (JS) | 2 |  |
| 436 | `ProxyOwnPropertyKeys` | builtin (JS) | 2 |  |
| 437 | `ProxyCall` | builtin (JS) | 3 |  |
| 438 | `ProxyConstruct` | builtin (JS) | 3 |  |
| 439 | `OrdinaryConstructSlow` | builtin (JS) | 3 |  |
| 440 | `ProxySetIntegrityLevel` | builtin (JS) | 2 |  |
| 441 | `ProxyTestIntegrityLevel` | builtin (JS) | 2 |  |
| 442 | `ForInStep` | builtin (JS) | 1 |  |
| 443 | `RegExpSpecies` | builtin (JS) | 0 |  |
| 444 | `RegExpPrototypeHasIndices` | builtin (JS) | 0 |  |
| 445 | `RegExpPrototypeGlobal` | builtin (JS) | 0 |  |
| 446 | `RegExpPrototypeIgnoreCase` | builtin (JS) | 0 |  |
| 447 | `RegExpPrototypeMultiline` | builtin (JS) | 0 |  |
| 448 | `RegExpPrototypeDotAll` | builtin (JS) | 0 |  |
| 449 | `RegExpPrototypeUnicode` | builtin (JS) | 0 |  |
| 450 | `RegExpPrototypeUnicodeSets` | builtin (JS) | 0 |  |
| 451 | `RegExpPrototypeSticky` | builtin (JS) | 0 |  |
| 452 | `RegExpPrototypeFlags` | builtin (JS) | 0 |  |
| 453 | `RegExpPrototypeExec` | builtin (JS) | 1 |  |
| 454 | `RegExpPrototypeMatch` | builtin (JS) | 1 |  |
| 455 | `SetPrototypeAdd` | builtin (JS) | 1 |  |
| 456 | `SetIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 457 | `ArraySpread` | builtin (JS) | 2 |  |
| 458 | `CopyDataProperties` | builtin (JS) | 2 |  |
| 459 | `CopyDataPropertiesExcluding` | builtin (JS) | 2 |  |
| 460 | `TypedArraySetElementSlow` | builtin (JS) | 3 |  |
| 461 | `FinalizationRegistryCleanupJob` | builtin (JS) | 1 |  |
<!-- /gen -->
