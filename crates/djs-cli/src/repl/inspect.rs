//! The REPL's value printer: reads the heap directly (no JS runs, so no getter, Proxy
//! trap or `toString` can interfere) and prints in the style of node's `util.inspect`.
//! It need not match qjs or node (D1100: the codemode console is the prelude's).

use djs_core::bytecode::template::func_flags;
use djs_core::heap::cells::FuncTemplate;
use djs_core::heap::typed::{self, Elem};
use djs_core::heap::{Atom, Heap, ObjectClass, Property, atoms};
use djs_core::interp::{Intrinsic, error_summary, intrinsic};
use djs_core::value::{Kind, Obj, Value};

/// Nested objects deeper than this print as `[Object]`, `[Array]`, ...
const MAX_DEPTH: usize = 3;
/// Elements shown of an array, typed array, Map or Set.
const MAX_ITEMS: usize = 100;
/// A composite value prints on one line when it fits in this many columns.
const LINE_WIDTH: usize = 72;

/// The value as the REPL prints a result: strings quoted.
pub fn inspect(heap: &Heap, v: Value) -> String {
    Printer::new(heap).top(v)
}

/// The value as `console.log` prints an argument: strings raw, the rest as `inspect`.
pub fn log_arg(heap: &Heap, v: Value) -> String {
    match v.as_string() {
        Some(s) => heap.str_to_string(s),
        None => inspect(heap, v),
    }
}

struct Printer<'h> {
    heap: &'h Heap,
    /// The objects being printed, outermost first (cycle detection).
    path: Vec<Obj>,
    /// Objects found in a cycle, with their `*n` numbers.
    refs: Vec<(Obj, usize)>,
}

impl<'h> Printer<'h> {
    fn new(heap: &'h Heap) -> Self {
        Printer {
            heap,
            path: Vec::new(),
            refs: Vec::new(),
        }
    }

    fn top(&mut self, v: Value) -> String {
        // A top-level error shows its stack, like node.
        if let Some(o) = v.as_object()
            && self.heap.object_class(o) == ObjectClass::Error
        {
            let mut s = error_summary(self.heap, v).unwrap_or_default();
            if let Some(stack) = self.own_string(o, atoms::STACK) {
                s.push('\n');
                s.push_str(stack.trim_end());
            }
            let extra = self.props(o, 0, 0, &[atoms::STACK, atoms::MESSAGE]);
            if !extra.is_empty() {
                s.push(' ');
                s.push_str(&self.wrap("{", "}", &extra, 0));
            }
            return s;
        }
        self.value(v, 0, 0)
    }

    /// `v` printed at nesting `depth`, starting at column `indent`.
    fn value(&mut self, v: Value, depth: usize, indent: usize) -> String {
        match v.kind() {
            Kind::Undefined => "undefined".into(),
            Kind::Null => "null".into(),
            Kind::Bool => v.as_bool().unwrap_or(false).to_string(),
            Kind::Number => number(v.as_number().unwrap_or(f64::NAN)),
            Kind::String => quote(&self.heap.str_to_string(v.as_string().unwrap())),
            Kind::Symbol => self.symbol(v),
            Kind::BigInt => format!("{}n", self.heap.bigint(v).to_string_radix(10)),
            Kind::Object => self.object(v.as_object().unwrap(), depth, indent),
            Kind::Hole => "<hole>".into(),
            Kind::Uninitialized => "<uninitialized>".into(),
            Kind::HostRef => "<host ref>".into(),
            Kind::Internal => "<internal>".into(),
        }
    }

    fn symbol(&self, v: Value) -> String {
        let d = self.heap.symbol_description(v.as_symbol().unwrap());
        match d.as_string() {
            Some(s) => format!("Symbol({})", self.heap.str_to_string(s)),
            None => "Symbol()".into(),
        }
    }

    fn object(&mut self, o: Obj, depth: usize, indent: usize) -> String {
        if let Some(i) = self.path.iter().position(|&p| p == o) {
            let target = self.path[i];
            let n = match self.refs.iter().find(|r| r.0 == target) {
                Some(&(_, n)) => n,
                None => {
                    let n = self.refs.len() + 1;
                    self.refs.push((target, n));
                    n
                }
            };
            return format!("[Circular *{n}]");
        }
        self.path.push(o);
        let s = self.object_body(o, depth, indent);
        self.path.pop();
        match self.refs.iter().find(|r| r.0 == o) {
            Some(&(_, n)) => format!("<ref *{n}> {s}"),
            None => s,
        }
    }

    fn object_body(&mut self, o: Obj, depth: usize, indent: usize) -> String {
        let heap = self.heap;
        let class = heap.object_class(o);
        if heap.is_callable(o) && class != ObjectClass::Proxy {
            let head = self.function_head(o);
            let props = self.props(o, depth, indent, &[]);
            return if props.is_empty() || depth > MAX_DEPTH {
                head
            } else {
                format!("{head} {}", self.wrap("{", "}", &props, indent))
            };
        }
        if depth > MAX_DEPTH {
            return match class {
                ObjectClass::Array => "[Array]".into(),
                _ => format!("[{}]", self.constructor_name(o).unwrap_or("Object".into())),
            };
        }
        let d = depth + 1;
        let ind = indent + 2;
        match class {
            ObjectClass::Array => {
                let mut items = self.elements(o, d, ind);
                items.extend(self.props(o, d, ind, &[]));
                let prefix = self.prefix(o, "Array", false);
                format!("{prefix}{}", self.wrap("[", "]", &items, indent))
            }
            ObjectClass::Error => {
                let summary = error_summary(heap, Value::object(o)).unwrap_or_default();
                let props = self.props(o, d, ind, &[atoms::STACK, atoms::MESSAGE]);
                if props.is_empty() {
                    format!("[{summary}]")
                } else {
                    format!("[{summary}] {}", self.wrap("{", "}", &props, indent))
                }
            }
            ObjectClass::Map | ObjectClass::Set => {
                let is_map = class == ObjectClass::Map;
                let entries = heap.collection_entries_list(o);
                let mut items = Vec::new();
                for (k, v) in entries.iter().take(MAX_ITEMS) {
                    let k = self.value(*k, d, ind);
                    items.push(if is_map {
                        format!("{k} => {}", self.value(*v, d, ind))
                    } else {
                        k
                    });
                }
                if entries.len() > MAX_ITEMS {
                    items.push(more(entries.len() - MAX_ITEMS));
                }
                items.extend(self.props(o, d, ind, &[]));
                let name = if is_map { "Map" } else { "Set" };
                let prefix = self.prefix(o, &format!("{name}({})", entries.len()), true);
                format!("{prefix}{}", self.wrap("{", "}", &items, indent))
            }
            ObjectClass::WeakMap | ObjectClass::WeakSet => {
                let name = if class == ObjectClass::WeakMap {
                    "WeakMap"
                } else {
                    "WeakSet"
                };
                format!(
                    "{} {{ <items unknown> }}",
                    self.prefix(o, name, true).trim_end()
                )
            }
            ObjectClass::Promise => {
                let state = heap.object_slot(o, 0).as_int().unwrap_or(0);
                let result = heap.object_slot(o, 1);
                let mut items = vec![match state {
                    0 => "<pending>".into(),
                    1 => self.value(result, d, ind),
                    _ => format!("<rejected> {}", self.value(result, d, ind)),
                }];
                items.extend(self.props(o, d, ind, &[]));
                let prefix = self.prefix(o, "Promise", true);
                format!("{prefix}{}", self.wrap("{", "}", &items, indent))
            }
            ObjectClass::TypedArray => {
                let kind = heap.ta_kind(o);
                let len = heap.ta_length(o);
                let mut items = Vec::new();
                for i in 0..len.unwrap_or(0).min(MAX_ITEMS as u32) {
                    items.push(match heap.ta_elem(o, i) {
                        Some(Elem::Number(n)) => number(n),
                        Some(Elem::Big(bits)) => {
                            format!("{}n", typed::elem_bigint(kind, bits).to_string_radix(10))
                        }
                        None => "<detached>".into(),
                    });
                }
                if let Some(n) = len.filter(|&n| n as usize > MAX_ITEMS) {
                    items.push(more(n as usize - MAX_ITEMS));
                }
                items.extend(self.props(o, d, ind, &[]));
                let head = match len {
                    Some(n) => format!("{}({n})", kind.name()),
                    None => format!("{} <detached>", kind.name()),
                };
                let prefix = self.prefix(o, &head, true);
                format!("{prefix}{}", self.wrap("[", "]", &items, indent))
            }
            ObjectClass::ArrayBuffer => {
                let len = heap.buffer_len(o);
                let data = heap.buffer_data(o);
                let mut hex: Vec<String> =
                    data.iter().take(50).map(|b| format!("{b:02x}")).collect();
                if data.len() > 50 {
                    hex.push(format!("... {} more bytes", data.len() - 50));
                }
                let mut items = vec![
                    format!("[Uint8Contents]: <{}>", hex.join(" ")),
                    format!("byteLength: {len}"),
                ];
                items.extend(self.props(o, d, ind, &[]));
                let prefix = self.prefix(o, "ArrayBuffer", true);
                format!("{prefix}{}", self.wrap("{", "}", &items, indent))
            }
            ObjectClass::Date => {
                let t = heap.object_slot(o, 0).as_number().unwrap_or(f64::NAN);
                self.with_props(o, iso_date(t), d, indent)
            }
            ObjectClass::RegExp => {
                let src = heap
                    .object_slot(o, 1)
                    .as_string()
                    .map(|s| heap.str_to_string(s))
                    .unwrap_or_default();
                let bits = heap.object_slot(o, 2).as_int().unwrap_or(0);
                let flags: String = "dgimsuvy"
                    .chars()
                    .enumerate()
                    .filter(|&(i, _)| bits & (1 << i) != 0)
                    .map(|(_, c)| c)
                    .collect();
                let props = self.props(o, d, ind, &[atoms::LAST_INDEX]);
                self.join_props(format!("/{src}/{flags}"), props, indent)
            }
            ObjectClass::Boolean
            | ObjectClass::Number
            | ObjectClass::String
            | ObjectClass::Symbol
            | ObjectClass::BigInt => {
                let name = match class {
                    ObjectClass::Boolean => "Boolean",
                    ObjectClass::Number => "Number",
                    ObjectClass::String => "String",
                    ObjectClass::Symbol => "Symbol",
                    _ => "BigInt",
                };
                let inner = self.value(heap.object_slot(o, 0), d, ind);
                let mut props = self.props(o, d, ind, &[]);
                if class == ObjectClass::String {
                    // The index properties and `length` are the string itself.
                    props.retain(|p| !p.starts_with(|c: char| c.is_ascii_digit()));
                }
                self.join_props(format!("[{name}: {inner}]"), props, indent)
            }
            ObjectClass::Proxy => {
                let t = heap
                    .proxy_target(o)
                    .map(Value::object)
                    .unwrap_or(Value::NULL);
                let h = heap
                    .proxy_handler(o)
                    .map(Value::object)
                    .unwrap_or(Value::NULL);
                let items = vec![self.value(t, d, ind), self.value(h, d, ind)];
                format!("Proxy {}", self.wrap("[", "]", &items, indent))
            }
            ObjectClass::WeakRef => {
                let t = heap.weakref_target(o).unwrap_or(Value::UNDEFINED);
                let items = vec![self.value(t, d, ind)];
                format!(
                    "{}{}",
                    self.prefix(o, "WeakRef", true),
                    self.wrap("{", "}", &items, indent)
                )
            }
            ObjectClass::ArrayIterator
            | ObjectClass::CollectionIterator
            | ObjectClass::RegExpStringIterator
            | ObjectClass::Generator
            | ObjectClass::IteratorHelper => {
                let tag = self
                    .to_string_tag(o)
                    .unwrap_or_else(|| format!("{class:?}"));
                let props = self.props(o, d, ind, &[]);
                format!("Object [{tag}] {}", self.wrap("{", "}", &props, indent))
            }
            ObjectClass::Arguments => {
                let mut items = self.elements(o, d, ind);
                items.extend(self.props(o, d, ind, &[]));
                format!("[Arguments] {}", self.wrap("[", "]", &items, indent))
            }
            _ => {
                let props = self.props(o, d, ind, &[]);
                let fallback = match class {
                    ObjectClass::Ordinary => String::new(),
                    c => format!("{c:?}"),
                };
                let prefix = self.prefix(o, &fallback, true);
                format!("{prefix}{}", self.wrap("{", "}", &props, indent))
            }
        }
    }

    /// The text before the brackets: the class name, `[Object: null prototype]`, or the
    /// builtin's own name when its prototype is the usual one.
    fn prefix(&self, o: Obj, builtin: &str, always: bool) -> String {
        let heap = self.heap;
        let Some(proto) = heap.get_prototype_of(o) else {
            let base = if builtin.is_empty() {
                "Object"
            } else {
                builtin
            };
            return format!("[{base}: null prototype] ");
        };
        let name = self.constructor_name_of_proto(proto);
        let plain = proto
            == intrinsic(heap, Intrinsic::ObjectProto)
                .as_object()
                .expect("Object.prototype")
            || proto
                == intrinsic(heap, Intrinsic::ArrayProto)
                    .as_object()
                    .expect("Array.prototype");
        let base_name = builtin.split('(').next().unwrap_or("");
        match name {
            Some(n) if !plain && n != base_name => {
                if builtin.is_empty() {
                    format!("{n} ")
                } else {
                    format!("{n} [{builtin}] ")
                }
            }
            _ if always && !builtin.is_empty() => format!("{builtin} "),
            _ => String::new(),
        }
    }

    fn constructor_name(&self, o: Obj) -> Option<String> {
        self.constructor_name_of_proto(self.heap.get_prototype_of(o)?)
    }

    fn constructor_name_of_proto(&self, proto: Obj) -> Option<String> {
        let (_, p) = self.heap.find_property(proto, atoms::CONSTRUCTOR)?;
        let c = p.value()?.as_object()?;
        self.own_string(c, atoms::NAME).filter(|n| !n.is_empty())
    }

    fn to_string_tag(&self, o: Obj) -> Option<String> {
        let (_, p) = self.heap.find_property(o, atoms::SYM_TO_STRING_TAG)?;
        let s = p.value()?.as_string()?;
        Some(self.heap.str_to_string(s))
    }

    fn own_string(&self, o: Obj, key: Atom) -> Option<String> {
        let v = self.heap.get_own_property(o, key)?.value()?;
        Some(self.heap.str_to_string(v.as_string()?))
    }

    fn function_head(&self, o: Obj) -> String {
        let heap = self.heap;
        let name = self.own_string(o, atoms::NAME).unwrap_or_default();
        let mut kind = "Function";
        if heap.object_class(o) == ObjectClass::Function
            && let Some(t) = heap.object_slot(o, 0).as_internal()
        {
            let flags: u32 = heap.arena().get(t, FuncTemplate::FLAGS);
            match func_flags::kind(flags) {
                func_flags::KIND_CLASS_CTOR | func_flags::KIND_DERIVED_CTOR => {
                    let mut s = String::from("[class ");
                    s.push_str(if name.is_empty() {
                        "(anonymous)"
                    } else {
                        &name
                    });
                    let fproto = intrinsic(heap, Intrinsic::FunctionProto).as_object();
                    if let Some(parent) = heap.get_prototype_of(o)
                        && Some(parent) != fproto
                    {
                        let pname = self.own_string(parent, atoms::NAME).unwrap_or_default();
                        s.push_str(" extends ");
                        s.push_str(if pname.is_empty() {
                            "(anonymous)"
                        } else {
                            &pname
                        });
                    }
                    s.push(']');
                    return s;
                }
                _ => {
                    kind = match (
                        flags & func_flags::ASYNC != 0,
                        flags & func_flags::GENERATOR != 0,
                    ) {
                        (true, true) => "AsyncGeneratorFunction",
                        (true, false) => "AsyncFunction",
                        (false, true) => "GeneratorFunction",
                        _ => "Function",
                    };
                }
            }
        }
        if name.is_empty() {
            format!("[{kind} (anonymous)]")
        } else {
            format!("[{kind}: {name}]")
        }
    }

    /// The elements of an array-like, with holes counted as node does.
    fn elements(&mut self, o: Obj, depth: usize, indent: usize) -> Vec<String> {
        let heap = self.heap;
        let len = if heap.object_class(o) == ObjectClass::Array {
            heap.array_length(o)
        } else {
            heap.dense_len(o)
        };
        let mut items = Vec::new();
        let mut holes = 0;
        let flush = |items: &mut Vec<String>, holes: &mut u32| {
            if *holes > 0 {
                let s = if *holes == 1 { "" } else { "s" };
                items.push(format!("<{holes} empty item{s}>"));
                *holes = 0;
            }
        };
        let mut shown = 0;
        for i in 0..len {
            if shown >= MAX_ITEMS {
                flush(&mut items, &mut holes);
                items.push(more((len - i) as usize));
                break;
            }
            let v = heap.get_element_fast(o, i).or_else(|| {
                let p = heap.get_own_property(o, Atom::int(i)?)?;
                Some(match p {
                    Property::Data { value, .. } => value,
                    Property::Accessor { .. } => Value::HOLE,
                })
            });
            match v {
                None => holes += 1,
                Some(v) if v.is_hole() => {
                    flush(&mut items, &mut holes);
                    items.push(accessor_label(
                        heap.get_own_property(o, Atom::int(i).unwrap()),
                    ));
                    shown += 1;
                }
                Some(v) => {
                    flush(&mut items, &mut holes);
                    items.push(self.value(v, depth, indent));
                    shown += 1;
                }
            }
        }
        flush(&mut items, &mut holes);
        items
    }

    /// The enumerable own properties as `key: value`, except the array indexes and
    /// `skip`. Accessors are not called.
    fn props(&mut self, o: Obj, depth: usize, indent: usize, skip: &[Atom]) -> Vec<String> {
        let heap = self.heap;
        let class = heap.object_class(o);
        let indexed = matches!(
            class,
            ObjectClass::Array | ObjectClass::Arguments | ObjectClass::TypedArray
        );
        let mut out = Vec::new();
        for key in heap.own_keys(o) {
            if skip.contains(&key) || (indexed && heap.atom_array_index(key).is_some()) {
                continue;
            }
            let Some(p) = heap.get_own_property(o, key) else {
                continue;
            };
            if !p.flags().enumerable() {
                continue;
            }
            let k = self.key(key);
            let v = match p {
                Property::Data { value, .. } => self.value(value, depth, indent + k.len() + 2),
                Property::Accessor { .. } => accessor_label(Some(p)),
            };
            out.push(format!("{k}: {v}"));
        }
        out
    }

    fn key(&self, key: Atom) -> String {
        if let Some(sym) = self.heap.atom_symbol(key) {
            return format!("[{}]", self.symbol(Value::symbol(sym)));
        }
        let name = self.heap.atom_name(key);
        if is_identifier(&name) || name.parse::<u32>().is_ok() {
            name
        } else {
            quote(&name)
        }
    }

    fn with_props(&mut self, o: Obj, head: String, depth: usize, indent: usize) -> String {
        let props = self.props(o, depth, indent + 2, &[]);
        self.join_props(head, props, indent)
    }

    fn join_props(&self, head: String, props: Vec<String>, indent: usize) -> String {
        if props.is_empty() {
            head
        } else {
            format!("{head} {}", self.wrap("{", "}", &props, indent))
        }
    }

    /// `open items close` on one line if it fits, else one item per line.
    fn wrap(&self, open: &str, close: &str, items: &[String], indent: usize) -> String {
        if items.is_empty() {
            return format!("{open}{close}");
        }
        let width: usize = items.iter().map(|s| s.len() + 2).sum::<usize>() + 2;
        if indent + width <= LINE_WIDTH && !items.iter().any(|s| s.contains('\n')) {
            return format!("{open} {} {close}", items.join(", "));
        }
        let pad = " ".repeat(indent + 2);
        let mut s = String::from(open);
        for (i, item) in items.iter().enumerate() {
            s.push('\n');
            s.push_str(&pad);
            s.push_str(item);
            if i + 1 < items.len() {
                s.push(',');
            }
        }
        s.push('\n');
        s.push_str(&" ".repeat(indent));
        s.push_str(close);
        s
    }
}

fn more(n: usize) -> String {
    let s = if n == 1 { "" } else { "s" };
    format!("... {n} more item{s}")
}

fn accessor_label(p: Option<Property>) -> String {
    match p {
        Some(Property::Accessor { getter, setter, .. }) => {
            match (!getter.is_undefined(), !setter.is_undefined()) {
                (true, true) => "[Getter/Setter]",
                (true, false) => "[Getter]",
                (false, true) => "[Setter]",
                _ => "undefined",
            }
        }
        _ => "undefined",
    }
    .into()
}

pub fn number(n: f64) -> String {
    if n == 0.0 && n.is_sign_negative() {
        return "-0".into();
    }
    djs_core::num::to_string(n).as_str().into()
}

fn is_identifier(s: &str) -> bool {
    let mut cs = s.chars();
    matches!(cs.next(), Some(c) if c.is_alphabetic() || c == '_' || c == '$')
        && cs.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

/// A string literal in single quotes (double quotes when the text has single quotes and
/// no double quotes, as node does).
pub fn quote(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(q);
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            c if c == q => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push(q);
    out
}

/// A time value as `Date.prototype.toISOString` shows it (UTC), or `Invalid Date`.
fn iso_date(t: f64) -> String {
    if !t.is_finite() {
        return "Invalid Date".into();
    }
    let ms = t as i64;
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let year = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else {
        format!("{}{:06}", if y < 0 { '-' } else { '+' }, y.abs())
    };
    format!(
        "{year}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(iso_date(0.0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_date(1_700_000_000_123.0), "2023-11-14T22:13:20.123Z");
        assert_eq!(iso_date(-1.0), "1969-12-31T23:59:59.999Z");
        assert_eq!(iso_date(f64::NAN), "Invalid Date");
    }

    #[test]
    fn quoting() {
        assert_eq!(quote("a'b"), "\"a'b\"");
        assert_eq!(quote("a\n\"'"), "'a\\n\"\\''");
    }
}
