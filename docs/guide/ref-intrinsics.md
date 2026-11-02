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
| 8 | `AsObj` | leaf | 1 |  |
| 9 | `AsFn` | leaf | 1 |  |
| 10 | `AsStr` | leaf | 1 |  |
| 11 | `AsNumber` | leaf | 1 |  |
| 12 | `AsSym` | leaf | 1 |  |
| 13 | `AsBool` | leaf | 1 |  |
| 14 | `ToBoolean` | leaf | 1 | ToBoolean ( argument ). |
| 15 | `ToNumberPrimitive` | leaf | 1 | ToNumber of a primitive (TypeError for symbols); objects go through ToPrimitive first. |
| 16 | `ToStringPrimitive` | leaf | 1 | ToString of a primitive (TypeError for symbols); objects go through ToPrimitive first. |
| 17 | `ToObject` | leaf | 1 | ToObject ( argument ): TypeError for undefined and null. |
| 18 | `ToIntegerOrInfinityNumber` | leaf | 1 | ToIntegerOrInfinity of a number. |
| 19 | `F64ToU32` | leaf | 1 | An integral f64 in u32 range to u32 (an internal error otherwise). |
| 20 | `F64ToI32` | leaf | 1 | An integral f64 in i32 range to i32 (an internal error otherwise). |
| 21 | `StrLength` | leaf | 1 | The length of a string in code units. |
| 22 | `Get` | reentrant | 2 | Get ( O, P ): may call a getter. |
| 23 | `GetV` | reentrant | 2 | GetV ( V, P ): Get with a primitive base looked up on its prototype. |
| 24 | `Set` | reentrant | 3 | Set ( O, P, V, true ): may call a setter; throws on failure. |
| 25 | `HasProperty` | reentrant | 2 | HasProperty ( O, P ): a proxy's `has` trap. |
| 26 | `CreateDataPropertyOrThrow` | reentrant | 3 | CreateDataPropertyOrThrow ( O, P, V ): a proxy's `defineProperty` trap. |
| 27 | `DeletePropertyOrThrow` | reentrant | 2 | DeletePropertyOrThrow ( O, P ): a proxy's `deleteProperty` trap. |
| 28 | `Call` | reentrant | 2 + rest | Call ( F, V, args ): TypeError if f is not callable. |
| 29 | `Construct` | reentrant | 1 + rest | Construct ( F, args ) with newTarget F: TypeError if f is not a constructor. |
| 30 | `ObjectCreate` | leaf | 1 | OrdinaryObjectCreate ( proto ). |
| 31 | `ArrayCreate` | leaf | 1 | ArrayCreate ( length ): RangeError above 2**32 - 1. |
| 32 | `NewTypeError` | leaf | 1 | A new TypeError of the current realm. |
| 33 | `NewNotConstructorError` | leaf | 1 | A TypeError "<name> is not a constructor" with f's internal name, as QuickJS words it. |
| 34 | `NewSyntaxError` | leaf | 1 | A new SyntaxError of the current realm. |
| 35 | `NewReferenceError` | leaf | 1 | A new ReferenceError of the current realm. |
| 36 | `NewRangeError` | leaf | 1 | A new RangeError of the current realm. |
| 37 | `WellKnownSymbol` | leaf | 1 | The well-known symbol Symbol.<name>. |
| 38 | `RealmIntrinsic` | leaf | 1 | A realm intrinsic object by name: ObjectPrototype, FunctionPrototype, ArrayPrototype, ErrorPrototype and the native error prototypes (TypeErrorPrototype, ...). |
| 39 | `ErrorCreate` | leaf | 1 | An object with [[ErrorData]] and prototype `proto`. |
| 40 | `DOMExceptionCreate` | leaf | 3 | A DOMException: an object with [[ErrorData]], its name and message in slots (D1840). |
| 41 | `DOMExceptionField` | leaf | 2 | Slot i (0 name, 1 message) of a DOMException; undefined if v is not one. |
| 42 | `DOMExceptionCode` | leaf | 1 | The legacy code of a DOMException name, 0 if it has none. |
| 43 | `DOMExceptionConstant` | leaf | 1 | The name of the legacy code constant i, 1 to 25 (`INDEX_SIZE_ERR`, ...). |
| 44 | `Btoa` | leaf | 1 | btoa's base64 of the code units of s; undefined if one is above 0xFF. |
| 45 | `Atob` | leaf | 1 | atob's forgiving-base64 decoding of s into a string of bytes; undefined if invalid. |
| 46 | `CaptureStack` | leaf | 1 | Records `o.stack` (D1282) from the current frames, without the prelude frames at the top: an Error constructor does not show itself. With Error.prepareStackTrace it is pending until FinishErrorStack (D2050). |
| 47 | `ErrorStack` | leaf | 1 | The stack an Error object recorded (D1971), a pending one too (D2050), or undefined for any other value. |
| 48 | `StackTraceValue` | leaf | 1 | The stack trace of the current frames (the calling builtin left out): a string, or a pending stack with Error.prepareStackTrace (D2050). With a function `below`, only the frames below its topmost call, none if it is not on the stack (Error.captureStackTrace, D2010). |
| 49 | `IsPendingStack` | leaf | 1 | Whether v is a stack waiting for Error.prepareStackTrace (D2050). |
| 50 | `SetErrorStack` | leaf | 2 | Sets the stack slot of an Error (an own `stack` of anything else). |
| 51 | `CallSiteSlot` | leaf | 2 | Slot i of a CallSite (0 function, 1 name, 2 file, 3 line, 4 column, 5 native, 6 constructor); TypeError "CallSite object expected" for anything else. |
| 52 | `DefineDataPropertyOrThrow` | reentrant | 4 | DefinePropertyOrThrow ( O, P, desc ) with a complete data descriptor; `attrs` are writable 1, enumerable 2, configurable 4. |
| 53 | `DefineOwnPropertyFromRecord` | reentrant | 3 | [[DefineOwnProperty]] with a descriptor record from ToPropertyDescriptor (an object whose own data properties are the present fields). False if it was rejected. |
| 54 | `GetOwnPropertyDescriptorObject` | reentrant | 2 | FromPropertyDescriptor ( [[GetOwnProperty]](o, key) ): undefined if absent. |
| 55 | `HasOwnProperty` | reentrant | 2 | HasOwnProperty ( O, P ). |
| 56 | `OwnPropertyAttributes` | reentrant | 2 | The attributes of an own property: -1 if absent, else writable 1, enumerable 2, configurable 4, accessor 8. |
| 57 | `OwnPropertyKeys` | reentrant | 2 | The own keys of o as a new array, in [[OwnPropertyKeys]] order: kind 0 strings, 1 symbols, 2 both, 3 enumerable strings. |
| 58 | `GetPrototypeOf` | reentrant | 1 | [[GetPrototypeOf]] ( ): an object or null. |
| 59 | `SetPrototypeOf` | reentrant | 2 | [[SetPrototypeOf]] ( V ) with an object or null: false if refused. |
| 60 | `PreventExtensions` | reentrant | 1 | [[PreventExtensions]] ( ). |
| 61 | `IsExtensible` | reentrant | 1 | [[IsExtensible]] ( ). |
| 62 | `SetIntegrityLevel` | reentrant | 2 | SetIntegrityLevel ( O, level ): sealed, or frozen if `frozen`. |
| 63 | `TestIntegrityLevel` | reentrant | 2 | TestIntegrityLevel ( O, level ): sealed, or frozen if `frozen`. |
| 64 | `SameValue` | leaf | 2 | SameValue ( x, y ). |
| 65 | `ObjectClassTag` | leaf | 1 | The builtinTag of Object.prototype.toString: Array, Arguments, Function, Error, Boolean, Number, String, Date, RegExp, else Object. |
| 66 | `WrapPrimitive` | leaf | 2 | A Boolean, Number, String or Symbol wrapper of the primitive v with prototype proto. |
| 67 | `PrimitiveValue` | leaf | 1 | The primitive of a wrapper object ([[BooleanData]] and so on), a primitive itself, or undefined for other objects. |
| 68 | `ConstructWithArray` | reentrant | 3 | Construct ( F, args, newTarget ) with the arguments in a dense array. |
| 69 | `ConstructWithThis` | reentrant | 4 | [[Construct]] of an ordinary (non-builtin) function f with `thisArg` as the new object (OrdinaryCreateFromConstructor done by the caller) and new.target. |
| 70 | `GetWithReceiver` | reentrant | 3 | [[Get]] ( P, Receiver ): a getter runs with `receiver` as this. |
| 71 | `SetWithReceiver` | reentrant | 4 | OrdinarySet ( O, P, V, Receiver ): false when refused; a setter runs with receiver. |
| 72 | `DeleteProperty` | reentrant | 2 | [[Delete]] ( P ): false for a non-configurable property. |
| 73 | `IsProxy` | leaf | 1 | Whether v is a Proxy exotic object (revoked or not). |
| 74 | `NewProxy` | leaf | 2 | A new proxy (ProxyCreate's object, 10.5.14): callable/constructor as target is. |
| 75 | `ProxyTarget` | leaf | 1 | [[ProxyTarget]] of a proxy: TypeError "revoked proxy" once revoked. |
| 76 | `ProxyHandler` | leaf | 1 | [[ProxyHandler]] of a proxy: TypeError "revoked proxy" once revoked. |
| 77 | `ProxyRevoke` | leaf | 1 | Revokes a proxy (target and handler become null). |
| 78 | `CollectionCreate` | leaf | 2 | A new empty collection: class 0 Map, 1 Set, 2 WeakMap, 3 WeakSet. |
| 79 | `CollectionClass` | leaf | 1 | The class of a collection object (0 Map, 1 Set, 2 WeakMap, 3 WeakSet), -1 otherwise. |
| 80 | `CollectionGet` | leaf | 2 | The value of key (SameValueZero), or undefined. |
| 81 | `CollectionHas` | leaf | 2 | Whether key is present. |
| 82 | `CollectionSet` | leaf | 3 | Adds or replaces key (-0 becomes +0); a Set passes true as the value. |
| 83 | `CollectionDelete` | leaf | 2 | Removes key; false if it was absent. |
| 84 | `CollectionClear` | leaf | 1 | Removes every entry. |
| 85 | `CollectionSize` | leaf | 1 | The number of entries. |
| 86 | `CollectionIterator` | leaf | 2 | A Map Iterator or Set Iterator over m (kind 0 keys, 1 values, 2 entries): it visits entries added while iterating and skips deleted ones. |
| 87 | `CollectionIteratorNext` | leaf | 2 | %MapIteratorPrototype%.next / %SetIteratorPrototype%.next: the iterator result. TypeError "Map Iterator object expected" (or Set) for anything else. |
| 88 | `IsOriginalCollectionAdder` | leaf | 2 | Whether `adder` is the installed Map.prototype.set (kind 0) or Set.prototype.add (1): the constructors then store directly. |
| 89 | `CollectionIteratorStep` | leaf | 1 | Advances an iterator to the next entry; false when done. |
| 90 | `CollectionIteratorKey` | leaf | 1 | The key of the entry the last step reached. |
| 91 | `CollectionIteratorValue` | leaf | 1 | The value of the entry the last step reached. |
| 92 | `CanBeHeldWeakly` | leaf | 1 | CanBeHeldWeakly ( v ): an object, or a symbol not in the Symbol.for registry. |
| 93 | `DateNow` | leaf | 0 | The host's clock (ms since the epoch; Vm::set_now). |
| 94 | `DateCreate` | leaf | 2 | A new Date object with prototype proto and time value t. |
| 95 | `DateValue` | leaf | 1 | thisTimeValue: the time value of a Date, TypeError "not a Date object" otherwise. |
| 96 | `DateSetValue` | leaf | 2 | Stores time value t in a Date (TypeError "not a Date object" otherwise); returns t. |
| 97 | `DateFromFields` | leaf | 2 | A time value from the Numbers year, month[, date, hours, minutes, seconds, ms] in a dense array (the constructor's and Date.UTC's form: years 0-99 are 1900-1999). |
| 98 | `DateGetField` | leaf | 3 | Field `field` of t (0 year, 1 month, 2 date, 3 hours, 4 minutes, 5 seconds, 6 ms, 7 weekday), local or UTC; NaN for an invalid date. |
| 99 | `DateSetFields` | leaf | 4 | A setter's new time value: fields first.. of t replaced by the Numbers in args (a dense array), local or UTC (qjs's set_date_field). |
| 100 | `DateFormat` | leaf | 3 | Formats t: fmt 0 toUTCString, 1 toString, 2 toISOString (RangeError for NaN), 3 toLocale*; part 1 date, 2 time, 3 both. "Invalid Date" for NaN. |
| 101 | `DateParse` | leaf | 1 | Date.parse of a string (ISO and the formats qjs's toString/toUTCString emit). |
| 102 | `DateTimezoneOffset` | leaf | 1 | getTimezoneOffset of t: minutes (UTC - local), NaN for an invalid date. |
| 103 | `ArrayBufferCreate` | leaf | 3 | AllocateArrayBuffer: byteLength zeroed bytes; max is the maxByteLength of a resizable buffer, true for an immutable one (D1970), or undefined. RangeError "invalid array buffer length" (or "... max length"). |
| 104 | `BufferInfo` | leaf | 2 | A field of an ArrayBuffer: 0 byteLength, 1 maxByteLength, 2 resizable, 3 detached, 4 immutable (D1970). TypeError "ArrayBuffer object expected" for anything else. |
| 105 | `BufferDetach` | leaf | 1 | DetachArrayBuffer. |
| 106 | `BufferResize` | leaf | 2 | Resizes a resizable buffer in place (new bytes are 0). |
| 107 | `BufferTransfer` | leaf | 3 | A new buffer with the first bytes of b (max: a Number for resizable, true for immutable), b detached; an immutable b is a TypeError. |
| 108 | `BufferCopy` | leaf | 5 | Copies count bytes from src at from to dst at to (memmove). |
| 109 | `SortNumbers` | leaf | 1 | Sorts a dense array of Numbers numerically in place (a typed array's default order: -0 before +0, NaN last; stable), or one of BigInts by value. |
| 110 | `DataViewCreate` | leaf | 4 | A DataView over buffer (byteLength -1: length-tracking). |
| 111 | `DataViewField` | leaf | 2 | A slot of a DataView: 0 buffer, 1 byteOffset, 2 byteLength (-1: length-tracking). TypeError "DataView object expected" for anything else. |
| 112 | `DataViewGet` | leaf | 4 | The element of kind at byte `at` of the view's buffer (bounds checked by the caller): a Number, or a BigInt for kinds 10 and 11. |
| 113 | `DataViewSet` | leaf | 5 | Writes v (a Number, or a BigInt for kinds 10 and 11) as kind at byte `at` of the view's buffer. |
| 114 | `IsDataView` | leaf | 1 | Whether v is a DataView. |
| 115 | `RealmConstructor` | leaf | 1 | The constructor installed as `name` (the realm's %Name%, whatever the global is now). |
| 116 | `IsTypedArray` | leaf | 1 | Whether v is a typed array. |
| 117 | `TypedArrayCreate` | leaf | 5 | A typed array of `kind` over buffer (length -1: length-tracking). |
| 118 | `TypedArrayField` | leaf | 2 | A field of a typed array: 0 kind, 1 buffer, 2 byteOffset, 3 length, 4 byteLength (2-4 are 0 when out of bounds), 5 out of bounds or detached, 6 length-tracking, 7 [[ByteOffset]] (also when out of bounds). TypeError "not a TypedArray" for anything else. |
| 119 | `TypedArrayGet` | leaf | 2 | Element i (undefined if not a valid index). |
| 120 | `TypedArraySet` | leaf | 3 | Writes v (a Number, or a BigInt for a BigInt array) to element i (nothing if not a valid index). |
| 121 | `TypedArrayCopy` | leaf | 5 | Copies count elements from src (at from) to dst (at to), converting; overlaps ok. |
| 122 | `TypedArrayKindProto` | leaf | 1 | The prototype of the typed array constructor for kind (the realm's). |
| 123 | `Base64Encode` | leaf | 3 | The bytes of a Uint8Array as base64 (TypeError if detached or out of bounds). |
| 124 | `HexEncode` | leaf | 1 | The bytes of a Uint8Array as lowercase hex (TypeError if detached). |
| 125 | `Base64Decode` | leaf | 4 | FromBase64 ( string, alphabet, lastChunkHandling, maxLength ): a record object { read, bytes (an ArrayBuffer), error } (last: 0 loose, 1 strict, 2 stop-before-partial). |
| 126 | `HexDecode` | leaf | 2 | FromHex ( string, maxLength ): the same record. |
| 127 | `CallWithArray` | reentrant | 3 | Call ( F, V, args ) with the arguments in a dense array. |
| 128 | `BoundFunctionCreate` | leaf | 3 | BoundFunctionCreate ( target, boundThis, boundArgs ), without name and length. |
| 129 | `BoundTargetFunction` | leaf | 1 | [[BoundTargetFunction]] of a bound function, or undefined for anything else. |
| 130 | `FunctionToString` | leaf | 1 | The source text of a function (natives: `function name() { [native code] }`). |
| 131 | `NumberPow` | leaf | 2 | Number::exponentiate ( base, exponent ). |
| 132 | `SymbolDescriptiveString` | leaf | 1 | SymbolDescriptiveString ( sym ): `Symbol(description)`. |
| 133 | `CompareStrings` | leaf | 2 | Compares two strings by code units (IsLessThan for strings): -1, 0 or 1. Also localeCompare's order (no Intl). |
| 134 | `StringCodePoints` | leaf | 1 | The code points of s as an array of strings (the string iterator's values). |
| 135 | `SortPrimitives` | leaf | 1 | Sorts the dense array `list` in place by SortCompare without a comparefn (undefined last, then ToString order; stable) if every element is a primitive other than a symbol. Returns false, leaving it unchanged, otherwise. |
| 136 | `ArraySpliceDense` | leaf | 4 | Splice's element moves in one step on a plain dense Array (no holes, default attributes, extensible, writable length, no index properties on the prototype chain): o[start .. start + deleteCount) becomes itemCount undefined slots and `length` follows. False, with nothing changed, for any other object. |
| 137 | `ArrayCanPopDense` | leaf | 1 | Whether `ArrayPopDense(o)` applies: an Array with only dense elements, a writable length, its last element present and no index properties on its prototype chain, where pop's Get, DeletePropertyOrThrow and Set observe nothing else. |
| 138 | `ArrayPopDense` | leaf | 1 | Array.prototype.pop of an array `ArrayCanPopDense` accepts. |
| 139 | `ArrayFillDense` | leaf | 4 | fill's Sets on an Array while each is a plain store (an existing dense element, or an append or hole fill of an extensible Array without index properties on its prototype chain), from k up to fin: the index the general loop continues from. |
| 140 | `ArrayPushDense` | leaf | 2 | push(v)'s Set and length update in one step when the Set is a plain append to the Array `o` (extensible, writable length, no holes at its end, no index properties on its prototype chain). The new length, or -1 with nothing changed. |
| 141 | `ArraySearchDense` | leaf | 5 | The search loops of indexOf (mode 0: `from` up to len, strict equality), lastIndexOf (1: `from` down to 0) and includes (2: up to len, SameValueZero) over an Array whose elements in the range are all present in dense storage, where HasProperty and Get observe nothing: the index found or -1. -2, for the general loop, for anything else (a hole, a shorter array, another object). |
| 142 | `ArraySliceDense` | leaf | 4 | slice's copy loop: `a[0 .. fin - k) = o[k .. fin)` when every element of `o` in the range is present in dense storage and `a` is an extensible Array without index properties (a fresh ArraySpeciesCreate result), so HasProperty, Get and CreateDataPropertyOrThrow observe nothing. False, with nothing changed, otherwise. |
| 143 | `EachDense` | reentrant | 7 | The dense part of the loops of the callback builtins. From `k`, while `o` is an Array whose element k is present in its dense store (so HasProperty and Get observe nothing), calls `f(o[k], k, o)` with `thisArg` and completes the step by `mode`: - 0 forEach; - 1 map: CreateDataPropertyOrThrow(a, k, result); - 2 filter: for a truthy result, CreateDataPropertyOrThrow(a, to, o[k]), with `to` the length of `a`, a fresh Array (the receiver's species is plain); - 3 some, 4 every: a truthy (some) or falsy (every) result decides; - 5 find: a truthy result decides, and o[k] goes to element 0 of the Array `a`; - 6 findIndex: a truthy result decides; - 7 reduce: calls `f(acc, o[k], k, o)` (`thisArg` undefined), with the accumulator as element 0 of the Array `a`, which gets the result; - 8 Array.from of an array-like: calls `f(o[k], k)` and does CreateDataPropertyOrThrow(a, k, result); `o` is any ordinary object (or array or function) whose element k, along its prototype chain, is a data property or absent (undefined), so Get observes nothing (D2306). One frame for `f`, entered again per element. Returns the k the builtin's own loop goes on from (`len`: done; `k` itself when `f` is not a plain closure or `a` is not as required), or -1 - k when the result at k decided. |
| 144 | `ArrayJoinDense` | leaf | 3 | join over o[0 .. len) when every element is present in dense storage and a primitive other than a symbol (ToString runs no user code): the string, or undefined for the general loop. |
| 145 | `EnumerableOwnDataProperties` | leaf | 2 | EnumerableOwnProperties ( o, kind ) for kind 1 (values) and 2 (entries) in one step when no enumerable string-keyed own property is an accessor and `o` is not exotic (no Proxy): the array, or undefined for the general loop. |
| 146 | `TypedArraySetFromDense` | leaf | 2 | Writes values[0 .. length) into the typed array `o` (at least that long) when `values` is an Array whose elements are all present in dense storage and primitives of `o`'s content type (ToNumber / ToBigInt run no user code). False, with nothing written, otherwise. |
| 147 | `StrLocaleCompare` | leaf | 2 | localeCompare without Intl (as qjs): the difference of the first differing code points, else -1, 0 or 1 by length. |
| 148 | `StrCharCodeAt` | leaf | 2 | The code unit at i as a Number, NaN out of range. |
| 149 | `StrCodePointAt` | leaf | 2 | The code point at i (a surrogate pair combined), undefined out of range. |
| 150 | `StrSubstring` | leaf | 3 | s[start .. end) in code units; the indexes are integers, clamped by the caller. |
| 151 | `StrIndexOf` | leaf | 3 | The first index ≥ from where search occurs, or -1. |
| 152 | `StrLastIndexOf` | leaf | 3 | The last index ≤ from where search occurs, or -1. |
| 153 | `StrToCase` | leaf | 2 | Full Unicode upper- or lowercase (SpecialCasing, Final_Sigma; no locales). |
| 154 | `StrTrim` | leaf | 2 | Trims WhiteSpace and LineTerminators: where 0 both ends, 1 start, 2 end. |
| 155 | `StrIsWellFormed` | leaf | 1 | IsStringWellFormedUnicode. |
| 156 | `StrToWellFormed` | leaf | 1 | Lone surrogates replaced by U+FFFD. |
| 157 | `StrRepeat` | leaf | 2 | s repeated n times (an integer ≥ 0; RangeError past the string length limit). |
| 158 | `StrFromCodeUnits` | leaf | 1 | A string of the code units ToUint16(n) of the Numbers in a dense array. |
| 159 | `StrFromCodePoints` | leaf | 1 | A string of the (valid) code points in a dense array. |
| 160 | `StrNormalize` | leaf | 2 | String.prototype.normalize for form 0 NFC, 1 NFD, 2 NFKC, 3 NFKD: the normalized string; undefined only when djs is built without the `unicode-normalization` feature and the string would need its tables. |
| 161 | `NewSymbol` | leaf | 1 | A new symbol; the description is a string or undefined. |
| 162 | `SymbolDescription` | leaf | 1 | [[Description]] of a symbol: a string or undefined. |
| 163 | `SymbolFor` | leaf | 1 | Symbol.for's GlobalSymbolRegistry lookup or insertion. |
| 164 | `SymbolKeyFor` | leaf | 1 | Symbol.keyFor's registry lookup: the key, or undefined. |
| 165 | `NumberToString` | leaf | 2 | Number::toString ( x, radix ) for an integral radix (RangeError outside 2..36), Number.prototype.toString. |
| 166 | `NumberToFixed` | leaf | 2 | Number.prototype.toFixed after ToIntegerOrInfinity(fractionDigits). |
| 167 | `NumberToExponential` | leaf | 2 | Number.prototype.toExponential; digits is undefined or an integral Number. |
| 168 | `NumberToPrecision` | leaf | 2 | Number.prototype.toPrecision; precision is undefined or an integral Number. |
| 169 | `StringParseFloat` | leaf | 1 | The global parseFloat after ToString. |
| 170 | `StringParseInt` | leaf | 2 | The global parseInt after ToString; radix is ToInt32(radix). |
| 171 | `UriEncode` | leaf | 2 | Encode ( string, extraUnescaped ), 19.2.6.5, after ToString: encodeURI keeps the reserved characters (`component` false), encodeURIComponent escapes them. URIError for a lone surrogate. |
| 172 | `UriDecode` | leaf | 2 | Decode ( string, preserveEscapeSet ), 19.2.6.6, after ToString: decodeURI keeps the escapes of reserved characters (`component` false). URIError for a malformed escape or UTF-8 sequence. |
| 173 | `StrEscape` | leaf | 1 | escape ( string ), B.2.1.1, after ToString. |
| 174 | `StrUnescape` | leaf | 1 | unescape ( string ), B.2.1.2, after ToString. |
| 175 | `CompileEval` | leaf | 1 | PerformEval's parse (19.2.1.1) for indirect eval: the source as a Script closure, to be called with `this` = the global object. SyntaxError for invalid source. Charges fuel per source byte. |
| 176 | `CompileFunction` | leaf | 3 | CreateDynamicFunction's parse (20.2.1.1.1): `kind` 0 function, 1 generator, 2 async, 3 async generator; `params` joined with ","; a Script closure whose call returns the new function. SyntaxError for invalid source. Charges fuel per source byte. |
| 177 | `GlobalObject` | leaf | 0 | The realm's global object. |
| 178 | `EnqueueMicrotask` | leaf | 1 | Queues the job MicrotaskJob(callback) (queueMicrotask). |
| 179 | `NumberToInt32` | leaf | 1 | ToInt32 of a number. |
| 180 | `NumberToUint32` | leaf | 1 | ToUint32 of a number, as an f64. |
| 181 | `F64Abs` | leaf | 1 |  |
| 182 | `F64Acos` | leaf | 1 |  |
| 183 | `F64Acosh` | leaf | 1 |  |
| 184 | `F64Asin` | leaf | 1 |  |
| 185 | `F64Asinh` | leaf | 1 |  |
| 186 | `F64Atan` | leaf | 1 |  |
| 187 | `F64Atanh` | leaf | 1 |  |
| 188 | `F64Atan2` | leaf | 2 |  |
| 189 | `F64Cbrt` | leaf | 1 |  |
| 190 | `F64Hypot` | leaf | 2 |  |
| 191 | `F64Ceil` | leaf | 1 |  |
| 192 | `F64Clz32` | leaf | 1 | Math.clz32 of a number (ToUint32 inside). |
| 193 | `F64Cos` | leaf | 1 |  |
| 194 | `F64Cosh` | leaf | 1 |  |
| 195 | `F64Exp` | leaf | 1 |  |
| 196 | `F64Expm1` | leaf | 1 |  |
| 197 | `F64Floor` | leaf | 1 |  |
| 198 | `F64Fround` | leaf | 1 | Rounds to the nearest f32. |
| 199 | `F64F16round` | leaf | 1 | Rounds to the nearest binary16 value (Math.f16round). |
| 200 | `F64SumPrecise` | leaf | 1 | The exactly rounded sum of the Numbers of the dense Array list (Math.sumPrecise). |
| 201 | `F64Imul` | leaf | 2 | Math.imul of two numbers (ToUint32 inside). |
| 202 | `F64Log` | leaf | 1 |  |
| 203 | `F64Log1p` | leaf | 1 |  |
| 204 | `F64Log10` | leaf | 1 |  |
| 205 | `F64Log2` | leaf | 1 |  |
| 206 | `F64Round` | leaf | 1 | Math.round: half up, -0 kept. |
| 207 | `F64Sign` | leaf | 1 |  |
| 208 | `F64Sin` | leaf | 1 |  |
| 209 | `F64Sinh` | leaf | 1 |  |
| 210 | `F64Sqrt` | leaf | 1 |  |
| 211 | `F64Tan` | leaf | 1 |  |
| 212 | `F64Tanh` | leaf | 1 |  |
| 213 | `F64Trunc` | leaf | 1 |  |
| 214 | `MathRandom` | leaf | 0 | The next number in [0, 1) of the realm's generator (xorshift128+, seeded by the host). |
| 215 | `CreateArrayIterator` | leaf | 2 | A new Array Iterator over o (kind 0: keys, 1: values, 2: entries), 23.1.5.1. |
| 216 | `ArrayIteratorTarget` | leaf | 1 | The object an Array Iterator iterates, or undefined once it is exhausted. TypeError "Array Iterator object expected" if it is not an Array Iterator. |
| 217 | `ArrayIteratorIndex` | leaf | 1 | The next index of an Array Iterator. |
| 218 | `ArrayIteratorSetIndex` | leaf | 2 |  |
| 219 | `ArrayIteratorFinish` | leaf | 1 | Marks an Array Iterator exhausted (its target becomes undefined). |
| 220 | `ArrayIteratorKind` | leaf | 1 | The kind of an Array Iterator (0: keys, 1: values, 2: entries). |
| 221 | `CreateStringIterator` | leaf | 1 | A String Iterator over s (CreateStringIterator, 22.1.5.1): an Array Iterator object of kind 3 with %StringIteratorPrototype%; ArrayIteratorIndex/SetIndex/Finish apply. |
| 222 | `StringIteratorTarget` | leaf | 1 | The string a String Iterator iterates, or undefined once it is exhausted. TypeError "String Iterator object expected" otherwise. |
| 223 | `StrCodePointString` | leaf | 2 | The code point at index i of s as a string (one or two code units). |
| 224 | `HasOriginalIteration` | leaf | 1 | Whether v is an Array or a string whose iteration is the original Symbol.iterator method and `next` (data properties only): iterating it then equals reading it directly. |
| 225 | `IteratorFastReady` | leaf | 2 | Whether the next step of `iterator` with `next` gives a value without running user code: `next` is the original one of an Array or String Iterator and the value is at hand (D1800). Then `IteratorFastValue` returns it; otherwise call `next`. |
| 226 | `IteratorFastValue` | leaf | 2 | The next value of an iterator `IteratorFastReady` accepts (it advances). |
| 227 | `IteratorFastDrain` | leaf | 4 | The values `IteratorFastValue` would give one by one for a keys or values Array Iterator, stored with CreateDataPropertyOrThrow into the Array `arr` at n, n + 1, ... in one step, while they are plain dense stores (D2305): the next n. Stops before a value only `next` can read (a hole), at the end (the next step finishes the iterator) and where a store is not plain; gives n back unchanged for anything else. |
| 228 | `CreateIterResultObject` | leaf | 2 | CreateIterResultObject ( value, done ), 7.4.14. |
| 229 | `CreateArrayFromPair` | leaf | 2 | A new Array [a, b]. |
| 230 | `PromiseCreate` | leaf | 1 | A new pending Promise with prototype proto. |
| 231 | `IsPromise` | leaf | 1 | Whether v is a Promise object (has [[PromiseState]]). |
| 232 | `IntrinsicPromise` | leaf | 0 | The realm's %Promise% constructor. |
| 233 | `PromiseState` | leaf | 1 | [[PromiseState]]: 0 pending, 1 fulfilled, 2 rejected. |
| 234 | `PromiseResult` | leaf | 1 | [[PromiseResult]]. |
| 235 | `FulfillPromise` | leaf | 2 | FulfillPromise ( promise, value ), 27.2.1.4: reactions become jobs. No-op unless pending. |
| 236 | `RejectPromise` | leaf | 2 | RejectPromise ( promise, reason ), 27.2.1.7, with the rejection tracker. No-op unless pending. |
| 237 | `PromiseClaimResolve` | leaf | 1 | Sets the [[AlreadyResolved]] of p's own resolving functions (and internal capability); false if it was already set. |
| 238 | `PerformPromiseThen` | leaf | 4 | PerformPromiseThen ( promise, onFulfilled, onRejected, resultCapability ), 27.2.5.4.1, without the result: handlers that are not callable count as empty. |
| 239 | `EnqueuePromiseResolveThenableJob` | leaf | 3 | Queues NewPromiseResolveThenableJob ( promise, thenable, then ), 27.2.2.2. |
| 240 | `NewRecord` | leaf | 1 | A new internal Record with n slots, all undefined. |
| 241 | `RecordGet` | leaf | 2 | Slot i of a Record, or of an object that keeps its state in slots (IteratorHelper). |
| 242 | `RecordSet` | leaf | 3 |  |
| 243 | `NewBuiltinClosure` | leaf | 3 | A function (name "", `length`) that calls the builtin `kind` (the table in interp/run/intrinsics.rs: 0 PromiseResolveFunction, 1 PromiseRejectFunction, 2 GetCapabilitiesExecutor, 3 PromiseAllResolveElement, 4 PromiseAllSettledResolveElement, 5 PromiseAllSettledRejectElement, 6 PromiseAnyRejectElement, 7 PromiseThenFinally, 8 PromiseCatchFinally, 9 PromiseValueThunk, 10 PromiseThrower, 11 AsyncGeneratorAwaitReturnFulfilled, 12 AsyncGeneratorAwaitReturnRejected, 13 AsyncFromSyncIteratorUnwrap, 14 AsyncFromSyncIteratorCloseThrow, 15 ProxyRevokeFunction, 16 FromAsyncFulfilled, 17 FromAsyncRejected, 18 AsyncDisposeFulfilled, 19 AsyncDisposeRejected, 20 DecoratorAddInitializer, 21 DecoratorAccessGet, 22 DecoratorAccessSet, 23 DecoratorAccessHas, 24 DecoratorParamCall) with `this` = data and its own arguments. |
| 244 | `GeneratorState` | leaf | 1 | The state of a generator object (not an async state), or -1 for anything else. |
| 245 | `GeneratorSetState` | leaf | 2 |  |
| 246 | `GeneratorResume` | reentrant | 3 | Resumes a suspended generator or async function (state 0 or 1) in a frame of its own: mode 0 continues with value (a generator at a yield gets `value false`, an async function the value), 1 throws value at the suspension point, 2 (generators at a yield) returns value from there (`value true`). The result is what the activation hands back when it next suspends or completes. |
| 247 | `IsAsyncFunctionState` | leaf | 1 | Whether v is the state of an async function call. |
| 248 | `AsyncFunctionPromise` | leaf | 1 | The result promise of an async function call. |
| 249 | `AsyncGeneratorState` | leaf | 1 | The state of an async generator, or -1 for anything else. |
| 250 | `AsyncGeneratorEnqueue` | leaf | 2 | Appends a request to the generator's queue. |
| 251 | `AsyncGeneratorFirstRequest` | leaf | 1 | The oldest request, or undefined. |
| 252 | `AsyncGeneratorDequeue` | leaf | 1 | Removes and returns the oldest request (undefined if none). |
| 253 | `CreateAsyncFromSyncIterator` | leaf | 2 | CreateAsyncFromSyncIterator ( syncIteratorRecord ), 27.1.6.1: a Record [iterator, next] with %AsyncFromSyncIteratorPrototype%. |
| 254 | `CreateIteratorHelper` | leaf | 3 | A new IteratorHelper object of `kind` (0: Iterator.from's wrapper, with %WrapForValidIteratorPrototype%; else a helper, with %IteratorHelperPrototype%): slot 0 kind, 1 state 0, 2 iterator, 3 next, the others undefined. |
| 255 | `ThisIteratorHelper` | leaf | 2 | v if it is an iterator helper (wrap false) or Iterator.from's wrapper (wrap true), else TypeError "Iterator Helper object expected" / "Iterator Wrap object expected". |
| 256 | `WeakRefCreate` | leaf | 2 | A new WeakRef with prototype proto and target (AddToKeptObjects ( target ) too). |
| 257 | `WeakRefDeref` | leaf | 1 | WeakRefDeref of v (AddToKeptObjects of a live target): the target, or undefined once it died. TypeError "WeakRef object expected" for anything else. |
| 258 | `FinalizationRegistryCreate` | leaf | 2 | A new FinalizationRegistry with prototype proto and cleanupCallback. |
| 259 | `ThisFinalizationRegistry` | leaf | 1 | v if it is a FinalizationRegistry, else TypeError "FinalizationRegistry object expected". |
| 260 | `FinalizationRegistryRegister` | leaf | 4 | Appends a cell (token undefined for none). |
| 261 | `FinalizationRegistryUnregister` | leaf | 2 | Removes the cells registered with token; whether there were any. |
| 262 | `FinalizationRegistryNextDead` | leaf | 1 | The index of the first cell whose target died, or -1. |
| 263 | `FinalizationRegistryTake` | leaf | 2 | The held value of dead cell i, removing the cell. |
| 264 | `FinalizationRegistrySetQueued` | leaf | 2 | Marks whether a cleanup job for r is queued. |
| 265 | `DisposableStackCreate` | leaf | 2 | A new DisposableStack (isAsync false) or AsyncDisposableStack with prototype proto: slot 0 isAsync, 1 state 0 (pending), 2 a new empty Array of resources. |
| 266 | `ThisDisposableStack` | leaf | 2 | v if it is a DisposableStack (isAsync false) or an AsyncDisposableStack, else TypeError "DisposableStack object expected" / "AsyncDisposableStack object expected". |
| 267 | `JsonParse` | leaf | 2 | JSON.parse of text without a reviver; qjs's SyntaxError for invalid JSON. With withSource, an Array [value, record] where record is the JSON Parse Record tree for the reviver's source text access: [value, source, elements, entries] (D1611). |
| 268 | `JsonRawJSON` | leaf | 1 | JSON.rawJSON's work after ToString: a frozen, null-prototype RawJSON object with the own property rawJSON = text; SyntaxError "invalid rawJSON string" unless text is a JSON primitive without leading or trailing whitespace. |
| 269 | `IsRawJSON` | leaf | 1 | Whether v has [[IsRawJSON]] (a JSON.rawJSON object). |
| 270 | `JsonStringifyFast` | leaf | 2 | JSON.stringify(v, undefined, space) with gap already computed, for plain data: the string, undefined for an undefined result, or null when the value needs the full algorithm (accessors, toJSON, wrappers, exotic objects; nothing observable happened). TypeError "circular reference" and RangeError "Maximum call stack size exceeded" are final. |
| 271 | `JsonStringifyFastAt` | leaf | 4 | JsonStringifyFast for an object the full algorithm reached, after its toJSON and replacer (its own toJSON is not looked at again), nested in the objects of the algorithm's `stack` (a cycle back to one throws; they set the indentation), with an array replacer's PropertyList or undefined: the string, or null when it is not plain data. |
| 272 | `JsonQuote` | leaf | 1 | QuoteJSONString ( value ), 25.5.2.3. |
| 273 | `CreateDataProperty` | reentrant | 3 | CreateDataProperty ( O, P, V ): false if refused. |
| 274 | `IsBigInt` | leaf | 1 |  |
| 275 | `ToBigIntPrimitive` | leaf | 1 | ToBigInt of a primitive: TypeError for undefined, null, numbers and symbols; SyntaxError for a bad string. |
| 276 | `NumberToBigInt` | leaf | 1 | NumberToBigInt ( number ), 21.2.1.1.1: RangeError unless integral. |
| 277 | `BigIntToString` | leaf | 2 | The digits of a BigInt in radix 2..36. |
| 278 | `BigIntAsIntN` | leaf | 2 | BigInt.asIntN on an index and a BigInt. |
| 279 | `BigIntAsUintN` | leaf | 2 | BigInt.asUintN on an index and a BigInt. |
| 280 | `BigIntToNumber` | leaf | 1 | The Number value of a BigInt (rounded to nearest). |
| 281 | `IsRegExpObject` | leaf | 1 | Whether v is a RegExp object (has [[RegExpMatcher]]). |
| 282 | `RegExpCreate` | leaf | 3 | RegExpAlloc + RegExpInitialize after the ToStrings: a new RegExp with prototype proto (%RegExp.prototype% for undefined) and lastIndex 0. qjs's SyntaxError for a bad pattern or flags. Regex literals compile to this. |
| 283 | `RegExpSource` | leaf | 1 | [[OriginalSource]] of a RegExp object. |
| 284 | `RegExpFlags` | leaf | 1 | [[OriginalFlags]] of a RegExp object, in canonical order. |
| 285 | `RegExpHasFlag` | leaf | 2 | Whether a RegExp object has the flag letter `flag`. |
| 286 | `RegExpEscapedSource` | leaf | 1 | EscapeRegExpPattern of [[OriginalSource]] (`(?:)` when empty). |
| 287 | `RegExpExecRaw` | leaf | 4 | RegExpBuiltinExec ( R, S ) once lastIndex is Get(R, "lastIndex") as a Number (ToLength is applied here): the match array or null, and lastIndex updated for g and y. Pass undefined as state: when the fuel runs out mid-match the interpreter suspends here and keeps the matcher's state in that argument. InternalError "out of memory in regexp execution" if the backtracking stack overflows. |
| 288 | `RegExpTestRaw` | leaf | 4 | RegExpExecRaw without the match array: whether it matched (lastIndex is updated the same way). Suspendable like RegExpExecRaw. |
| 289 | `IsOriginalRegExpExec` | leaf | 1 | Whether v is the original RegExp.prototype.exec (the installed builtin). |
| 290 | `CreateRegExpStringIterator` | leaf | 4 | A RegExp String Iterator (CreateRegExpStringIterator, 22.2.9.1). |
| 291 | `RegExpStringIteratorSlot` | leaf | 2 | Slot i of a RegExp String Iterator: 0 the regexp, 1 the string, 2 global, 3 unicode, 4 done. TypeError "RegExp String Iterator object expected" for anything else. |
| 292 | `RegExpStringIteratorFinish` | leaf | 1 | Marks a RegExp String Iterator done. |
| 293 | `RegExpEscapeString` | leaf | 1 | RegExp.escape of a string (EncodeForRegExpEscape, qjs's lowercase hex). |
| 294 | `IsPristineRegExp` | leaf | 1 | Whether v is a RegExp whose protocol lookups (exec, flags and the flag getters, constructor, Symbol.species, Symbol.match, lastIndex) are all original data, so that skipping them is unobservable (D1503). |
| 295 | `RegExpSplitRaw` | leaf | 4 | RegExp.prototype[Symbol.split] of a pristine RegExp on a non-empty string with 0 < lim: the result array. Suspendable like RegExpExecRaw (pass undefined as state). |
| 296 | `RegExpReplaceRaw` | leaf | 4 | RegExp.prototype[Symbol.replace] of a pristine global RegExp with a string replacement template: the result, and lastIndex 0. Suspendable like RegExpExecRaw. |
| 297 | `IsDenseArray` | leaf | 1 | Whether v is an Array whose elements 0 .. length-1 are all present in its dense store (no holes): reading them with Get then runs no user code. |
| 298 | `AllNumbers` | leaf | 1 | Whether every element of a dense array (no holes) is a Number. |
| 299 | `AllCodePoints` | leaf | 1 | Whether every element of a dense array is an integral Number in [0, 0x10FFFF]. |
| 300 | `InstallMethod` | leaf | 4 | Defines f as a method of the object at the dotted path `target` (from the global object): writable, configurable, not enumerable, with `name` = key and `length`. The key is a string or a symbol (`[Symbol.iterator]` paths). |
| 301 | `InstallConstructor` | leaf | 5 | `@constructor`: defines f as the global constructor `key` (writable, configurable, not enumerable) and sets its `name` and `length`; with a parent (a dotted path), f's [[Prototype]] is the parent constructor. Its statics follow, then InstallPrototype. |
| 302 | `InstallPrototype` | leaf | 3 | The `prototype` of the constructor `key`, installed after its statics (D2052, qjs's order): the realm's intrinsic prototype for that name if there is one, else a new object inheriting from the parent's prototype (or Object.prototype). Sets `f.prototype`. |
| 303 | `InstallPrototypeConstructor` | leaf | 1 | `prototype.constructor` of the constructor `key`, installed among the prototype's methods where qjs's order puts it (D2150). |
| 304 | `InstallAccessor` | leaf | 4 | `@getter`/`@setter`: one half of an accessor (configurable, not enumerable). |
| 305 | `InstallValue` | leaf | 4 | `@value`: a data property with `attrs` (writable 1, enumerable 2, configurable 4). |
| 306 | `InstallIntrinsic` | leaf | 2 | Registers f so that privileged code can call it as %name(...). |
| 307 | `MarkNative` | leaf | 2 | Makes the builtin f native builtin `index` of the generated table: with native builtins on, its calls run the Rust state machine. |
| 308 | `MarkFast` | leaf | 1 | Gives the builtin f its leaf fast path (`@fast`, D1850): with native builtins on, a call first tries the Rust function of the same name in builtins/fast.rs. |
| 309 | `ArraySpeciesCreate` | builtin (JS) | 2 |  |
| 310 | `ArrayConstructor` | builtin (JS) | 0 + rest |  |
| 311 | `ArraySpecies` | builtin (JS) | 0 |  |
| 312 | `FindViaPredicate` | builtin (JS) | 6 |  |
| 313 | `EveryOrSome` | builtin (JS) | 4 |  |
| 314 | `Reduce` | builtin (JS) | 4 |  |
| 315 | `SortList` | builtin (JS) | 3 |  |
| 316 | `CollectPresent` | builtin (JS) | 3 |  |
| 317 | `ArraySetLength` | builtin (JS) | 3 |  |
| 318 | `AsyncGeneratorStep` | builtin (JS) | 3 |  |
| 319 | `AsyncGeneratorAwaitReturnFulfilled` | builtin (JS) | 1 |  |
| 320 | `AsyncGeneratorAwaitReturnRejected` | builtin (JS) | 1 |  |
| 321 | `GetAsyncIterator` | builtin (JS) | 1 |  |
| 322 | `AsyncIteratorReturnMethod` | builtin (JS) | 1 |  |
| 323 | `AsyncIteratorCloseThrow` | builtin (JS) | 1 |  |
| 324 | `AsyncFromSyncIteratorUnwrap` | builtin (JS) | 1 |  |
| 325 | `AsyncFromSyncIteratorCloseThrow` | builtin (JS) | 1 |  |
| 326 | `ToBigInt` | builtin (JS) | 1 |  |
| 327 | `ToNumeric` | builtin (JS) | 1 |  |
| 328 | `ToPrimitive` | builtin (JS) | 2 |  |
| 329 | `ToNumber` | builtin (JS) | 1 |  |
| 330 | `ToString` | builtin (JS) | 1 |  |
| 331 | `LengthOfArrayLike` | builtin (JS) | 1 |  |
| 332 | `ToPropertyKey` | builtin (JS) | 1 |  |
| 333 | `GetPrototypeFromConstructor` | builtin (JS) | 2 |  |
| 334 | `DatePrototypeValueOf` | builtin (JS) | 0 |  |
| 335 | `DatePrototypeToISOString` | builtin (JS) | 0 |  |
| 336 | `DatePrototypeToJSON` | builtin (JS) | 1 |  |
| 337 | `DatePrototypeToPrimitive` | builtin (JS) | 1 |  |
| 338 | `DecoratorStart` | builtin (JS) | 2 |  |
| 339 | `ApplyDecorators` | builtin (JS) | 6 |  |
| 340 | `DecoratorRunInits` | builtin (JS) | 3 |  |
| 341 | `DecoratorRunExtras` | builtin (JS) | 3 |  |
| 342 | `DecoratorAddInitializer` | builtin (JS) | 1 |  |
| 343 | `DecoratorAccessGet` | builtin (JS) | 1 |  |
| 344 | `DecoratorAccessSet` | builtin (JS) | 2 |  |
| 345 | `DecoratorAccessHas` | builtin (JS) | 1 |  |
| 346 | `ApplyLegacyDecorators` | builtin (JS) | 4 |  |
| 347 | `DecoratorParam` | builtin (JS) | 2 |  |
| 348 | `DecoratorParamCall` | builtin (JS) | 2 |  |
| 349 | `DecoratorMetadata` | builtin (JS) | 2 |  |
| 350 | `AsyncDisposeFulfilled` | builtin (JS) | 1 |  |
| 351 | `AsyncDisposeRejected` | builtin (JS) | 1 |  |
| 352 | `UsingCreate` | builtin (JS) | 0 |  |
| 353 | `UsingAdd` | builtin (JS) | 2 |  |
| 354 | `AwaitUsingAdd` | builtin (JS) | 2 |  |
| 355 | `UsingDisposeError` | builtin (JS) | 2 |  |
| 356 | `UsingDispose` | builtin (JS) | 1 |  |
| 357 | `UsingDisposeStep` | builtin (JS) | 1 |  |
| 358 | `UsingAwaitValue` | builtin (JS) | 1 |  |
| 359 | `CallPrepareStackTrace` | builtin (JS) | 4 |  |
| 360 | `ThrowWithStack` | builtin (JS) | 1 |  |
| 361 | `MicrotaskJob` | builtin (JS) | 1 |  |
| 362 | `FromAsyncFulfilled` | builtin (JS) | 1 |  |
| 363 | `FromAsyncRejected` | builtin (JS) | 1 |  |
| 364 | `InstanceofOperator` | builtin (JS) | 2 |  |
| 365 | `ClassHeritage` | builtin (JS) | 1 |  |
| 366 | `ThrowTypeError` | builtin (JS) | 1 |  |
| 367 | `FunctionPrototypeBehavior` | builtin (JS) | 0 |  |
| 368 | `AsyncAwait` | builtin (JS) | 2 |  |
| 369 | `AsyncFunctionReturn` | builtin (JS) | 2 |  |
| 370 | `AsyncFunctionThrow` | builtin (JS) | 2 |  |
| 371 | `GeneratorFunctionConstructor` | builtin (JS) | 0 + rest |  |
| 372 | `AsyncGeneratorFunctionConstructor` | builtin (JS) | 0 + rest |  |
| 373 | `AsyncFunctionConstructor` | builtin (JS) | 0 + rest |  |
| 374 | `DynamicImport` | builtin (JS) | 2 |  |
| 375 | `GetIterator` | builtin (JS) | 1 |  |
| 376 | `IteratorClose` | builtin (JS) | 1 |  |
| 377 | `IteratorCloseThrow` | builtin (JS) | 1 |  |
| 378 | `IteratorRest` | builtin (JS) | 2 |  |
| 379 | `ArrayPrototypeValues` | builtin (JS) | 0 |  |
| 380 | `ArrayIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 381 | `StringPrototypeIterator` | builtin (JS) | 0 |  |
| 382 | `StringIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 383 | `SerializeJSONProperty` | builtin (JS) | 7 |  |
| 384 | `JSONStackIncludes` | builtin (JS) | 2 |  |
| 385 | `JSONJoin` | builtin (JS) | 7 |  |
| 386 | `SerializeJSONObject` | builtin (JS) | 6 |  |
| 387 | `SerializeJSONArray` | builtin (JS) | 6 |  |
| 388 | `AddEntriesFromIterable` | builtin (JS) | 3 |  |
| 389 | `MapPrototypeSet` | builtin (JS) | 2 |  |
| 390 | `MapIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 391 | `ResolvePromise` | builtin (JS) | 2 |  |
| 392 | `PromiseResolveFunction` | builtin (JS) | 1 |  |
| 393 | `PromiseRejectFunction` | builtin (JS) | 1 |  |
| 394 | `GetCapabilitiesExecutor` | builtin (JS) | 2 |  |
| 395 | `PromiseReactionJob` | builtin (JS) | 4 |  |
| 396 | `PromiseResolveThenableJob` | builtin (JS) | 3 |  |
| 397 | `PromiseConstructor` | builtin (JS) | 1 |  |
| 398 | `PromiseAllResolveElement` | builtin (JS) | 1 |  |
| 399 | `PromiseAllSettledResolveElement` | builtin (JS) | 1 |  |
| 400 | `PromiseAllSettledRejectElement` | builtin (JS) | 1 |  |
| 401 | `PromiseAnyRejectElement` | builtin (JS) | 1 |  |
| 402 | `PromiseThenFinally` | builtin (JS) | 1 |  |
| 403 | `PromiseCatchFinally` | builtin (JS) | 1 |  |
| 404 | `PromiseValueThunk` | builtin (JS) | 0 |  |
| 405 | `PromiseThrower` | builtin (JS) | 0 |  |
| 406 | `HostCallSettle` | builtin (JS) | 3 |  |
| 407 | `ProxyRevokeFunction` | builtin (JS) | 0 |  |
| 408 | `ProxyGetPrototypeOf` | builtin (JS) | 1 |  |
| 409 | `ProxySetPrototypeOf` | builtin (JS) | 2 |  |
| 410 | `ProxyIsExtensible` | builtin (JS) | 1 |  |
| 411 | `ProxyPreventExtensions` | builtin (JS) | 1 |  |
| 412 | `ProxyGetOwnPropertyDescriptorObject` | builtin (JS) | 2 |  |
| 413 | `ProxyHasOwnProperty` | builtin (JS) | 2 |  |
| 414 | `ProxyOwnPropertyAttributes` | builtin (JS) | 2 |  |
| 415 | `ProxyDefineOwnPropertyFromRecord` | builtin (JS) | 3 |  |
| 416 | `ProxyDefineDataPropertyOrThrow` | builtin (JS) | 4 |  |
| 417 | `ProxyCreateDataProperty` | builtin (JS) | 3 |  |
| 418 | `ProxyCreateDataPropertyOrThrow` | builtin (JS) | 3 |  |
| 419 | `ProxyHas` | builtin (JS) | 2 |  |
| 420 | `ProxyGet` | builtin (JS) | 3 |  |
| 421 | `ProxySet` | builtin (JS) | 4 |  |
| 422 | `ProxySetOrThrow` | builtin (JS) | 4 |  |
| 423 | `ProxySetOnReceiver` | builtin (JS) | 3 |  |
| 424 | `ProxyDeleteProperty` | builtin (JS) | 2 |  |
| 425 | `ProxyDeletePropertyOrThrow` | builtin (JS) | 2 |  |
| 426 | `ProxyOwnPropertyKeys` | builtin (JS) | 2 |  |
| 427 | `ProxyCall` | builtin (JS) | 3 |  |
| 428 | `ProxyConstruct` | builtin (JS) | 3 |  |
| 429 | `OrdinaryConstructSlow` | builtin (JS) | 3 |  |
| 430 | `ProxySetIntegrityLevel` | builtin (JS) | 2 |  |
| 431 | `ProxyTestIntegrityLevel` | builtin (JS) | 2 |  |
| 432 | `ForInStep` | builtin (JS) | 1 |  |
| 433 | `RegExpSpecies` | builtin (JS) | 0 |  |
| 434 | `RegExpPrototypeHasIndices` | builtin (JS) | 0 |  |
| 435 | `RegExpPrototypeGlobal` | builtin (JS) | 0 |  |
| 436 | `RegExpPrototypeIgnoreCase` | builtin (JS) | 0 |  |
| 437 | `RegExpPrototypeMultiline` | builtin (JS) | 0 |  |
| 438 | `RegExpPrototypeDotAll` | builtin (JS) | 0 |  |
| 439 | `RegExpPrototypeUnicode` | builtin (JS) | 0 |  |
| 440 | `RegExpPrototypeUnicodeSets` | builtin (JS) | 0 |  |
| 441 | `RegExpPrototypeSticky` | builtin (JS) | 0 |  |
| 442 | `RegExpPrototypeFlags` | builtin (JS) | 0 |  |
| 443 | `RegExpPrototypeExec` | builtin (JS) | 1 |  |
| 444 | `RegExpPrototypeMatch` | builtin (JS) | 1 |  |
| 445 | `SetPrototypeAdd` | builtin (JS) | 1 |  |
| 446 | `SetIteratorPrototypeNext` | builtin (JS) | 0 |  |
| 447 | `ArraySpread` | builtin (JS) | 2 |  |
| 448 | `CopyDataProperties` | builtin (JS) | 2 |  |
| 449 | `CopyDataPropertiesExcluding` | builtin (JS) | 2 |  |
| 450 | `TypedArraySetElementSlow` | builtin (JS) | 3 |  |
| 451 | `FinalizationRegistryCleanupJob` | builtin (JS) | 1 |  |
<!-- /gen -->
