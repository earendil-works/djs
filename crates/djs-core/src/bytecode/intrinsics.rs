//! The intrinsic id table: the immediate of `call_intrinsic id, argc` (docs/compiler.md
//! 7.3, D1270).
//!
//! The table is the contract between the compiler (privileged mode resolves `%Name` to
//! an id) and the interpreter (dispatches on the id). Its source of truth is
//! `builtins/intrinsics.d.ts` (the Rust intrinsics, in declaration order) followed by the
//! builtins tagged `@intrinsic` in `builtins/*.ts` (in file-name order, then declaration
//! order, the order of `builtins/gen/prelude.js`). Those are JS functions that the
//! prelude registers with `%InstallIntrinsic(name, fn)`. The `table_matches_sources` test
//! checks the table against those files and prints the expected Rust when they differ.
//!
//! Ids are indexes into [`INTRINSICS`]. They are part of the bytecode format, so the
//! table is hashed into [`INTRINSICS_HASH`] (combine it into the build hash with
//! `OPS_HASH`).

/// What runs for an intrinsic.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum IntrinsicKind {
    /// Rust that never runs user code (`@leaf`).
    Leaf,
    /// Rust that may call one user function whose result completes it (`@reentrant`,
    /// D1173).
    Reentrant,
    /// A JS builtin (`@intrinsic`), registered at prelude install time by
    /// `%InstallIntrinsic`; calling it is an ordinary JS call.
    Builtin,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IntrinsicInfo {
    pub name: &'static str,
    pub kind: IntrinsicKind,
    /// Declared parameters (without a rest parameter, without `this`).
    pub params: u8,
    /// Takes `...args` after the declared parameters.
    pub variadic: bool,
}

const fn i(name: &'static str, kind: IntrinsicKind, params: u8, variadic: bool) -> IntrinsicInfo {
    IntrinsicInfo {
        name,
        kind,
        params,
        variadic,
    }
}

use IntrinsicKind::{Builtin as B, Leaf as L, Reentrant as R};

/// All intrinsics; the id is the index.
pub const INTRINSICS: &[IntrinsicInfo] = &[
    i("IsObject", L, 1, false),
    i("IsCallable", L, 1, false),
    i("IsConstructor", L, 1, false),
    i("IsArray", L, 1, false),
    i("IsString", L, 1, false),
    i("IsNumber", L, 1, false),
    i("IsSymbol", L, 1, false),
    i("IsBoolean", L, 1, false),
    i("AsObj", L, 1, false),
    i("AsFn", L, 1, false),
    i("AsStr", L, 1, false),
    i("AsNumber", L, 1, false),
    i("AsSym", L, 1, false),
    i("AsBool", L, 1, false),
    i("ToBoolean", L, 1, false),
    i("ToNumberPrimitive", L, 1, false),
    i("ToStringPrimitive", L, 1, false),
    i("ToObject", L, 1, false),
    i("ToIntegerOrInfinityNumber", L, 1, false),
    i("F64ToU32", L, 1, false),
    i("F64ToI32", L, 1, false),
    i("StrLength", L, 1, false),
    i("Get", R, 2, false),
    i("GetV", R, 2, false),
    i("Set", R, 3, false),
    i("HasProperty", R, 2, false),
    i("CreateDataPropertyOrThrow", R, 3, false),
    i("DeletePropertyOrThrow", R, 2, false),
    i("Call", R, 2, true),
    i("Construct", R, 1, true),
    i("ObjectCreate", L, 1, false),
    i("ArrayCreate", L, 1, false),
    i("NewTypeError", L, 1, false),
    i("NewNotConstructorError", L, 1, false),
    i("NewRangeError", L, 1, false),
    i("WellKnownSymbol", L, 1, false),
    i("RealmIntrinsic", L, 1, false),
    i("ErrorCreate", L, 1, false),
    i("CaptureStack", L, 1, false),
    i("DefineDataPropertyOrThrow", L, 4, false),
    i("DefineOwnPropertyFromRecord", L, 3, false),
    i("GetOwnPropertyDescriptorObject", L, 2, false),
    i("HasOwnProperty", L, 2, false),
    i("OwnPropertyAttributes", L, 2, false),
    i("OwnPropertyKeys", L, 2, false),
    i("GetPrototypeOf", L, 1, false),
    i("SetPrototypeOf", L, 2, false),
    i("PreventExtensions", L, 1, false),
    i("IsExtensible", L, 1, false),
    i("SetIntegrityLevel", L, 2, false),
    i("TestIntegrityLevel", L, 2, false),
    i("SameValue", L, 2, false),
    i("ObjectClassTag", L, 1, false),
    i("WrapPrimitive", L, 2, false),
    i("PrimitiveValue", L, 1, false),
    i("CallWithArray", R, 3, false),
    i("BoundFunctionCreate", L, 3, false),
    i("BoundTargetFunction", L, 1, false),
    i("FunctionToString", L, 1, false),
    i("NumberPow", L, 2, false),
    i("SymbolDescriptiveString", L, 1, false),
    i("CompareStrings", L, 2, false),
    i("StringCodePoints", L, 1, false),
    i("SortPrimitives", L, 1, false),
    i("ArraySpliceDense", L, 4, false),
    i("StrLocaleCompare", L, 2, false),
    i("StrCharCodeAt", L, 2, false),
    i("StrCodePointAt", L, 2, false),
    i("StrSubstring", L, 3, false),
    i("StrIndexOf", L, 3, false),
    i("StrLastIndexOf", L, 3, false),
    i("StrToCase", L, 2, false),
    i("StrTrim", L, 2, false),
    i("StrIsWellFormed", L, 1, false),
    i("StrToWellFormed", L, 1, false),
    i("StrRepeat", L, 2, false),
    i("StrFromCodeUnits", L, 1, false),
    i("StrFromCodePoints", L, 1, false),
    i("StrNormalize", L, 2, false),
    i("NewSymbol", L, 1, false),
    i("SymbolDescription", L, 1, false),
    i("SymbolFor", L, 1, false),
    i("SymbolKeyFor", L, 1, false),
    i("NumberToString", L, 2, false),
    i("NumberToFixed", L, 2, false),
    i("NumberToExponential", L, 2, false),
    i("NumberToPrecision", L, 2, false),
    i("StringParseFloat", L, 1, false),
    i("StringParseInt", L, 2, false),
    i("NumberToInt32", L, 1, false),
    i("NumberToUint32", L, 1, false),
    i("F64Abs", L, 1, false),
    i("F64Acos", L, 1, false),
    i("F64Acosh", L, 1, false),
    i("F64Asin", L, 1, false),
    i("F64Asinh", L, 1, false),
    i("F64Atan", L, 1, false),
    i("F64Atanh", L, 1, false),
    i("F64Atan2", L, 2, false),
    i("F64Cbrt", L, 1, false),
    i("F64Hypot", L, 2, false),
    i("F64Ceil", L, 1, false),
    i("F64Clz32", L, 1, false),
    i("F64Cos", L, 1, false),
    i("F64Cosh", L, 1, false),
    i("F64Exp", L, 1, false),
    i("F64Expm1", L, 1, false),
    i("F64Floor", L, 1, false),
    i("F64Fround", L, 1, false),
    i("F64Imul", L, 2, false),
    i("F64Log", L, 1, false),
    i("F64Log1p", L, 1, false),
    i("F64Log10", L, 1, false),
    i("F64Log2", L, 1, false),
    i("F64Round", L, 1, false),
    i("F64Sign", L, 1, false),
    i("F64Sin", L, 1, false),
    i("F64Sinh", L, 1, false),
    i("F64Sqrt", L, 1, false),
    i("F64Tan", L, 1, false),
    i("F64Tanh", L, 1, false),
    i("F64Trunc", L, 1, false),
    i("MathRandom", L, 0, false),
    i("InstallMethod", L, 4, false),
    i("InstallConstructor", L, 5, false),
    i("InstallAccessor", L, 4, false),
    i("InstallValue", L, 4, false),
    i("InstallIntrinsic", L, 2, false),
    i("ToPrimitive", B, 2, false),
    i("ToNumber", B, 1, false),
    i("ToString", B, 1, false),
    i("InstanceofOperator", B, 2, false),
];

/// The id of the intrinsic `name`.
pub fn intrinsic_id(name: &str) -> Option<u16> {
    INTRINSICS
        .iter()
        .position(|i| i.name == name)
        .map(|i| i as u16)
}

/// The intrinsic with id `id`.
pub fn intrinsic(id: u16) -> Option<&'static IntrinsicInfo> {
    INTRINSICS.get(id as usize)
}

/// FNV-1a over the table (names, kinds, arities, order).
pub const INTRINSICS_HASH: u32 = {
    let mut h: u32 = 0x811c_9dc5;
    let mut k = 0;
    while k < INTRINSICS.len() {
        let e = &INTRINSICS[k];
        let n = e.name.as_bytes();
        let mut j = 0;
        while j < n.len() {
            h = (h ^ n[j] as u32).wrapping_mul(0x0100_0193);
            j += 1;
        }
        let tail = [0u8, e.kind as u8, e.params, e.variadic as u8];
        let mut j = 0;
        while j < tail.len() {
            h = (h ^ tail[j] as u32).wrapping_mul(0x0100_0193);
            j += 1;
        }
        k += 1;
    }
    h
};

const _: () = assert!(INTRINSICS.len() <= u16::MAX as usize);

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::format;
    use alloc::string::String;
    use alloc::vec::Vec;
    use std::path::PathBuf;

    use super::*;

    fn builtins_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../builtins")
    }

    /// `(name, params, variadic)` of a parameter list text (without parentheses).
    fn arity(params: &str) -> (u8, bool) {
        let mut n = 0;
        let mut variadic = false;
        for p in params.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if p.starts_with("...") {
                variadic = true;
            } else if !p.starts_with("this:") && !p.starts_with("this :") {
                n += 1;
            }
        }
        (n, variadic)
    }

    /// The JSDoc comment right before `at` (only whitespace in between), if any.
    fn doc_before(src: &str, at: usize) -> &str {
        let before = src[..at].trim_end();
        if !before.ends_with("*/") {
            return "";
        }
        match before.rfind("/**") {
            Some(s) => &before[s..],
            None => "",
        }
    }

    /// The entries the sources describe.
    fn from_sources() -> Vec<IntrinsicInfo> {
        let mut out = Vec::new();
        let dts = std::fs::read_to_string(builtins_dir().join("intrinsics.d.ts")).unwrap();
        for (at, _) in dts.match_indices("declare function ") {
            let rest = &dts[at + "declare function ".len()..];
            let name = &rest[..rest.find('(').unwrap()];
            let params = &rest[name.len() + 1..rest.find(')').unwrap()];
            let doc = doc_before(&dts, at);
            let kind = if doc.contains("@reentrant") {
                IntrinsicKind::Reentrant
            } else {
                assert!(doc.contains("@leaf"), "{name}: no @leaf/@reentrant tag");
                IntrinsicKind::Leaf
            };
            let (n, variadic) = arity(params);
            out.push(IntrinsicInfo {
                name: std::boxed::Box::leak(String::from(name).into_boxed_str()),
                kind,
                params: n,
                variadic,
            });
        }
        let mut files: Vec<PathBuf> = std::fs::read_dir(builtins_dir())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.extension().is_some_and(|e| e == "ts") && !p.to_string_lossy().ends_with(".d.ts")
            })
            .collect();
        files.sort();
        for f in files {
            let src = std::fs::read_to_string(&f).unwrap();
            for (at, _) in src.match_indices("\nfunction ") {
                let at = at + 1;
                if !doc_before(&src, at).contains("@intrinsic") {
                    continue;
                }
                let rest = &src[at + "function ".len()..];
                let name = &rest[..rest.find('(').unwrap()];
                let params = &rest[name.len() + 1..rest.find(')').unwrap()];
                let (n, variadic) = arity(params);
                out.push(IntrinsicInfo {
                    name: std::boxed::Box::leak(String::from(name).into_boxed_str()),
                    kind: IntrinsicKind::Builtin,
                    params: n,
                    variadic,
                });
            }
        }
        out
    }

    #[test]
    fn table_matches_sources() {
        let expected = from_sources();
        if expected.as_slice() != INTRINSICS {
            let mut rust = String::new();
            for e in &expected {
                let k = match e.kind {
                    IntrinsicKind::Leaf => "L",
                    IntrinsicKind::Reentrant => "R",
                    IntrinsicKind::Builtin => "B",
                };
                rust.push_str(&format!(
                    "    i({:?}, {k}, {}, {}),\n",
                    e.name, e.params, e.variadic
                ));
            }
            panic!("INTRINSICS is out of date with builtins/; replace its entries with:\n{rust}");
        }
    }

    #[test]
    fn lookup() {
        assert_eq!(intrinsic_id("IsObject"), Some(0));
        let get = intrinsic_id("Get").unwrap();
        assert_eq!(intrinsic(get).unwrap().kind, IntrinsicKind::Reentrant);
        assert_eq!(intrinsic_id("Nope"), None);
        assert_eq!(
            intrinsic(intrinsic_id("ToPrimitive").unwrap())
                .unwrap()
                .kind,
            IntrinsicKind::Builtin
        );
        assert_ne!(INTRINSICS_HASH, 0);
    }
}
