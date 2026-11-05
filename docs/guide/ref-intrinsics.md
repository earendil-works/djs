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
| 41 | `ErrorCreate` | leaf | 1 | An object with [[ErrorData]] and prototype `proto`. |
| 42 | `DOMExceptionCreate` | leaf | 3 | A DOMException: an object with [[ErrorData]], its name and message in slots. |
| 43 | `DOMExceptionField` | leaf | 2 | Slot i (0 name, 1 message) of a DOMException; undefined if v is not one. |
| 44 | `DOMExceptionCode` | leaf | 1 | The legacy code of a DOMException name, 0 if it has none. |
| 45 | `DOMExceptionConstant` | leaf | 1 | The name of the legacy code constant i, 1 to 25 (`INDEX_SIZE_ERR`, ...). |
| 46 | `Btoa` | leaf | 1 | btoa's base64 of the code units of s; undefined if one is above 0xFF. |
| 47 | `Atob` | leaf | 1 | atob's forgiving-base64 decoding of s into a string of bytes; undefined if invalid. |
| 48 | `CaptureStack` | leaf | 1 | Records `o.stack` from the current frames, without the prelude frames at the top: an Error constructor does not show itself. With Error.prepareStackTrace it is pending until FinishErrorStack. |
| 49 | `ErrorStack` | leaf | 1 | The stack an Error object recorded, a pending one too, or undefined for any other value. |
| 50 | `StackTraceValue` | leaf | 1 | The stack trace of the current frames (the calling builtin left out): a string, or a pending stack with Error.prepareStackTrace. With a function `below`, only the frames below its topmost call, none if it is not on the stack (Error.captureStackTrace). |
| 51 | `IsPendingStack` | leaf | 1 | Whether v is a stack waiting for Error.prepareStackTrace. |
| 52 | `SetErrorStack` | leaf | 2 | Sets the stack slot of an Error (an own `stack` of anything else). |
| 53 | `CallSiteSlot` | leaf | 2 | Slot i of a CallSite (0 function, 1 name, 2 file, 3 line, 4 column, 5 native, 6 constructor); TypeError "CallSite object expected" for anything else. |
| 54 | `DefineDataPropertyOrThrow` | reentrant | 4 | DefinePropertyOrThrow ( O, P, desc ) with a complete data descriptor; `attrs` are writable 1, enumerable 2, configurable 4. |
| 55 | `DefineOwnPropertyFromRecord` | reentrant | 3 | [[DefineOwnProperty]] with a descriptor record from ToPropertyDescriptor (an object whose own data properties are the present fields). False if it was rejected. |
| 56 | `GetOwnPropertyDescriptorObject` | reentrant | 2 | FromPropertyDescriptor ( [[GetOwnProperty]](o, key) ): undefined if absent. |
| 57 | `HasOwnProperty` | reentrant | 2 | HasOwnProperty ( O, P ). |
| 58 | `OwnPropertyAttributes` | reentrant | 2 | The attributes of an own property: -1 if absent, else writable 1, enumerable 2, configurable 4, accessor 8. |
| 59 | `OwnPropertyKeys` | reentrant | 2 | The own keys of o as a new array, in [[OwnPropertyKeys]] order: kind 0 strings, 1 symbols, 2 both, 3 enumerable strings. |
| 60 | `GetPrototypeOf` | reentrant | 1 | [[GetPrototypeOf]] ( ): an object or null. |
| 61 | `SetPrototypeOf` | reentrant | 2 | [[SetPrototypeOf]] ( V ) with an object or null: false if refused. |
| 62 | `PreventExtensions` | reentrant | 1 | [[PreventExtensions]] ( ). |
| 63 | `IsExtensible` | reentrant | 1 | [[IsExtensible]] ( ). |
| 64 | `SetIntegrityLevel` | reentrant | 2 | SetIntegrityLevel ( O, level ): sealed, or frozen if `frozen`. |
| 65 | `TestIntegrityLevel` | reentrant | 2 | TestIntegrityLevel ( O, level ): sealed, or frozen if `frozen`. |
| 66 | `SameValue` | leaf | 2 | SameValue ( x, y ). |
| 67 | `ObjectClassTag` | leaf | 1 | The builtinTag of Object.prototype.toString: Array, Arguments, Function, Error, Boolean, Number, String, Date, RegExp, else Object. |
| 68 | `WrapPrimitive` | leaf | 2 | A Boolean, Number, String or Symbol wrapper of the primitive v with prototype proto. |
| 69 | `PrimitiveValue` | leaf | 1 | The primitive of a wrapper object ([[BooleanData]] and so on), a primitive itself, or undefined for other objects. |
| 70 | `ConstructWithArray` | reentrant | 3 | Construct ( F, args, newTarget ) with the arguments in a dense array. |
| 71 | `ConstructWithThis` | reentrant | 4 | [[Construct]] of an ordinary (non-builtin) function f with `thisArg` as the new object (OrdinaryCreateFromConstructor done by the caller) and new.target. |
| 72 | `GetWithReceiver` | reentrant | 3 | [[Get]] ( P, Receiver ): a getter runs with `receiver` as this. |
| 73 | `SetWithReceiver` | reentrant | 4 | OrdinarySet ( O, P, V, Receiver ): false when refused; a setter runs with receiver. |
| 74 | `DeleteProperty` | reentrant | 2 | [[Delete]] ( P ): false for a non-configurable property. |
| 75 | `IsProxy` | leaf | 1 | Whether v is a Proxy exotic object (revoked or not). |
| 76 | `NewProxy` | leaf | 2 | A new proxy (ProxyCreate's object, 10.5.14): callable/constructor as target is. |
| 77 | `ProxyTarget` | leaf | 1 | [[ProxyTarget]] of a proxy: TypeError "revoked proxy" once revoked. |
| 78 | `ProxyHandler` | leaf | 1 | [[ProxyHandler]] of a proxy: TypeError "revoked proxy" once revoked. |
| 79 | `ProxyRevoke` | leaf | 1 | Revokes a proxy (target and handler become null). |
| 80 | `CollectionCreate` | leaf | 2 | A new empty collection: class 0 Map, 1 Set, 2 WeakMap, 3 WeakSet. |
| 81 | `CollectionClass` | leaf | 1 | The class of a collection object (0 Map, 1 Set, 2 WeakMap, 3 WeakSet), -1 otherwise. |
| 82 | `CollectionGet` | leaf | 2 | The value of key (SameValueZero), or undefined. |
| 83 | `CollectionHas` | leaf | 2 | Whether key is present. |
| 84 | `CollectionSet` | leaf | 3 | Adds or replaces key (-0 becomes +0); a Set passes true as the value. |
| 85 | `CollectionDelete` | leaf | 2 | Removes key; false if it was absent. |
| 86 | `CollectionClear` | leaf | 1 | Removes every entry. |
| 87 | `CollectionSize` | leaf | 1 | The number of entries. |
| 88 | `CollectionIterator` | leaf | 2 | A Map Iterator or Set Iterator over m (kind 0 keys, 1 values, 2 entries): it visits entries added while iterating and skips deleted ones. |
| 89 | `CollectionIteratorNext` | leaf | 2 | %MapIteratorPrototype%.next / %SetIteratorPrototype%.next: the iterator result. TypeError "Map Iterator object expected" (or Set) for anything else. |
| 90 | `IsOriginalCollectionAdder` | leaf | 2 | Whether `adder` is the installed Map.prototype.set (kind 0) or Set.prototype.add (1): the constructors then store directly. |
| 91 | `CollectionIteratorStep` | leaf | 1 | Advances an iterator to the next entry; false when done. |
| 92 | `CollectionIteratorKey` | leaf | 1 | The key of the entry the last step reached. |
| 93 | `CollectionIteratorValue` | leaf | 1 | The value of the entry the last step reached. |
| 94 | `CanBeHeldWeakly` | leaf | 1 | CanBeHeldWeakly ( v ): an object, or a symbol not in the Symbol.for registry. |
| 95 | `DateNow` | leaf | 0 | The host's clock (ms since the epoch; Vm::set_now). |
| 96 | `DateCreate` | leaf | 2 | A new Date object with prototype proto and time value t. |
| 97 | `DateValue` | leaf | 1 | thisTimeValue: the time value of a Date, TypeError "not a Date object" otherwise. |
| 98 | `DateSetValue` | leaf | 2 | Stores time value t in a Date (TypeError "not a Date object" otherwise); returns t. |
| 99 | `DateFromFields` | leaf | 2 | A time value from the Numbers year, month[, date, hours, minutes, seconds, ms] in a dense array (the constructor's and Date.UTC's form: years 0-99 are 1900-1999). |
| 100 | `DateGetField` | leaf | 3 | Field `field` of t (0 year, 1 month, 2 date, 3 hours, 4 minutes, 5 seconds, 6 ms, 7 weekday), local or UTC; NaN for an invalid date. |
| 101 | `DateSetFields` | leaf | 4 | A setter's new time value: fields first.. of t replaced by the Numbers in args (a dense array), local or UTC (qjs's set_date_field). |
| 102 | `DateFormat` | leaf | 3 | Formats t: fmt 0 toUTCString, 1 toString, 2 toISOString (RangeError for NaN), 3 toLocale*; part 1 date, 2 time, 3 both. "Invalid Date" for NaN. |
| 103 | `DateParse` | leaf | 1 | Date.parse of a string (ISO and the formats qjs's toString/toUTCString emit). |
| 104 | `DateTimezoneOffset` | leaf | 1 | getTimezoneOffset of t: minutes (UTC - local), NaN for an invalid date. |
| 105 | `ArrayBufferCreate` | leaf | 3 | AllocateArrayBuffer: byteLength zeroed bytes; max is the maxByteLength of a resizable buffer, true for an immutable one, or undefined. RangeError "invalid array buffer length" (or "... max length"). |
| 106 | `BufferInfo` | leaf | 2 | A field of an ArrayBuffer: 0 byteLength, 1 maxByteLength, 2 resizable, 3 detached, 4 immutable. TypeError "ArrayBuffer object expected" for anything else. |
| 107 | `BufferDetach` | leaf | 1 | DetachArrayBuffer. |
| 108 | `BufferResize` | leaf | 2 | Resizes a resizable buffer in place (new bytes are 0). |
| 109 | `BufferTransfer` | leaf | 3 | A new buffer with the first bytes of b (max: a Number for resizable, true for immutable), b detached; an immutable b is a TypeError. |
| 110 | `BufferCopy` | leaf | 5 | Copies count bytes from src at from to dst at to (memmove). |
| 111 | `SortNumbers` | leaf | 1 | Sorts a dense array of Numbers numerically in place (a typed array's default order: -0 before +0, NaN last; stable), or one of BigInts by value. |
| 112 | `DataViewCreate` | leaf | 4 | A DataView over buffer (byteLength -1: length-tracking). |
| 113 | `DataViewField` | leaf | 2 | A slot of a DataView: 0 buffer, 1 byteOffset, 2 byteLength (-1: length-tracking). TypeError "DataView object expected" for anything else. |
| 114 | `DataViewGet` | leaf | 4 | The element of kind at byte `at` of the view's buffer (bounds checked by the caller): a Number, or a BigInt for kinds 10 and 11. |
| 115 | `DataViewSet` | leaf | 5 | Writes v (a Number, or a BigInt for kinds 10 and 11) as kind at byte `at` of the view's buffer. |
| 116 | `IsDataView` | leaf | 1 | Whether v is a DataView. |
| 117 | `RealmConstructor` | leaf | 1 | The constructor installed as `name` (the realm's %Name%, whatever the global is now). |
| 118 | `IsTypedArray` | leaf | 1 | Whether v is a typed array. |
| 119 | `TypedArrayCreate` | leaf | 5 | A typed array of `kind` over buffer (length -1: length-tracking). |
| 120 | `TypedArrayField` | leaf | 2 | A field of a typed array: 0 kind, 1 buffer, 2 byteOffset, 3 length, 4 byteLength (2-4 are 0 when out of bounds), 5 out of bounds or detached, 6 length-tracking, 7 [[ByteOffset]] (also when out of bounds). TypeError "not a TypedArray" for anything else. |
| 121 | `TypedArrayGet` | leaf | 2 | Element i (undefined if not a valid index). |
| 122 | `TypedArraySet` | leaf | 3 | Writes v (a Number, or a BigInt for a BigInt array) to element i (nothing if not a valid index). |
| 123 | `TypedArrayCopy` | leaf | 5 | Copies count elements from src (at from) to dst (at to), converting; overlaps ok. |
| 124 | `TypedArrayKindProto` | leaf | 1 | The prototype of the typed array constructor for kind (the realm's). |
| 125 | `Base64Encode` | leaf | 3 | The bytes of a Uint8Array as base64 (TypeError if detached or out of bounds). |
| 126 | `HexEncode` | leaf | 1 | The bytes of a Uint8Array as lowercase hex (TypeError if detached). |
| 127 | `Base64Decode` | leaf | 4 | FromBase64 ( string, alphabet, lastChunkHandling, maxLength ): a record object { read, bytes (an ArrayBuffer), error } (last: 0 loose, 1 strict, 2 stop-before-partial). |
| 128 | `HexDecode` | leaf | 2 | FromHex ( string, maxLength ): the same record. |
| 129 | `CallWithArray` | reentrant | 3 | Call ( F, V, args ) with the arguments in a dense array. |
| 130 | `BoundFunctionCreate` | leaf | 3 | BoundFunctionCreate ( target, boundThis, boundArgs ), without name and length. |
| 131 | `BoundTargetFunction` | leaf | 1 | [[BoundTargetFunction]] of a bound function, or undefined for anything else. |
| 132 | `FunctionToString` | leaf | 1 | The source text of a function (natives: `function name() { [native code] }`). |
| 133 | `NumberPow` | leaf | 2 | Number::exponentiate ( base, exponent ). |
| 134 | `SymbolDescriptiveString` | leaf | 1 | SymbolDescriptiveString ( sym ): `Symbol(description)`. |
| 135 | `CompareStrings` | leaf | 2 | Compares two strings by code units (IsLessThan for strings): -1, 0 or 1. Also localeCompare's order (no Intl). |
| 136 | `StringCodePoints` | leaf | 1 | The code points of s as an array of strings (the string iterator's values). |
| 137 | `SortPrimitives` | leaf | 1 | Sorts the dense array `list` in place by SortCompare without a comparefn (undefined last, then ToString order; stable) if every element is a primitive other than a symbol. Returns false, leaving it unchanged, otherwise. |
| 138 | `ArraySpliceDense` | leaf | 4 | Splice's element moves in one step on a plain dense Array (no holes, default attributes, extensible, writable length, no index properties on the prototype chain): o[start .. start + deleteCount) becomes itemCount undefined slots and `length` follows. False, with nothing changed, for any other object. |
| 139 | `ArrayCanPopDense` | leaf | 1 | Whether `ArrayPopDense(o)` applies: an Array with only dense elements, a writable length, its last element present and no index properties on its prototype chain, where pop's Get, DeletePropertyOrThrow and Set observe nothing else. |
| 140 | `ArrayPopDense` | leaf | 1 | Array.prototype.pop of an array `ArrayCanPopDense` accepts. |
| 141 | `ArrayFillDense` | leaf | 4 | fill's Sets on an Array while each is a plain store (an existing dense element, or an append or hole fill of an extensible Array without index properties on its prototype chain), from k up to fin: the index the general loop continues from. |
| 142 | `ArrayPushDense` | leaf | 2 | push(v)'s Set and length update in one step when the Set is a plain append to the Array `o` (extensible, writable length, no holes at its end, no index properties on its prototype chain). The new length, or -1 with nothing changed. |
| 143 | `ArraySearchDense` | leaf | 5 | The search loops of indexOf (mode 0: `from` up to len, strict equality), lastIndexOf (1: `from` down to 0) and includes (2: up to len, SameValueZero) over an Array whose elements in the range are all present in dense storage, where HasProperty and Get observe nothing: the index found or -1. -2, for the general loop, for anything else (a hole, a shorter array, another object). |
| 144 | `ArraySliceDense` | leaf | 4 | slice's copy loop: `a[0 .. fin - k) = o[k .. fin)` when every element of `o` in the range is present in dense storage and `a` is an extensible Array without index properties (a fresh ArraySpeciesCreate result), so HasProperty, Get and CreateDataPropertyOrThrow observe nothing. False, with nothing changed, otherwise. |
| 145 | `EachDense` | reentrant | 7 | The dense part of the loops of the callback builtins. From `k`, while `o` is an Array whose element k is present in its dense store (so HasProperty and Get observe nothing), calls `f(o[k], k, o)` with `thisArg` and completes the step by `mode`: - 0 forEach; - 1 map: CreateDataPropertyOrThrow(a, k, result); - 2 filter: for a truthy result, CreateDataPropertyOrThrow(a, to, o[k]), with `to` the length of `a`, a fresh Array (the receiver's species is plain); - 3 some, 4 every: a truthy (some) or falsy (every) result decides; - 5 find: a truthy result decides, and o[k] goes to element 0 of the Array `a`; - 6 findIndex: a truthy result decides; - 7 reduce: calls `f(acc, o[k], k, o)` with `this` undefined; the accumulator starts as `thisArg`, and each result goes to element 0 of the Array `a` (so `a` holds the accumulator once the returned k is past the starting one); - 8 Array.from of an array-like: calls `f(o[k], k)` and does CreateDataPropertyOrThrow(a, k, result); `o` is any ordinary object (or array or function) whose element k, along its prototype chain, is a data property or absent (undefined), so Get observes nothing. One frame for `f`, entered again per element. Returns the k the builtin's own loop goes on from (`len`: done; `k` itself when `f` is not a plain closure or `a` is not as required), or -1 - k when the result at k decided. |
| 146 | `ArrayJoinDense` | leaf | 3 | join over o[0 .. len) when every element is present in dense storage and a primitive other than a symbol (ToString runs no user code): the string, or undefined for the general loop. |
| 147 | `EnumerableOwnDataProperties` | leaf | 2 | EnumerableOwnProperties ( o, kind ) for kind 1 (values) and 2 (entries) in one step when no enumerable string-keyed own property is an accessor and `o` is not exotic (no Proxy): the array, or undefined for the general loop. |
| 148 | `TypedArraySetFromDense` | leaf | 2 | Writes values[0 .. length) into the typed array `o` (at least that long) when `values` is an Array whose elements are all present in dense storage and primitives of `o`'s content type (ToNumber / ToBigInt run no user code). False, with nothing written, otherwise. |
| 149 | `StrLocaleCompare` | leaf | 2 | localeCompare without Intl (as qjs): the difference of the first differing code points, else -1, 0 or 1 by length. |
| 150 | `StrCharCodeAt` | leaf | 2 | The code unit at i as a Number, NaN out of range. |
| 151 | `StrCodePointAt` | leaf | 2 | The code point at i (a surrogate pair combined), undefined out of range. |
| 152 | `StrSubstring` | leaf | 3 | s[start .. end) in code units; the indexes are integers, clamped by the caller. |
| 153 | `StrIndexOf` | leaf | 3 | The first index ≥ from where search occurs, or -1. |
| 154 | `StrLastIndexOf` | leaf | 3 | The last index ≤ from where search occurs, or -1. |
| 155 | `StrToCase` | leaf | 2 | Full Unicode upper- or lowercase (SpecialCasing, Final_Sigma; no locales). |
| 156 | `StrTrim` | leaf | 2 | Trims WhiteSpace and LineTerminators: where 0 both ends, 1 start, 2 end. |
| 157 | `StrIsWellFormed` | leaf | 1 | IsStringWellFormedUnicode. |
| 158 | `StrToWellFormed` | leaf | 1 | Lone surrogates replaced by U+FFFD. |
| 159 | `StrRepeat` | leaf | 2 | s repeated n times (an integer ≥ 0; RangeError past the string length limit). |
| 160 | `StrFromCodeUnits` | leaf | 1 | A string of the code units ToUint16(n) of the Numbers in a dense array. |
| 161 | `StrFromCodePoints` | leaf | 1 | A string of the (valid) code points in a dense array. |
| 162 | `StrNormalize` | leaf | 2 | String.prototype.normalize for form 0 NFC, 1 NFD, 2 NFKC, 3 NFKD: the normalized string; undefined only when djs is built without the `unicode-normalization` feature and the string would need its tables. |
| 163 | `NewSymbol` | leaf | 1 | A new symbol; the description is a string or undefined. |
| 164 | `SymbolDescription` | leaf | 1 | [[Description]] of a symbol: a string or undefined. |
| 165 | `SymbolFor` | leaf | 1 | Symbol.for's GlobalSymbolRegistry lookup or insertion. |
| 166 | `SymbolKeyFor` | leaf | 1 | Symbol.keyFor's registry lookup: the key, or undefined. |
| 167 | `NumberToString` | leaf | 2 | Number::toString ( x, radix ) for an integral radix (RangeError outside 2..36), Number.prototype.toString. |
| 168 | `NumberToFixed` | leaf | 2 | Number.prototype.toFixed after ToIntegerOrInfinity(fractionDigits). |
| 169 | `NumberToExponential` | leaf | 2 | Number.prototype.toExponential; digits is undefined or an integral Number. |
| 170 | `NumberToPrecision` | leaf | 2 | Number.prototype.toPrecision; precision is undefined or an integral Number. |
| 171 | `StringParseFloat` | leaf | 1 | The global parseFloat after ToString. |
| 172 | `StringParseInt` | leaf | 2 | The global parseInt after ToString; radix is ToInt32(radix). |
| 173 | `UriEncode` | leaf | 2 | Encode ( string, extraUnescaped ), 19.2.6.5, after ToString: encodeURI keeps the reserved characters (`component` false), encodeURIComponent escapes them. URIError for a lone surrogate. |
| 174 | `UriDecode` | leaf | 2 | Decode ( string, preserveEscapeSet ), 19.2.6.6, after ToString: decodeURI keeps the escapes of reserved characters (`component` false). URIError for a malformed escape or UTF-8 sequence. |
| 175 | `StrEscape` | leaf | 1 | escape ( string ), B.2.1.1, after ToString. |
| 176 | `StrUnescape` | leaf | 1 | unescape ( string ), B.2.1.2, after ToString. |
| 177 | `CompileEval` | leaf | 1 | PerformEval's parse (19.2.1.1) for indirect eval: the source as a Script closure, to be called with `this` = the global object. SyntaxError for invalid source. Charges fuel per source byte. |
| 178 | `CompileFunction` | leaf | 3 | CreateDynamicFunction's parse (20.2.1.1.1): `kind` 0 function, 1 generator, 2 async, 3 async generator; `params` joined with ","; a Script closure whose call returns the new function. SyntaxError for invalid source. Charges fuel per source byte. |
| 179 | `GlobalObject` | leaf | 0 | The realm's global object. |
| 180 | `EnqueueMicrotask` | leaf | 1 | Queues the job MicrotaskJob(callback) (queueMicrotask). |
| 181 | `NumberToInt32` | leaf | 1 | ToInt32 of a number. |
| 182 | `NumberToUint32` | leaf | 1 | ToUint32 of a number, as an f64. |
| 183 | `F64Abs` | leaf | 1 |  |
| 184 | `F64Acos` | leaf | 1 |  |
| 185 | `F64Acosh` | leaf | 1 |  |
| 186 | `F64Asin` | leaf | 1 |  |
| 187 | `F64Asinh` | leaf | 1 |  |
| 188 | `F64Atan` | leaf | 1 |  |
| 189 | `F64Atanh` | leaf | 1 |  |
| 190 | `F64Atan2` | leaf | 2 |  |
| 191 | `F64Cbrt` | leaf | 1 |  |
| 192 | `F64Hypot` | leaf | 2 |  |
| 193 | `F64Ceil` | leaf | 1 |  |
| 194 | `F64Clz32` | leaf | 1 | Math.clz32 of a number (ToUint32 inside). |
| 195 | `F64Cos` | leaf | 1 |  |
| 196 | `F64Cosh` | leaf | 1 |  |
| 197 | `F64Exp` | leaf | 1 |  |
| 198 | `F64Expm1` | leaf | 1 |  |
| 199 | `F64Floor` | leaf | 1 |  |
| 200 | `F64Fround` | leaf | 1 | Rounds to the nearest f32. |
| 201 | `F64F16round` | leaf | 1 | Rounds to the nearest binary16 value (Math.f16round). |
| 202 | `F64SumPrecise` | leaf | 1 | The exactly rounded sum of the Numbers of the dense Array list (Math.sumPrecise). |
| 203 | `F64Imul` | leaf | 2 | Math.imul of two numbers (ToUint32 inside). |
| 204 | `F64Log` | leaf | 1 |  |
| 205 | `F64Log1p` | leaf | 1 |  |
| 206 | `F64Log10` | leaf | 1 |  |
| 207 | `F64Log2` | leaf | 1 |  |
| 208 | `F64Round` | leaf | 1 | Math.round: half up, -0 kept. |
| 209 | `F64Sign` | leaf | 1 |  |
| 210 | `F64Sin` | leaf | 1 |  |
| 211 | `F64Sinh` | leaf | 1 |  |
| 212 | `F64Sqrt` | leaf | 1 |  |
| 213 | `F64Tan` | leaf | 1 |  |
| 214 | `F64Tanh` | leaf | 1 |  |
| 215 | `F64Trunc` | leaf | 1 |  |
| 216 | `MathRandom` | leaf | 0 | The next number in [0, 1) of the realm's generator (xorshift128+, seeded by the host). |
| 217 | `CreateArrayIterator` | leaf | 2 | A new Array Iterator over o (kind 0: keys, 1: values, 2: entries), 23.1.5.1. |
| 218 | `ArrayIteratorTarget` | leaf | 1 | The object an Array Iterator iterates, or undefined once it is exhausted. TypeError "Array Iterator object expected" if it is not an Array Iterator. |
| 219 | `ArrayIteratorIndex` | leaf | 1 | The next index of an Array Iterator. |
| 220 | `ArrayIteratorSetIndex` | leaf | 2 |  |
| 221 | `ArrayIteratorFinish` | leaf | 1 | Marks an Array Iterator exhausted (its target becomes undefined). |
| 222 | `ArrayIteratorKind` | leaf | 1 | The kind of an Array Iterator (0: keys, 1: values, 2: entries). |
| 223 | `CreateStringIterator` | leaf | 1 | A String Iterator over s (CreateStringIterator, 22.1.5.1): an Array Iterator object of kind 3 with %StringIteratorPrototype%; ArrayIteratorIndex/SetIndex/Finish apply. |
| 224 | `StringIteratorTarget` | leaf | 1 | The string a String Iterator iterates, or undefined once it is exhausted. TypeError "String Iterator object expected" otherwise. |
| 225 | `StrCodePointString` | leaf | 2 | The code point at index i of s as a string (one or two code units). |
| 226 | `HasOriginalIteration` | leaf | 1 | Whether v is an Array or a string whose iteration is the original Symbol.iterator method and `next` (data properties only): iterating it then equals reading it directly. |
| 227 | `IteratorFastReady` | leaf | 2 | Whether the next step of `iterator` with `next` gives a value without running user code: `next` is the original one of an Array or String Iterator and the value is at hand. Then `IteratorFastValue` returns it; otherwise call `next`. |
| 228 | `IteratorFastValue` | leaf | 2 | The next value of an iterator `IteratorFastReady` accepts (it advances). |
| 229 | `IteratorFastDrain` | leaf | 4 | The values `IteratorFastValue` would give one by one for a keys or values Array Iterator, stored with CreateDataPropertyOrThrow into the Array `arr` at n, n + 1, ... in one step, while they are plain dense stores: the next n. Stops before a value only `next` can read (a hole), at the end (the next step finishes the iterator) and where a store is not plain; gives n back unchanged for anything else. |
| 230 | `CreateIterResultObject` | leaf | 2 | CreateIterResultObject ( value, done ), 7.4.14. |
| 231 | `CreateArrayFromPair` | leaf | 2 | A new Array [a, b]. |
| 232 | `PromiseCreate` | leaf | 1 | A new pending Promise with prototype proto. |
| 233 | `IsPromise` | leaf | 1 | Whether v is a Promise object (has [[PromiseState]]). |
| 234 | `IntrinsicPromise` | leaf | 0 | The realm's %Promise% constructor. |
| 235 | `PromiseState` | leaf | 1 | [[PromiseState]]: 0 pending, 1 fulfilled, 2 rejected. |
| 236 | `PromiseResult` | leaf | 1 | [[PromiseResult]]. |
| 237 | `FulfillPromise` | leaf | 2 | FulfillPromise ( promise, value ), 27.2.1.4: reactions become jobs. No-op unless pending. |
| 238 | `RejectPromise` | leaf | 2 | RejectPromise ( promise, reason ), 27.2.1.7, with the rejection tracker. No-op unless pending. |
| 239 | `PromiseClaimResolve` | leaf | 1 | Sets the [[AlreadyResolved]] of p's own resolving functions (and internal capability); false if it was already set. |
| 240 | `PerformPromiseThen` | leaf | 4 | PerformPromiseThen ( promise, onFulfilled, onRejected, resultCapability ), 27.2.5.4.1, without the result: handlers that are not callable count as empty. |
| 241 | `PromiseCombinatorRun` | leaf | 5 | The elements of PerformPromiseCombinator (Promise.all kind 0, allSettled 1, any 2, race 3) that an Array values iterator gives without user code (as `IteratorFastValue` would), one after another while their steps run no user code either: `fixed` is the Record [C, promiseResolve, capability, values, remaining, resolveFn, rejectFn], C %Promise% with the original `resolve`, and the element a Promise of %Promise% or a value that fulfills one without a `then` call, whose `then`, `constructor` and %Promise%[@@species] are the original ones. Each is done as the loop body would (values slot, element Record, functions, `then`); returns the next index. |
| 242 | `EnqueuePromiseResolveThenableJob` | leaf | 3 | Queues NewPromiseResolveThenableJob ( promise, thenable, then ), 27.2.2.2. |
| 243 | `NewRecord` | leaf | 1 | A new internal Record with n slots, all undefined. |
| 244 | `RecordGet` | leaf | 2 | Slot i of a Record, or of an object that keeps its state in slots (IteratorHelper). |
| 245 | `RecordSet` | leaf | 3 |  |
| 246 | `NewBuiltinClosure` | leaf | 3 | A function (name "", `length`) that calls the builtin `kind` (the table in interp/run/intrinsics.rs: 0 PromiseResolveFunction, 1 PromiseRejectFunction, 2 GetCapabilitiesExecutor, 3 PromiseAllResolveElement, 4 PromiseAllSettledResolveElement, 5 PromiseAllSettledRejectElement, 6 PromiseAnyRejectElement, 7 PromiseThenFinally, 8 PromiseCatchFinally, 9 PromiseValueThunk, 10 PromiseThrower, 11 AsyncGeneratorAwaitReturnFulfilled, 12 AsyncGeneratorAwaitReturnRejected, 13 AsyncFromSyncIteratorUnwrap, 14 AsyncFromSyncIteratorCloseThrow, 15 ProxyRevokeFunction, 16 FromAsyncFulfilled, 17 FromAsyncRejected, 18 AsyncDisposeFulfilled, 19 AsyncDisposeRejected, 20 DecoratorAddInitializer, 21 DecoratorAccessGet, 22 DecoratorAccessSet, 23 DecoratorAccessHas, 24 DecoratorParamCall) with `this` = data and its own arguments. |
| 247 | `GeneratorState` | leaf | 1 | The state of a generator object (not an async state), or -1 for anything else. |
| 248 | `GeneratorSetState` | leaf | 2 |  |
| 249 | `GeneratorResume` | reentrant | 3 | Resumes a suspended generator or async function (state 0 or 1) in a frame of its own: mode 0 continues with value (a generator at a yield gets `value false`, an async function the value), 1 throws value at the suspension point, 2 (generators at a yield) returns value from there (`value true`). The result is what the activation hands back when it next suspends or completes. |
| 250 | `IsAsyncFunctionState` | leaf | 1 | Whether v is the state of an async function call. |
| 251 | `AsyncFunctionPromise` | leaf | 1 | The result promise of an async function call. |
| 252 | `AsyncGeneratorState` | leaf | 1 | The state of an async generator, or -1 for anything else. |
| 253 | `AsyncGeneratorEnqueue` | leaf | 2 | Appends a request to the generator's queue. |
| 254 | `AsyncGeneratorFirstRequest` | leaf | 1 | The oldest request, or undefined. |
| 255 | `AsyncGeneratorDequeue` | leaf | 1 | Removes and returns the oldest request (undefined if none). |
| 256 | `CreateAsyncFromSyncIterator` | leaf | 2 | CreateAsyncFromSyncIterator ( syncIteratorRecord ), 27.1.6.1: a Record [iterator, next] with %AsyncFromSyncIteratorPrototype%. |
| 257 | `CreateIteratorHelper` | leaf | 3 | A new IteratorHelper object of `kind` (0: Iterator.from's wrapper, with %WrapForValidIteratorPrototype%; else a helper, with %IteratorHelperPrototype%): slot 0 kind, 1 state 0, 2 iterator, 3 next, the others undefined. |
| 258 | `ThisIteratorHelper` | leaf | 2 | v if it is an iterator helper (wrap false) or Iterator.from's wrapper (wrap true), else TypeError "Iterator Helper object expected" / "Iterator Wrap object expected". |
| 259 | `WeakRefCreate` | leaf | 2 | A new WeakRef with prototype proto and target (AddToKeptObjects ( target ) too). |
| 260 | `WeakRefDeref` | leaf | 1 | WeakRefDeref of v (AddToKeptObjects of a live target): the target, or undefined once it died. TypeError "WeakRef object expected" for anything else. |
| 261 | `FinalizationRegistryCreate` | leaf | 2 | A new FinalizationRegistry with prototype proto and cleanupCallback. |
| 262 | `ThisFinalizationRegistry` | leaf | 1 | v if it is a FinalizationRegistry, else TypeError "FinalizationRegistry object expected". |
| 263 | `FinalizationRegistryRegister` | leaf | 4 | Appends a cell (token undefined for none). |
| 264 | `FinalizationRegistryUnregister` | leaf | 2 | Removes the cells registered with token; whether there were any. |
| 265 | `FinalizationRegistryNextDead` | leaf | 1 | The index of the first cell whose target died, or -1. |
| 266 | `FinalizationRegistryTake` | leaf | 2 | The held value of dead cell i, removing the cell. |
| 267 | `FinalizationRegistrySetQueued` | leaf | 2 | Marks whether a cleanup job for r is queued. |
| 268 | `DisposableStackCreate` | leaf | 2 | A new DisposableStack (isAsync false) or AsyncDisposableStack with prototype proto: slot 0 isAsync, 1 state 0 (pending), 2 a new empty Array of resources. |
| 269 | `ThisDisposableStack` | leaf | 2 | v if it is a DisposableStack (isAsync false) or an AsyncDisposableStack, else TypeError "DisposableStack object expected" / "AsyncDisposableStack object expected". |
| 270 | `JsonParse` | leaf | 2 | JSON.parse of text without a reviver; qjs's SyntaxError for invalid JSON. With withSource, an Array [value, record] where record is the JSON Parse Record tree for the reviver's source text access: [value, source, elements, entries]. |
| 271 | `JsonRawJSON` | leaf | 1 | JSON.rawJSON's work after ToString: a frozen, null-prototype RawJSON object with the own property rawJSON = text; SyntaxError "invalid rawJSON string" unless text is a JSON primitive without leading or trailing whitespace. |
| 272 | `IsRawJSON` | leaf | 1 | Whether v has [[IsRawJSON]] (a JSON.rawJSON object). |
| 273 | `JsonStringifyFast` | leaf | 2 | JSON.stringify(v, undefined, space) with gap already computed, for plain data: the string, undefined for an undefined result, or null when the value needs the full algorithm (accessors, toJSON, wrappers, exotic objects; nothing observable happened). TypeError "circular reference" and RangeError "Maximum call stack size exceeded" are final. |
| 274 | `JsonStringifyFastAt` | leaf | 4 | JsonStringifyFast for an object the full algorithm reached, after its toJSON and replacer (its own toJSON is not looked at again), nested in the objects of the algorithm's `stack` (a cycle back to one throws; they set the indentation), with an array replacer's PropertyList or undefined: the string, or null when it is not plain data. |
| 275 | `JsonQuote` | leaf | 1 | QuoteJSONString ( value ), 25.5.2.3. |
| 276 | `CreateDataProperty` | reentrant | 3 | CreateDataProperty ( O, P, V ): false if refused. |
| 277 | `IsBigInt` | leaf | 1 |  |
| 278 | `ToBigIntPrimitive` | leaf | 1 | ToBigInt of a primitive: TypeError for undefined, null, numbers and symbols; SyntaxError for a bad string. |
| 279 | `NumberToBigInt` | leaf | 1 | NumberToBigInt ( number ), 21.2.1.1.1: RangeError unless integral. |
| 280 | `BigIntToString` | leaf | 2 | The digits of a BigInt in radix 2..36. |
| 281 | `BigIntAsIntN` | leaf | 2 | BigInt.asIntN on an index and a BigInt. |
| 282 | `BigIntAsUintN` | leaf | 2 | BigInt.asUintN on an index and a BigInt. |
| 283 | `BigIntToNumber` | leaf | 1 | The Number value of a BigInt (rounded to nearest). |
| 284 | `IsRegExpObject` | leaf | 1 | Whether v is a RegExp object (has [[RegExpMatcher]]). |
| 285 | `RegExpCreate` | leaf | 3 | RegExpAlloc + RegExpInitialize after the ToStrings: a new RegExp with prototype proto (%RegExp.prototype% for undefined) and lastIndex 0. qjs's SyntaxError for a bad pattern or flags. Regex literals compile to this. The pattern may be a RegExp object instead of its source: with the same flags the new one shares its program. |
| 286 | `RegExpSource` | leaf | 1 | [[OriginalSource]] of a RegExp object. |
| 287 | `RegExpFlags` | leaf | 1 | [[OriginalFlags]] of a RegExp object, in canonical order. |
| 288 | `RegExpHasFlag` | leaf | 2 | Whether a RegExp object has the flag letter `flag`. |
| 289 | `RegExpEscapedSource` | leaf | 1 | EscapeRegExpPattern of [[OriginalSource]] (`(?:)` when empty). |
| 290 | `RegExpExecRaw` | leaf | 4 | RegExpBuiltinExec ( R, S ) once lastIndex is Get(R, "lastIndex") as a Number (ToLength is applied here): the match array or null, and lastIndex updated for g and y. Pass undefined as state: when the fuel runs out mid-match the interpreter suspends here and keeps the matcher's state in that argument. InternalError "out of memory in regexp execution" if the backtracking stack overflows. |
| 291 | `RegExpTestRaw` | leaf | 4 | RegExpExecRaw without the match array: whether it matched (lastIndex is updated the same way). Suspendable like RegExpExecRaw. |
| 292 | `RegExpExecOwn` | leaf | 4 | RegExpExec ( R, S ) when its lookups read only data: R is a RegExp without an own `exec`, whose prototype is %RegExp.prototype% with the original `exec`, and whose own lastIndex is a Number. Then the match array or null (with `test`, true or false), lastIndex updated; else undefined, with nothing done. Suspendable like RegExpExecRaw (pass undefined as state). |
| 293 | `IsOriginalRegExpExec` | leaf | 1 | Whether v is the original RegExp.prototype.exec (the installed builtin). |
| 294 | `CreateRegExpStringIterator` | leaf | 4 | A RegExp String Iterator (CreateRegExpStringIterator, 22.2.9.1). |
| 295 | `RegExpStringIteratorSlot` | leaf | 2 | Slot i of a RegExp String Iterator: 0 the regexp, 1 the string, 2 global, 3 unicode, 4 done. TypeError "RegExp String Iterator object expected" for anything else. |
| 296 | `RegExpStringIteratorFinish` | leaf | 1 | Marks a RegExp String Iterator done. |
| 297 | `RegExpEscapeString` | leaf | 1 | RegExp.escape of a string (EncodeForRegExpEscape, qjs's lowercase hex). |
| 298 | `IsPristineRegExp` | leaf | 1 | Whether v is a RegExp whose protocol lookups (exec, flags and the flag getters, constructor, Symbol.species, Symbol.match, lastIndex) are all original data, so that skipping them is unobservable. |
| 299 | `RegExpSplitRaw` | leaf | 4 | RegExp.prototype[Symbol.split] of a pristine RegExp on a non-empty string with 0 < lim: the result array. Suspendable like RegExpExecRaw (pass undefined as state). |
| 300 | `RegExpReplaceRaw` | leaf | 4 | RegExp.prototype[Symbol.replace] of a pristine global RegExp with a string replacement template: the result, and lastIndex 0. Suspendable like RegExpExecRaw. |
| 301 | `IsDenseArray` | leaf | 1 | Whether v is an Array whose elements 0 .. length-1 are all present in its dense store (no holes): reading them with Get then runs no user code. |
| 302 | `AllNumbers` | leaf | 1 | Whether every element of a dense array (no holes) is a Number. |
| 303 | `AllCodePoints` | leaf | 1 | Whether every element of a dense array is an integral Number in [0, 0x10FFFF]. |
| 304 | `InstallMethod` | leaf | 4 | Defines f as a method of the object at the dotted path `target` (from the global object): writable, configurable, not enumerable, with `name` = key and `length`. The key is a string or a symbol (`[Symbol.iterator]` paths). |
| 305 | `InstallConstructor` | leaf | 5 | `@constructor`: defines f as the global constructor `key` (writable, configurable, not enumerable) and sets its `name` and `length`; with a parent (a dotted path), f's [[Prototype]] is the parent constructor. Its statics follow, then InstallPrototype. |
| 306 | `InstallPrototype` | leaf | 3 | The `prototype` of the constructor `key`, installed after its statics (qjs's order): the realm's intrinsic prototype for that name if there is one, else a new object inheriting from the parent's prototype (or Object.prototype). Sets `f.prototype`. |
| 307 | `InstallPrototypeConstructor` | leaf | 1 | `prototype.constructor` of the constructor `key`, installed among the prototype's methods where qjs's order puts it. |
| 308 | `InstallAccessor` | leaf | 4 | `@getter`/`@setter`: one half of an accessor (configurable, not enumerable). |
| 309 | `InstallValue` | leaf | 4 | `@value`: a data property with `attrs` (writable 1, enumerable 2, configurable 4). |
| 310 | `InstallIntrinsic` | leaf | 2 | Registers f so that privileged code can call it as %name(...). |
| 311 | `MarkNative` | leaf | 2 | Makes the builtin f native builtin `index` of the generated table: with native builtins on, its calls run the Rust state machine. |
| 312 | `MarkFast` | leaf | 1 | Gives the builtin f its leaf fast path (`@fast`): with native builtins on, a call first tries the Rust function of the same name in builtins/fast.rs. |
| 313 | `ArraySpeciesCreate` | builtin (JS) | 2 |  |
| 314 | `ArrayConstructor` | builtin (JS) | 0 + rest |  |
| 315 | `ArraySpecies` | builtin (JS) | 0 |  |
| 316 | `FindViaPredicate` | builtin (JS) | 6 |  |
| 317 | `EveryOrSome` | builtin (JS) | 4 |  |
| 318 | `SortList` | builtin (JS) | 3 |  |
| 319 | `CollectPresent` | builtin (JS) | 3 |  |
| 320 | `ArraySetLength` | builtin (JS) | 3 |  |
| 321 | `AsyncGeneratorStep` | builtin (JS) | 3 |  |
| 322 | `AsyncGeneratorAwaitReturnFulfilled` | builtin (JS) | 1 |  |
| 323 | `AsyncGeneratorAwaitReturnRejected` | builtin (JS) | 1 |  |
| 324 | `GetAsyncIterator` | builtin (JS) | 1 |  |
| 325 | `AsyncIteratorReturnMethod` | builtin (JS) | 1 |  |
| 326 | `AsyncIteratorCloseThrow` | builtin (JS) | 1 |  |
| 327 | `AsyncFromSyncIteratorUnwrap` | builtin (JS) | 1 |  |
| 328 | `AsyncFromSyncIteratorCloseThrow` | builtin (JS) | 1 |  |
| 329 | `ToBigInt` | builtin (JS) | 1 |  |
| 330 | `ToPrimitive` | builtin (JS) | 2 |  |
| 331 | `ToNumber` | builtin (JS) | 1 |  |
| 332 | `ToString` | builtin (JS) | 1 |  |
| 333 | `LengthOfArrayLike` | builtin (JS) | 1 |  |
| 334 | `ToPropertyKey` | builtin (JS) | 1 |  |
| 335 | `GetPrototypeFromConstructor` | builtin (JS) | 2 |  |
| 336 | `ToNumeric` | builtin (JS) | 1 |  |
| 337 | `DatePrototypeValueOf` | builtin (JS) | 0 |  |
| 338 | `DatePrototypeToISOString` | builtin (JS) | 0 |  |
| 339 | `DatePrototypeToJSON` | builtin (JS) | 1 |  |
| 340 | `DatePrototypeToPrimitive` | builtin (JS) | 1 |  |
| 341 | `DecoratorStart` | builtin (JS) | 2 |  |
| 342 | `ApplyDecorators` | builtin (JS) | 6 |  |
| 343 | `DecoratorRunInits` | builtin (JS) | 3 |  |
| 344 | `DecoratorRunExtras` | builtin (JS) | 3 |  |
| 345 | `DecoratorAddInitializer` | builtin (JS) | 1 |  |
| 346 | `DecoratorAccessGet` | builtin (JS) | 1 |  |
| 347 | `DecoratorAccessSet` | builtin (JS) | 2 |  |
| 348 | `DecoratorAccessHas` | builtin (JS) | 1 |  |
| 349 | `ApplyLegacyDecorators` | builtin (JS) | 4 |  |
| 350 | `DecoratorParam` | builtin (JS) | 2 |  |
| 351 | `DecoratorParamCall` | builtin (JS) | 2 |  |
| 352 | `DecoratorMetadata` | builtin (JS) | 2 |  |
| 353 | `AsyncDisposeFulfilled` | builtin (JS) | 1 |  |
| 354 | `AsyncDisposeRejected` | builtin (JS) | 1 |  |
| 355 | `UsingCreate` | builtin (JS) | 0 |  |
| 356 | `UsingAdd` | builtin (JS) | 2 |  |
| 357 | `AwaitUsingAdd` | builtin (JS) | 2 |  |
| 358 | `UsingDisposeError` | builtin (JS) | 2 |  |
| 359 | `UsingDispose` | builtin (JS) | 1 |  |
| 360 | `UsingDisposeStep` | builtin (JS) | 1 |  |
| 361 | `UsingAwaitValue` | builtin (JS) | 1 |  |
| 362 | `CallPrepareStackTrace` | builtin (JS) | 4 |  |
| 363 | `ThrowWithStack` | builtin (JS) | 1 |  |
| 364 | `MicrotaskJob` | builtin (JS) | 1 |  |
| 365 | `FromAsyncFulfilled` | builtin (JS) | 1 |  |
| 366 | `FromAsyncRejected` | builtin (JS) | 1 |  |
| 367 | `InstanceofOperator` | builtin (JS) | 2 |  |
| 368 | `ClassHeritage` | builtin (JS) | 1 |  |
| 369 | `ThrowTypeError` | builtin (JS) | 1 |  |
| 370 | `FunctionPrototypeBehavior` | builtin (JS) | 0 |  |
| 371 | `AsyncAwait` | builtin (JS) | 2 |  |
| 372 | `AsyncFunctionReturn` | builtin (JS) | 2 |  |
| 373 | `GeneratorFunctionConstructor` | builtin (JS) | 0 + rest |  |
| 374 | `AsyncGeneratorFunctionConstructor` | builtin (JS) | 0 + rest |  |
| 375 | `AsyncFunctionConstructor` | builtin (JS) | 0 + rest |  |
| 376 | `DynamicImport` | builtin (JS) | 2 |  |
| 377 | `GetIterator` | builtin (JS) | 1 |  |
| 378 | `IteratorClose` | builtin (JS) | 1 |  |
| 379 | `IteratorCloseThrow` | builtin (JS) | 1 |  |
| 380 | `IteratorRest` | builtin (JS) | 2 |  |
| 381 | `ArrayPrototypeValues` | builtin (JS) | 0 |  |
| 382 | `ArrayIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 383 | `StringPrototypeIterator` | builtin (JS) | 0 |  |
| 384 | `StringIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 385 | `SerializeJSONProperty` | builtin (JS) | 7 |  |
| 386 | `JSONStackIncludes` | builtin (JS) | 2 |  |
| 387 | `JSONJoin` | builtin (JS) | 7 |  |
| 388 | `SerializeJSONObject` | builtin (JS) | 6 |  |
| 389 | `SerializeJSONArray` | builtin (JS) | 6 |  |
| 390 | `AddEntriesFromIterable` | builtin (JS) | 3 |  |
| 391 | `MapPrototypeSet` | builtin (JS) | 2 |  |
| 392 | `MapIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 393 | `ResolvePromise` | builtin (JS) | 2 |  |
| 394 | `PromiseResolveFunction` | builtin (JS) | 1 |  |
| 395 | `PromiseRejectFunction` | builtin (JS) | 1 |  |
| 396 | `GetCapabilitiesExecutor` | builtin (JS) | 2 |  |
| 397 | `PromiseReactionJob` | builtin (JS) | 4 |  |
| 398 | `PromiseResolveThenableJob` | builtin (JS) | 3 |  |
| 399 | `PromiseConstructor` | builtin (JS) | 1 |  |
| 400 | `PromiseSpecies` | builtin (JS) | 0 |  |
| 401 | `PromiseStaticResolve` | builtin (JS) | 1 |  |
| 402 | `PromiseAllResolveElement` | builtin (JS) | 1 |  |
| 403 | `PromiseAllSettledResolveElement` | builtin (JS) | 1 |  |
| 404 | `PromiseAllSettledRejectElement` | builtin (JS) | 1 |  |
| 405 | `PromiseAnyRejectElement` | builtin (JS) | 1 |  |
| 406 | `PromisePrototypeThen` | builtin (JS) | 2 |  |
| 407 | `PromiseThenFinally` | builtin (JS) | 1 |  |
| 408 | `PromiseCatchFinally` | builtin (JS) | 1 |  |
| 409 | `PromiseValueThunk` | builtin (JS) | 0 |  |
| 410 | `PromiseThrower` | builtin (JS) | 0 |  |
| 411 | `HostCallSettle` | builtin (JS) | 3 |  |
| 412 | `ProxyRevokeFunction` | builtin (JS) | 0 |  |
| 413 | `ProxyGetPrototypeOf` | builtin (JS) | 1 |  |
| 414 | `ProxySetPrototypeOf` | builtin (JS) | 2 |  |
| 415 | `ProxyIsExtensible` | builtin (JS) | 1 |  |
| 416 | `ProxyPreventExtensions` | builtin (JS) | 1 |  |
| 417 | `ProxyGetOwnPropertyDescriptorObject` | builtin (JS) | 2 |  |
| 418 | `ProxyHasOwnProperty` | builtin (JS) | 2 |  |
| 419 | `ProxyOwnPropertyAttributes` | builtin (JS) | 2 |  |
| 420 | `ProxyDefineOwnPropertyFromRecord` | builtin (JS) | 3 |  |
| 421 | `ProxyDefineDataPropertyOrThrow` | builtin (JS) | 4 |  |
| 422 | `ProxyCreateDataProperty` | builtin (JS) | 3 |  |
| 423 | `ProxyCreateDataPropertyOrThrow` | builtin (JS) | 3 |  |
| 424 | `ProxyHas` | builtin (JS) | 2 |  |
| 425 | `ProxyGet` | builtin (JS) | 3 |  |
| 426 | `ProxySet` | builtin (JS) | 4 |  |
| 427 | `ProxySetOrThrow` | builtin (JS) | 4 |  |
| 428 | `ProxySetOnReceiver` | builtin (JS) | 3 |  |
| 429 | `ProxyDeleteProperty` | builtin (JS) | 2 |  |
| 430 | `ProxyDeletePropertyOrThrow` | builtin (JS) | 2 |  |
| 431 | `ProxyOwnPropertyKeys` | builtin (JS) | 2 |  |
| 432 | `ProxyCall` | builtin (JS) | 3 |  |
| 433 | `ProxyConstruct` | builtin (JS) | 3 |  |
| 434 | `OrdinaryConstructSlow` | builtin (JS) | 3 |  |
| 435 | `ProxySetIntegrityLevel` | builtin (JS) | 2 |  |
| 436 | `ProxyTestIntegrityLevel` | builtin (JS) | 2 |  |
| 437 | `ForInStep` | builtin (JS) | 1 |  |
| 438 | `RegExpSpecies` | builtin (JS) | 0 |  |
| 439 | `RegExpPrototypeHasIndices` | builtin (JS) | 0 |  |
| 440 | `RegExpPrototypeGlobal` | builtin (JS) | 0 |  |
| 441 | `RegExpPrototypeIgnoreCase` | builtin (JS) | 0 |  |
| 442 | `RegExpPrototypeMultiline` | builtin (JS) | 0 |  |
| 443 | `RegExpPrototypeDotAll` | builtin (JS) | 0 |  |
| 444 | `RegExpPrototypeUnicode` | builtin (JS) | 0 |  |
| 445 | `RegExpPrototypeUnicodeSets` | builtin (JS) | 0 |  |
| 446 | `RegExpPrototypeSticky` | builtin (JS) | 0 |  |
| 447 | `RegExpPrototypeFlags` | builtin (JS) | 0 |  |
| 448 | `RegExpPrototypeExec` | builtin (JS) | 1 |  |
| 449 | `RegExpPrototypeMatch` | builtin (JS) | 1 |  |
| 450 | `SetPrototypeAdd` | builtin (JS) | 1 |  |
| 451 | `SetIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 452 | `ArraySpread` | builtin (JS) | 2 |  |
| 453 | `CopyDataProperties` | builtin (JS) | 2 |  |
| 454 | `CopyDataPropertiesExcluding` | builtin (JS) | 2 |  |
| 455 | `TypedArraySetElementSlow` | builtin (JS) | 3 |  |
| 456 | `FinalizationRegistryCleanupJob` | builtin (JS) | 1 |  |
<!-- /gen -->
