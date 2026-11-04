//! A small JSON value, parser and writer (djs-cli has no serde): the heap report's structured
//! sections and the tools' answers.

use std::fmt::Write;

#[derive(Clone, Debug, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    pub fn obj(fields: Vec<(&str, J)>) -> J {
        J::Obj(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            J::Null => out.push_str("null"),
            J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            J::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => {
                let _ = write!(out, "{}", *n as i64);
            }
            J::Num(n) if n.is_finite() => {
                let _ = write!(out, "{n}");
            }
            J::Num(_) => out.push_str("null"),
            J::Str(s) => quote(out, s),
            J::Arr(items) if items.is_empty() => out.push_str("[]"),
            J::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    out.push_str(if i == 0 { "\n" } else { ",\n" });
                    out.push_str(&"  ".repeat(indent + 1));
                    v.write(out, indent + 1);
                }
                out.push('\n');
                out.push_str(&"  ".repeat(indent));
                out.push(']');
            }
            J::Obj(fields) if fields.is_empty() => out.push_str("{}"),
            J::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    out.push_str(if i == 0 { "\n" } else { ",\n" });
                    out.push_str(&"  ".repeat(indent + 1));
                    quote(out, k);
                    out.push_str(": ");
                    v.write(out, indent + 1);
                }
                out.push('\n');
                out.push_str(&"  ".repeat(indent));
                out.push('}');
            }
        }
    }
}

fn quote(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl From<&str> for J {
    fn from(s: &str) -> J {
        J::Str(s.to_string())
    }
}
impl From<String> for J {
    fn from(s: String) -> J {
        J::Str(s)
    }
}
impl From<u64> for J {
    fn from(n: u64) -> J {
        J::Num(n as f64)
    }
}
impl From<u32> for J {
    fn from(n: u32) -> J {
        J::Num(n as f64)
    }
}
impl From<usize> for J {
    fn from(n: usize) -> J {
        J::Num(n as f64)
    }
}
impl From<f64> for J {
    fn from(n: f64) -> J {
        J::Num(n)
    }
}
impl From<bool> for J {
    fn from(b: bool) -> J {
        J::Bool(b)
    }
}
impl<T: Into<J>> From<Option<T>> for J {
    fn from(o: Option<T>) -> J {
        o.map_or(J::Null, Into::into)
    }
}

#[cfg(test)]
impl J {
    /// The value at a path of object keys and array indexes (tests).
    pub fn at(&self, path: &[&str]) -> &J {
        let mut cur = self;
        for p in path {
            cur = match cur {
                J::Obj(fields) => fields
                    .iter()
                    .find(|(k, _)| k == p)
                    .map(|(_, v)| v)
                    .unwrap_or(&J::Null),
                J::Arr(items) => p
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| items.get(i))
                    .unwrap_or(&J::Null),
                _ => &J::Null,
            };
        }
        cur
    }

    pub fn num(&self) -> f64 {
        match self {
            J::Num(n) => *n,
            _ => f64::NAN,
        }
    }

    pub fn str(&self) -> &str {
        match self {
            J::Str(s) => s,
            _ => "",
        }
    }

    pub fn arr(&self) -> &[J] {
        match self {
            J::Arr(a) => a,
            _ => &[],
        }
    }
}
