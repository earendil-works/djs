//! Record schemas and named constants (D3710): the `enum` declarations of the builtin
//! sources.
//!
//! - `@record` names the fields of a kind of internal record (an object of class Record,
//!   or `@class IteratorHelper`), in slot order. `NewRecord(Name)` makes one; the record
//!   carries its kind (`Name`'s id) in its header, so a logical snapshot writes it as
//!   `Name` with named fields. `@rest name` gives it a variable tail after the fields,
//!   `@extends Base` says that its fields start with Base's (code that reads only those
//!   takes either).
//! - `@slots Class` names (some of) an object class's slots, for builtins that keep state
//!   there: the generated Rust checks them against the class table at compile time.
//! - `@constants` and `@flags` name the values of a field (members `= value`; `@flags`
//!   values are bits). A field says which set it holds with `@enum Set` or `@flags Set`, so
//!   a snapshot writes the names instead of the numbers.
//!
//! `Name.member` is the slot index of a field or the value of a constant: records compile
//! to the same slot accesses as before. The table is generated as Rust
//! (`crates/djs-core/src/builtins/gen/records.rs`), which the engine's own readers of the
//! records and the snapshot use.

use std::collections::HashMap;
use std::fmt::Write;

use crate::ast::{Block, EnumDecl, Expr, ExprKind, Module, Stmt};
use crate::diag::{Diagnostic, Sources, Span};

/// The record kinds fit the object header's six bits (`OBJ_RECORD_KIND`); 0 is none.
pub const MAX_KINDS: u32 = 63;

#[derive(Debug, Clone, PartialEq)]
pub enum FieldTy {
    Value,
    Enum(String),
    Flags(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumKind {
    Record {
        id: u32,
        class: String,
        rest: Option<String>,
    },
    Slots {
        class: String,
    },
    Constants {
        flags: bool,
    },
}

#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub kind: EnumKind,
    /// Records and slots: the fields, by slot index.
    pub fields: Vec<(String, FieldTy)>,
    /// Constants: the members and their values.
    pub values: Vec<(String, i64)>,
    pub doc_file: u32,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct Enums {
    pub list: Vec<EnumInfo>,
    by_name: HashMap<String, usize>,
}

impl Enums {
    pub fn get(&self, name: &str) -> Option<&EnumInfo> {
        self.by_name.get(name).map(|&i| &self.list[i])
    }
}

const CLASSES: &[&str] = &["Record", "IteratorHelper"];

/// Collects and checks every enum: intrinsics.d.ts's first, then the files' in order
/// (that order numbers the record kinds; nothing outside one build depends on it).
pub fn collect(decls: &[EnumDecl], modules: &[Module], errors: &mut Vec<Diagnostic>) -> Enums {
    let all: Vec<&EnumDecl> = decls
        .iter()
        .chain(modules.iter().flat_map(|m| m.enums.iter()))
        .collect();
    let mut out = Enums::default();
    let mut next_id = 1;
    for e in &all {
        let tag = |n: &str| e.tags.iter().find(|t| t.name == n).map(|t| t.value.clone());
        let kinds = ["record", "slots", "constants", "flags"]
            .iter()
            .filter(|k| tag(k).is_some())
            .count();
        if kinds != 1 {
            errors.push(Diagnostic::new(
                e.span,
                format!(
                    "enum `{}` needs exactly one of `@record`, `@slots Class`, `@constants`, `@flags`",
                    e.name
                ),
            ));
            continue;
        }
        for t in &e.tags {
            if ![
                "record",
                "slots",
                "constants",
                "flags",
                "class",
                "rest",
                "extends",
            ]
            .contains(&t.name.as_str())
            {
                errors.push(Diagnostic::new(
                    e.span,
                    format!("unknown tag `@{}` on an enum", t.name),
                ));
            }
        }
        let mut info = EnumInfo {
            name: e.name.clone(),
            kind: EnumKind::Constants { flags: false },
            fields: Vec::new(),
            values: Vec::new(),
            doc_file: e.file,
            span: e.span,
        };
        let mut seen = HashMap::new();
        for m in &e.members {
            if seen.insert(m.name.clone(), ()).is_some() {
                errors.push(Diagnostic::new(
                    m.span,
                    format!("`{}.{}` twice", e.name, m.name),
                ));
            }
            let reserved = ["RECORD_KIND", "FIELD_COUNT"];
            if reserved.contains(&upper_snake(&m.name).as_str()) {
                errors.push(Diagnostic::new(
                    m.span,
                    format!("`{}` is reserved in the generated Rust", m.name),
                ));
            }
        }
        if tag("constants").is_some() || tag("flags").is_some() {
            let flags = tag("flags").is_some();
            info.kind = EnumKind::Constants { flags };
            let mut next = 0i64;
            for m in &e.members {
                if !m.tags.is_empty() {
                    errors.push(Diagnostic::new(m.span, "a constant has no tags"));
                }
                let v = match m.value {
                    Some(v) => v,
                    None if flags => {
                        errors.push(Diagnostic::new(m.span, "a flag needs its bit: `= 1`"));
                        0
                    }
                    None => next,
                };
                if flags && (v <= 0 || v & (v - 1) != 0) {
                    errors.push(Diagnostic::new(m.span, "a flag is one bit"));
                }
                if info.values.iter().any(|(_, w)| *w == v) {
                    errors.push(Diagnostic::new(
                        m.span,
                        format!("`{}.{}` has another member's value", e.name, m.name),
                    ));
                }
                info.values.push((m.name.clone(), v));
                next = v + 1;
            }
        } else {
            for m in &e.members {
                if m.value.is_some() {
                    errors.push(Diagnostic::new(
                        m.span,
                        "a field's slot is its position: no `= value`",
                    ));
                }
                let mut ty = FieldTy::Value;
                for t in &m.tags {
                    match t.name.as_str() {
                        "enum" => ty = FieldTy::Enum(t.value.clone()),
                        "flags" => ty = FieldTy::Flags(t.value.clone()),
                        n => errors.push(Diagnostic::new(
                            m.span,
                            format!("unknown tag `@{n}` on a field (`@enum Set`, `@flags Set`)"),
                        )),
                    }
                }
                info.fields.push((m.name.clone(), ty));
            }
            if let Some(class) = tag("slots") {
                info.kind = EnumKind::Slots { class };
            } else {
                let class = tag("class").unwrap_or_else(|| String::from("Record"));
                if !CLASSES.contains(&class.as_str()) {
                    errors.push(Diagnostic::new(
                        e.span,
                        format!("a record's class is one of {CLASSES:?}"),
                    ));
                }
                if next_id > MAX_KINDS {
                    errors.push(Diagnostic::new(
                        e.span,
                        format!("more than {MAX_KINDS} record kinds"),
                    ));
                }
                info.kind = EnumKind::Record {
                    id: next_id,
                    class,
                    rest: tag("rest").filter(|r| !r.is_empty()),
                };
                next_id += 1;
            }
            if let Some(base) = tag("extends") {
                match out.get(&base) {
                    Some(b) if matches!(b.kind, EnumKind::Record { .. }) => {
                        // The base's fields by name, in order; their types are inherited.
                        let ok = info.fields.len() >= b.fields.len()
                            && b.fields
                                .iter()
                                .zip(&mut info.fields)
                                .all(|((bn, bt), (n, t))| {
                                    if *t == FieldTy::Value {
                                        *t = bt.clone();
                                    }
                                    bn == n && bt == t
                                });
                        if !ok {
                            errors.push(Diagnostic::new(
                                e.span,
                                format!("`{}` must start with the fields of `{base}`", e.name),
                            ));
                        }
                    }
                    _ => errors.push(Diagnostic::new(
                        e.span,
                        format!("`@extends {base}`: no record `{base}` declared before"),
                    )),
                }
            }
        }
        if out.by_name.insert(e.name.clone(), out.list.len()).is_some() {
            errors.push(Diagnostic::new(
                e.span,
                format!("enum `{}` is declared twice", e.name),
            ));
        }
        out.list.push(info);
    }
    // Field types name constant sets.
    for e in &out.list {
        for (f, t) in &e.fields {
            if let FieldTy::Enum(s) | FieldTy::Flags(s) = t {
                let want_flags = matches!(t, FieldTy::Flags(_));
                match out.get(s).map(|c| &c.kind) {
                    Some(EnumKind::Constants { flags }) if *flags == want_flags => {}
                    _ => errors.push(Diagnostic::new(
                        e.span,
                        format!(
                            "{}.{f}: `{s}` is not a {} enum",
                            e.name,
                            if want_flags { "@flags" } else { "@constants" }
                        ),
                    )),
                }
            }
        }
    }
    out
}

/// Replaces `Enum.member` with its number and a record enum named as a value with its
/// kind id, in every function body.
pub fn resolve(enums: &Enums, modules: &mut [Module], errors: &mut Vec<Diagnostic>) {
    for m in modules {
        for f in &mut m.functions {
            block(enums, &mut f.body, errors);
        }
    }
}

fn block(enums: &Enums, b: &mut Block, errors: &mut Vec<Diagnostic>) {
    for s in &mut b.stmts {
        stmt(enums, s, errors);
    }
}

fn stmt(enums: &Enums, s: &mut Stmt, errors: &mut Vec<Diagnostic>) {
    match s {
        Stmt::Decl { init, .. } => expr(enums, init, errors),
        Stmt::Assign { value, .. } => expr(enums, value, errors),
        Stmt::Update { .. } | Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::Expr(e) | Stmt::Throw { value: e, .. } => expr(enums, e, errors),
        Stmt::If {
            cond, then, els, ..
        } => {
            expr(enums, cond, errors);
            stmt(enums, then, errors);
            if let Some(e) = els {
                stmt(enums, e, errors);
            }
        }
        Stmt::While { cond, body, .. } | Stmt::DoWhile { body, cond, .. } => {
            expr(enums, cond, errors);
            stmt(enums, body, errors);
        }
        Stmt::For {
            init,
            cond,
            update,
            body,
            ..
        } => {
            if let Some(i) = init {
                stmt(enums, i, errors);
            }
            if let Some(c) = cond {
                expr(enums, c, errors);
            }
            if let Some(u) = update {
                stmt(enums, u, errors);
            }
            stmt(enums, body, errors);
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                expr(enums, v, errors);
            }
        }
        Stmt::Try {
            body,
            catch,
            finally,
            ..
        } => {
            block(enums, body, errors);
            if let Some(c) = catch {
                block(enums, &mut c.body, errors);
            }
            if let Some(f) = finally {
                block(enums, f, errors);
            }
        }
        Stmt::Block(b) => block(enums, b, errors),
    }
}

fn expr(enums: &Enums, e: &mut Expr, errors: &mut Vec<Diagnostic>) {
    match &mut e.kind {
        ExprKind::Member { object, member } => {
            let Some(info) = enums.get(object) else {
                errors.push(Diagnostic::new(
                    e.span,
                    format!(
                        "property access is not in the subset (`{object}` is no enum); use an intrinsic such as Get(o, key)"
                    ),
                ));
                return;
            };
            let v = match info.kind {
                EnumKind::Constants { .. } => info
                    .values
                    .iter()
                    .find(|(n, _)| n == member)
                    .map(|(_, v)| *v),
                _ => info
                    .fields
                    .iter()
                    .position(|(n, _)| n == member)
                    .map(|i| i as i64),
            };
            match v {
                Some(v) => e.kind = ExprKind::Num(v as f64, v.to_string()),
                None => errors.push(Diagnostic::new(
                    e.span,
                    format!("`{object}` has no member `{member}`"),
                )),
            }
        }
        ExprKind::Ident(name) => {
            if let Some(EnumKind::Record { id, .. }) = enums.get(name).map(|i| &i.kind) {
                e.kind = ExprKind::Schema(name.clone(), *id);
            } else if enums.get(name).is_some() {
                errors.push(Diagnostic::new(
                    e.span,
                    format!("`{name}` is not a record schema: name a member (`{name}.x`)"),
                ));
            }
        }
        ExprKind::Unary(_, a) | ExprKind::Paren(a) => expr(enums, a, errors),
        ExprKind::Binary(_, a, b) => {
            expr(enums, a, errors);
            expr(enums, b, errors);
        }
        ExprKind::Cond(a, b, c) => {
            expr(enums, a, errors);
            expr(enums, b, errors);
            expr(enums, c, errors);
        }
        ExprKind::Call { args, .. } => {
            for a in args {
                expr(enums, a, errors);
            }
        }
        _ => {}
    }
}

pub fn upper_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

pub fn snake(s: &str) -> String {
    upper_snake(s).to_ascii_lowercase()
}

fn slot_kind(t: &FieldTy) -> String {
    match t {
        FieldTy::Value => String::from("SlotKind::Value"),
        FieldTy::Enum(s) => format!("SlotKind::Enum(&{})", upper_snake(s)),
        FieldTy::Flags(s) => format!("SlotKind::Bits(&{})", upper_snake(s)),
    }
}

/// `crates/djs-core/src/builtins/gen/records.rs`.
pub fn emit(enums: &Enums, sources: &Sources) -> String {
    let mut s = String::from(
        "// @generated by djs-bgen from builtins/*.ts (scripts/bgen.sh): do not edit.\n\
         //! The builtins' record schemas and named constants (D3710): the `enum`s of\n\
         //! builtins/*.ts. A record's fields are its slots in order; `RECORD_KIND` is the\n\
         //! id its header carries. Snapshots write records and constants by these names.\n\n\
         #![allow(dead_code)]\n\n\
         use crate::heap::{ConstSet, ObjectClass, RecordSchema, SlotKind};\n",
    );
    for e in &enums.list {
        let file = &sources.files[e.doc_file as usize].name;
        let m = snake(&e.name);
        match &e.kind {
            EnumKind::Constants { flags } => {
                let _ = writeln!(s, "\n/// `{}` ({file}).\npub mod {m} {{", e.name);
                for (n, v) in &e.values {
                    let _ = writeln!(s, "    pub const {}: i32 = {v};", upper_snake(n));
                }
                s.push_str("}\n");
                let _ = writeln!(
                    s,
                    "pub const {}: ConstSet = ConstSet {{\n    name: \"{}\",\n    flags: {flags},\n    members: &[{}],\n}};",
                    upper_snake(&e.name),
                    e.name,
                    e.values
                        .iter()
                        .map(|(n, v)| format!("(\"{n}\", {v})"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            EnumKind::Record { .. } | EnumKind::Slots { .. } => {
                let _ = writeln!(s, "\n/// `{}` ({file}).\npub mod {m} {{", e.name);
                if let EnumKind::Record { id, .. } = e.kind {
                    let _ = writeln!(s, "    pub const RECORD_KIND: u16 = {id};");
                    let _ = writeln!(s, "    pub const FIELD_COUNT: u32 = {};", e.fields.len());
                }
                for (i, (n, _)) in e.fields.iter().enumerate() {
                    let _ = writeln!(s, "    pub const {}: u32 = {i};", upper_snake(n));
                }
                s.push_str("}\n");
            }
        }
    }
    s.push_str("\n/// Every constant set, by name.\npub static CONSTANT_SETS: &[&ConstSet] = &[\n");
    for e in &enums.list {
        if let EnumKind::Constants { .. } = e.kind {
            let _ = writeln!(s, "    &{},", upper_snake(&e.name));
        }
    }
    s.push_str("];\n\n/// Every record kind, by `RECORD_KIND - 1`.\npub static RECORDS: &[RecordSchema] = &[\n");
    for e in &enums.list {
        if let EnumKind::Record { id, class, rest } = &e.kind {
            let _ = writeln!(
                s,
                "    RecordSchema {{\n        name: \"{}\",\n        kind: {id},\n        class: ObjectClass::{class},\n        fields: &[{}],\n        rest: {},\n    }},",
                e.name,
                e.fields
                    .iter()
                    .map(|(n, t)| format!("(\"{n}\", {})", slot_kind(t)))
                    .collect::<Vec<_>>()
                    .join(", "),
                match rest {
                    Some(r) => format!("Some(\"{r}\")"),
                    None => String::from("None"),
                }
            );
        }
    }
    s.push_str("];\n\n// The `@slots` views name the class table's slots (heap/object.rs).\nconst _: () = {\n");
    for e in &enums.list {
        if let EnumKind::Slots { class } = &e.kind {
            for (i, (n, t)) in e.fields.iter().enumerate() {
                let _ = writeln!(s, "    assert!(ObjectClass::{class}.slot(\"{n}\") == {i});");
                let _ = writeln!(
                    s,
                    "    assert!(ObjectClass::{class}.slot_kind(\"{n}\").same({}));",
                    slot_kind(t)
                );
            }
        }
    }
    s.push_str("};\n");
    s
}
