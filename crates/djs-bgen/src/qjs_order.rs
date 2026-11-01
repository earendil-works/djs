//! qjs's order of the own properties of constructors and namespace objects (0108,
//! D2072): the order of its function lists, which `Object.getOwnPropertyNames` shows.
//! The builtins are spread over files by topic, so source order cannot give it. Only
//! the objects whose order differs from source order are listed; a key not in its list
//! keeps source order after the listed ones.

const ORDER: &[(&str, &[&str])] = &[
    ("Array", &["isArray", "from", "fromAsync", "of"]),
    ("BigInt", &["asUintN", "asIntN"]),
    ("Iterator", &["concat", "from", "zip", "zipKeyed"]),
    (
        "Number",
        &[
            "parseInt",
            "parseFloat",
            "isNaN",
            "isFinite",
            "isInteger",
            "isSafeInteger",
            "MAX_VALUE",
            "MIN_VALUE",
            "NaN",
            "NEGATIVE_INFINITY",
            "POSITIVE_INFINITY",
            "EPSILON",
            "MAX_SAFE_INTEGER",
            "MIN_SAFE_INTEGER",
        ],
    ),
    (
        "Object",
        &[
            "create",
            "getPrototypeOf",
            "setPrototypeOf",
            "defineProperty",
            "defineProperties",
            "getOwnPropertyNames",
            "getOwnPropertySymbols",
            "groupBy",
            "keys",
            "values",
            "entries",
            "isExtensible",
            "preventExtensions",
            "getOwnPropertyDescriptor",
            "getOwnPropertyDescriptors",
            "is",
            "assign",
            "seal",
            "freeze",
            "isSealed",
            "isFrozen",
            "fromEntries",
            "hasOwn",
        ],
    ),
    (
        "Promise",
        &[
            "resolve",
            "reject",
            "all",
            "allSettled",
            "allKeyed",
            "allSettledKeyed",
            "any",
            "try",
            "race",
            "withResolvers",
        ],
    ),
    (
        "Symbol",
        &[
            "for",
            "keyFor",
            "toPrimitive",
            "iterator",
            "match",
            "matchAll",
            "replace",
            "search",
            "split",
            "toStringTag",
            "isConcatSpreadable",
            "hasInstance",
            "species",
            "unscopables",
            "asyncIterator",
            "dispose",
            "asyncDispose",
            "metadata",
        ],
    ),
    (
        "Uint8Array",
        &["BYTES_PER_ELEMENT", "fromBase64", "fromHex"],
    ),
    (
        "Math",
        &[
            "min",
            "max",
            "abs",
            "floor",
            "ceil",
            "round",
            "sqrt",
            "acos",
            "asin",
            "atan",
            "atan2",
            "cos",
            "exp",
            "log",
            "pow",
            "sin",
            "tan",
            "trunc",
            "sign",
            "cosh",
            "sinh",
            "tanh",
            "acosh",
            "asinh",
            "atanh",
            "expm1",
            "log1p",
            "log2",
            "log10",
            "cbrt",
            "hypot",
            "random",
            "f16round",
            "fround",
            "imul",
            "clz32",
            "sumPrecise",
            "E",
            "LN10",
            "LN2",
            "LOG2E",
            "LOG10E",
            "PI",
            "SQRT1_2",
            "SQRT2",
        ],
    ),
    ("JSON", &["isRawJSON", "parse", "rawJSON", "stringify"]),
];

/// The rank of `key` among the own properties of the object at `target` (unlisted keys
/// after the listed ones).
fn rank(target: &str, key: &str) -> usize {
    let keys = ORDER
        .iter()
        .find(|(t, _)| *t == target)
        .map_or(&[][..], |(_, k)| *k);
    keys.iter().position(|k| *k == key).unwrap_or(keys.len())
}

/// Reorders `items` (installs as `(target, key, line)`) so that the installs onto each
/// listed target come in qjs's order, in the slots they took; everything else stays.
pub fn reorder(items: &mut [(String, String, String)]) {
    for (target, _) in ORDER {
        let slots: Vec<usize> = (0..items.len())
            .filter(|&i| items[i].0 == *target)
            .collect();
        let mut order = slots.clone();
        // Stable: unlisted keys keep source order after the listed ones.
        order.sort_by_key(|&i| rank(target, &items[i].1));
        let sorted: Vec<_> = order.iter().map(|&i| items[i].clone()).collect();
        for (slot, item) in slots.into_iter().zip(sorted) {
            items[slot] = item;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::reorder;

    #[test]
    fn listed_targets_take_qjs_order_in_place() {
        let item = |t: &str, k: &str| (t.to_string(), k.to_string(), format!("{t}.{k}"));
        let mut v = vec![
            item("Array", "of"),
            item("Foo", "x"),
            item("Array", "extra"),
            item("Array", "isArray"),
            item("Array", "from"),
        ];
        reorder(&mut v);
        let lines: Vec<_> = v.iter().map(|i| i.2.as_str()).collect();
        assert_eq!(
            lines,
            [
                "Array.isArray",
                "Foo.x",
                "Array.from",
                "Array.of",
                "Array.extra"
            ]
        );
    }
}
