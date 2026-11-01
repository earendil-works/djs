//! Atoms (interned property keys) and symbols.
//!
//! An [`Atom`] is a `u32` id. Every property key is an atom:
//!
//! ```text
//! bit 31 = 1             integer index atom; bits 0-30 are the index (0 ..= 2^31-1)
//! bit 31 = 0, bit 30 = 1 symbol atom; bits 0-29 index the atom table
//! bit 31 = 0, bit 30 = 0 string atom; bits 0-29 index the atom table (0 = null atom)
//! ```
//!
//! Integer index atoms are computed, never interned, so element access does not touch the
//! atom table. A string that is a canonical array index below 2^31 always becomes an
//! integer atom; larger array indexes (up to 2^32-2) are string atoms, which
//! [`Heap::atom_array_index`] still recognizes.
//!
//! The atom table (a heap cell) maps a table index to its string or symbol, and has an
//! open-addressing hash index for interning strings. Atom ids are stable across GC (they
//! are indexes, not offsets), so bytecode and property tables can hold them without
//! being rewritten. Every symbol gets a table entry too, which makes symbols ordinary
//! property keys. For now atoms are never freed; atom GC is a follow-up.
//!
//! The well-known atoms (and the well-known symbols) have fixed ids, generated from the
//! list below, and are interned in order when the heap is created.

use alloc::string::String;
use core::fmt;

use super::cells::{AtomTable, Roots, Symbol, ValueArray};
use super::layout::Ref;
use super::string::{StrHasher, hash_units};
use super::{CellKind, Heap, HeapError};
use crate::value::{Str, Sym, Value};

pub(crate) const INT_BIT: u32 = 1 << 31;
pub(crate) const SYM_BIT: u32 = 1 << 30;
pub(crate) const INDEX_MASK: u32 = SYM_BIT - 1;

/// The largest integer index atom.
pub const MAX_INT_ATOM: u32 = INT_BIT - 1;

/// The largest array index (2^32 - 2).
pub const MAX_ARRAY_INDEX: u32 = u32::MAX - 1;

/// An interned property key: an integer index, a string or a symbol. See the module docs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Atom(pub(crate) u32);

impl Atom {
    /// The null atom: never a valid key (marks deleted property entries).
    pub const NULL: Atom = Atom(0);

    /// The integer atom for `i`, if `i <= MAX_INT_ATOM`. For any `u32` index use
    /// [`Heap::atom_from_index`].
    #[inline]
    pub const fn int(i: u32) -> Option<Atom> {
        if i <= MAX_INT_ATOM {
            Some(Atom(INT_BIT | i))
        } else {
            None
        }
    }

    /// The raw id.
    #[inline]
    pub const fn id(self) -> u32 {
        self.0
    }

    /// Rebuilds an atom from a raw id (for example a bytecode operand).
    #[inline]
    pub const fn from_id(id: u32) -> Atom {
        Atom(id)
    }

    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_int(self) -> bool {
        self.0 & INT_BIT != 0
    }

    #[inline]
    pub const fn is_symbol(self) -> bool {
        self.0 & (INT_BIT | SYM_BIT) == SYM_BIT
    }

    /// A string key (integer atoms are string keys in JS too, but this is false for them).
    #[inline]
    pub const fn is_string(self) -> bool {
        self.0 & (INT_BIT | SYM_BIT) == 0 && self.0 != 0
    }

    /// The index of an integer atom.
    #[inline]
    pub const fn as_int(self) -> Option<u32> {
        if self.is_int() {
            Some(self.0 & !INT_BIT)
        } else {
            None
        }
    }

    #[inline]
    const fn table_index(self) -> u32 {
        self.0 & INDEX_MASK
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(i) = self.as_int() {
            write!(f, "Atom(int {i})")
        } else if self.is_symbol() {
            write!(f, "Atom(sym #{})", self.table_index())
        } else {
            write!(f, "Atom(str #{})", self.table_index())
        }
    }
}

macro_rules! well_known {
    (
        strings { $( $name:ident = $s:literal, )* }
        symbols { $( $sname:ident = $sd:literal, )* }
    ) => {
        // Discriminants give the table indexes: strings first, then symbols.
        #[allow(non_camel_case_types, clippy::upper_case_acronyms, dead_code)]
        #[repr(u32)]
        enum WellKnown {
            Null,
            $( $name, )*
            $( $sname, )*
        }

        /// The well-known atoms, with fixed ids.
        pub mod atoms {
            use super::{Atom, SYM_BIT, WellKnown};
            $( #[doc = concat!("`\"", $s, "\"`")]
               pub const $name: Atom = Atom(WellKnown::$name as u32); )*
            $( #[doc = concat!("The well-known symbol `", $sd, "`.")]
               pub const $sname: Atom = Atom(SYM_BIT | WellKnown::$sname as u32); )*
        }

        const WELL_KNOWN_STRINGS: &[&str] = &[ $( $s, )* ];
        const WELL_KNOWN_SYMBOLS: &[&str] = &[ $( $sd, )* ];
        pub(crate) const WELL_KNOWN_NAMES_FOR_HASH: &[&str] = &[ $( $s, )* "|", $( $sd, )* ];
    };
}

well_known! {
    strings {
        EMPTY = "",
        LENGTH = "length",
        PROTOTYPE = "prototype",
        CONSTRUCTOR = "constructor",
        TO_STRING = "toString",
        TO_LOCALE_STRING = "toLocaleString",
        VALUE_OF = "valueOf",
        PROTO = "__proto__",
        NAME = "name",
        MESSAGE = "message",
        STACK = "stack",
        CAUSE = "cause",
        ERRORS = "errors",
        VALUE = "value",
        GET = "get",
        SET = "set",
        WRITABLE = "writable",
        ENUMERABLE = "enumerable",
        CONFIGURABLE = "configurable",
        UNDEFINED = "undefined",
        NULL = "null",
        TRUE = "true",
        FALSE = "false",
        NUMBER = "number",
        STRING = "string",
        BOOLEAN = "boolean",
        OBJECT = "object",
        FUNCTION = "function",
        SYMBOL = "symbol",
        BIGINT = "bigint",
        ARGUMENTS = "arguments",
        CALLEE = "callee",
        CALLER = "caller",
        DEFAULT = "default",
        DONE = "done",
        NEXT = "next",
        RETURN = "return",
        THROW = "throw",
        THEN = "then",
        CATCH = "catch",
        FINALLY = "finally",
        RESOLVE = "resolve",
        REJECT = "reject",
        TO_JSON = "toJSON",
        JOIN = "join",
        INDEX = "index",
        INPUT = "input",
        GROUPS = "groups",
        LAST_INDEX = "lastIndex",
        SOURCE = "source",
        FLAGS = "flags",
        RAW = "raw",
        KEYS = "keys",
        VALUES = "values",
        ENTRIES = "entries",
        SIZE = "size",
        NAN = "NaN",
        INFINITY = "Infinity",
        NEG_INFINITY = "-Infinity",
        GLOBAL_THIS = "globalThis",
        ERROR = "Error",
        TYPE_ERROR = "TypeError",
        RANGE_ERROR = "RangeError",
        SYNTAX_ERROR = "SyntaxError",
        REFERENCE_ERROR = "ReferenceError",
    }
    symbols {
        SYM_ITERATOR = "Symbol.iterator",
        SYM_ASYNC_ITERATOR = "Symbol.asyncIterator",
        SYM_HAS_INSTANCE = "Symbol.hasInstance",
        SYM_TO_PRIMITIVE = "Symbol.toPrimitive",
        SYM_TO_STRING_TAG = "Symbol.toStringTag",
        SYM_SPECIES = "Symbol.species",
        SYM_IS_CONCAT_SPREADABLE = "Symbol.isConcatSpreadable",
        SYM_UNSCOPABLES = "Symbol.unscopables",
        SYM_MATCH = "Symbol.match",
        SYM_MATCH_ALL = "Symbol.matchAll",
        SYM_REPLACE = "Symbol.replace",
        SYM_SEARCH = "Symbol.search",
        SYM_SPLIT = "Symbol.split",
    }
}

/// Initial atom table capacity (entries) and hash index size (buckets, a power of two).
const INITIAL_ATOMS: u32 = 256;
const INITIAL_BUCKETS: u32 = 512;

/// Parses a canonical array index ("0", or no leading zero, at most 2^32-2).
fn parse_array_index(units: impl Iterator<Item = u16>) -> Option<u32> {
    let mut n: u64 = 0;
    let mut len = 0;
    for u in units {
        if !(b'0' as u16..=b'9' as u16).contains(&u) || (len > 0 && n == 0) || len >= 10 {
            return None;
        }
        n = n * 10 + (u - b'0' as u16) as u64;
        len += 1;
    }
    if len == 0 || n > MAX_ARRAY_INDEX as u64 {
        None
    } else {
        Some(n as u32)
    }
}

/// Writes `n` in decimal into `buf` and returns the used tail.
fn format_u32(n: u32, buf: &mut [u8; 10]) -> &[u8] {
    let mut i = buf.len();
    let mut n = n;
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            return &buf[i..];
        }
    }
}

impl Heap {
    /// Creates the atom table and interns the well-known atoms and symbols.
    pub(crate) fn init_atoms(&mut self) -> Result<(), HeapError> {
        let table = self.alloc_cell(CellKind::AtomTable, AtomTable::LAYOUT.min_size)?;
        let entries = self.alloc_value_array(INITIAL_ATOMS)?;
        let index = self.alloc_u32_array(INITIAL_BUCKETS)?;
        let a = self.arena_mut();
        a.set(entries, ValueArray::LEN, 1); // slot 0 is the null atom
        a.set(table, AtomTable::ENTRIES, Ref(entries));
        a.set(table, AtomTable::INDEX, Ref(index));
        a.set(0, Roots::ATOMS, Ref(table));
        for (i, s) in WELL_KNOWN_STRINGS.iter().enumerate() {
            let atom = self.atom_str(s)?;
            assert_eq!(
                atom.0 as usize,
                i + 1,
                "well-known atom {s:?} is not unique"
            );
        }
        for (i, d) in WELL_KNOWN_SYMBOLS.iter().enumerate() {
            let desc = self.alloc_str(d)?;
            let sym = self.new_symbol(Value::string(desc))?;
            let atom = self.symbol_atom(sym);
            assert_eq!(atom.0, SYM_BIT | (WELL_KNOWN_STRINGS.len() + 1 + i) as u32);
        }
        Ok(())
    }

    #[inline]
    pub(super) fn atom_table(&self) -> u32 {
        self.arena().get(0, Roots::ATOMS).0
    }

    #[inline]
    fn atom_entries(&self) -> u32 {
        self.arena().get(self.atom_table(), AtomTable::ENTRIES).0
    }

    /// Number of atom table entries (string and symbol atoms; slot 0 included).
    pub fn atom_count(&self) -> u32 {
        self.value_array_len(self.atom_entries())
    }

    /// The table entry of a string or symbol atom.
    fn atom_entry(&self, a: Atom) -> Value {
        debug_assert!(!a.is_int() && !a.is_null());
        let i = a.table_index();
        let entries = self.atom_entries();
        assert!(i < self.value_array_len(entries), "invalid atom {a:?}");
        self.value_array_get(entries, i)
    }

    /// Appends an atom table entry and returns its index.
    fn push_atom_entry(&mut self, v: Value) -> Result<u32, HeapError> {
        let table = self.atom_table();
        let entries = self.atom_entries();
        let len = self.value_array_len(entries);
        if len > INDEX_MASK {
            return Err(HeapError::OutOfMemory);
        }
        let entries = self.value_array_reserve(entries, len + 1)?;
        self.arena_mut()
            .set(table, AtomTable::ENTRIES, Ref(entries));
        self.value_array_set(entries, len, v);
        self.arena_mut().set(entries, ValueArray::LEN, len + 1);
        Ok(len)
    }

    /// Finds a string atom by hash and content.
    fn find_string_atom(&self, hash: u32, eq: impl Fn(&Heap, Str) -> bool) -> Option<Atom> {
        let index = self.arena().get(self.atom_table(), AtomTable::INDEX).0;
        let entries = self.atom_entries();
        self.probe(index, hash, |_, idx| {
            if idx == 0 {
                return Some(None);
            }
            let s = self
                .value_array_get(entries, idx)
                .as_string()
                .expect("string atom");
            (self.str_hash_uncached(s) == hash && eq(self, s)).then_some(Some(Atom(idx)))
        })
    }

    fn index_insert(&mut self, index: u32, hash: u32, atom_index: u32) {
        let b = self.probe_empty(index, hash);
        self.u32_array_set(index, b, atom_index);
    }

    /// Adds the flat string `s` (not yet interned, hash cached) as a new string atom.
    fn add_string_atom(&mut self, s: Str, hash: u32) -> Result<Atom, HeapError> {
        let table = self.atom_table();
        let strings = self.arena().get(table, AtomTable::STRINGS) + 1;
        let mut index = self.arena().get(table, AtomTable::INDEX).0;
        // Keep the load factor at or below 1/2. Grow before pushing the entry so a failed
        // allocation leaves the table unchanged.
        if strings * 2 > self.u32_array_len(index) {
            let new = self.alloc_u32_array(self.u32_array_len(index) * 2)?;
            let entries = self.atom_entries();
            for i in 1..self.value_array_len(entries) {
                if let Some(t) = self.value_array_get(entries, i).as_string() {
                    let h = self.str_hash_uncached(t);
                    self.index_insert(new, h, i);
                }
            }
            index = new;
            self.arena_mut().set(table, AtomTable::INDEX, Ref(index));
        }
        let idx = self.push_atom_entry(Value::string(s))?;
        self.index_insert(index, hash, idx);
        self.arena_mut().set(table, AtomTable::STRINGS, strings);
        Ok(Atom(idx))
    }

    /// Interns a heap string. Canonical array indexes below 2^31 become integer atoms.
    pub fn intern(&mut self, s: Str) -> Result<Atom, HeapError> {
        if let Some(i) = self.str_array_index(s)
            && let Some(a) = Atom::int(i)
        {
            return Ok(a);
        }
        let hash = self.str_hash(s);
        if let Some(a) = self.find_string_atom(hash, |h, t| h.str_eq(s, t)) {
            return Ok(a);
        }
        let flat = self.flatten(s)?;
        self.str_hash(flat);
        self.add_string_atom(flat, hash)
    }

    /// Interns a Rust string; allocates only if it is not interned yet.
    pub fn atom_str(&mut self, t: &str) -> Result<Atom, HeapError> {
        if let Some(i) = parse_array_index(t.bytes().map(u16::from))
            && let Some(a) = Atom::int(i)
        {
            return Ok(a);
        }
        let hash = if t.is_ascii() {
            let mut h = StrHasher::new();
            for b in t.bytes() {
                h.write(b as u16);
            }
            h.finish()
        } else {
            hash_units(t.encode_utf16())
        };
        if let Some(a) = self.find_string_atom(hash, |h, s| h.str_eq_str(s, t)) {
            return Ok(a);
        }
        let s = self.alloc_str(t)?;
        self.str_hash(s);
        self.add_string_atom(s, hash)
    }

    /// Interns a string given as UTF-16 code units; allocates only if it is not interned
    /// yet.
    pub fn atom_utf16(&mut self, units: &[u16]) -> Result<Atom, HeapError> {
        if let Some(i) = parse_array_index(units.iter().copied())
            && let Some(a) = Atom::int(i)
        {
            return Ok(a);
        }
        let hash = hash_units(units.iter().copied());
        let found = self.find_string_atom(hash, |h, s| match h.str_view(s) {
            Some(v) => v.len() == units.len() && v.units().eq(units.iter().copied()),
            None => false,
        });
        if let Some(a) = found {
            return Ok(a);
        }
        let s = self.alloc_utf16(units)?;
        self.str_hash(s);
        self.add_string_atom(s, hash)
    }

    /// The atom for an array index: an integer atom below 2^31, else an interned string.
    pub fn atom_from_index(&mut self, i: u32) -> Result<Atom, HeapError> {
        if let Some(a) = Atom::int(i) {
            return Ok(a);
        }
        let mut buf = [0; 10];
        let digits = format_u32(i, &mut buf);
        let t = core::str::from_utf8(digits).expect("ascii digits");
        self.atom_str(t)
    }

    /// The array index a key denotes (integer atoms, and string atoms such as
    /// `"4294967294"`), or `None`.
    pub fn atom_array_index(&self, a: Atom) -> Option<u32> {
        if let Some(i) = a.as_int() {
            return Some(i);
        }
        if !a.is_string() {
            return None;
        }
        let s = self.atom_entry(a).as_string()?;
        // Shorter strings would have become integer atoms.
        if self.str_len(s) != 10 {
            return None;
        }
        self.str_array_index(s)
    }

    /// Parses `s` as a canonical array index (0 ..= 2^32-2).
    pub fn str_array_index(&self, s: Str) -> Option<u32> {
        let len = self.str_len(s);
        if len == 0 || len > 10 {
            return None;
        }
        let first = self.char_code_at(s, 0)?;
        if !(b'0' as u16..=b'9' as u16).contains(&first) {
            return None;
        }
        parse_array_index(self.str_units(s))
    }

    /// The string of a string or integer atom. Integer atoms allocate a new string.
    /// Panics on a symbol atom.
    pub fn atom_string(&mut self, a: Atom) -> Result<Str, HeapError> {
        if let Some(i) = a.as_int() {
            let mut buf = [0; 10];
            return self.alloc_latin1(format_u32(i, &mut buf));
        }
        assert!(!a.is_symbol(), "atom_string on a symbol atom");
        Ok(self.atom_entry(a).as_string().expect("string atom"))
    }

    /// The key as a JS value: a string or a symbol. Integer atoms allocate a string.
    pub fn atom_to_value(&mut self, a: Atom) -> Result<Value, HeapError> {
        if a.is_symbol() {
            Ok(self.atom_entry(a))
        } else {
            Ok(Value::string(self.atom_string(a)?))
        }
    }

    /// The symbol of a symbol atom.
    pub fn atom_symbol(&self, a: Atom) -> Option<Sym> {
        if a.is_symbol() {
            self.atom_entry(a).as_symbol()
        } else {
            None
        }
    }

    /// A readable name for debugging: the string, `[description]` for symbols.
    pub fn atom_name(&self, a: Atom) -> String {
        if let Some(i) = a.as_int() {
            let mut buf = [0; 10];
            return String::from(core::str::from_utf8(format_u32(i, &mut buf)).unwrap());
        }
        if a.is_null() {
            return String::from("<null atom>");
        }
        let v = self.atom_entry(a);
        if let Some(s) = v.as_string() {
            return self.str_to_string(s);
        }
        let sym = v.as_symbol().expect("symbol atom");
        let mut out = String::from("[");
        if let Some(d) = self.symbol_description(sym).as_string() {
            out.push_str(&self.str_to_string(d));
        }
        out.push(']');
        out
    }

    /// Creates a new symbol. `description` is a string value or `undefined`.
    pub fn new_symbol(&mut self, description: Value) -> Result<Sym, HeapError> {
        debug_assert!(description.is_undefined() || description.as_string().is_some());
        let cell = self.alloc_cell(CellKind::Symbol, Symbol::LAYOUT.min_size)?;
        self.arena_mut().set(cell, Symbol::DESCRIPTION, description);
        let idx = self.push_atom_entry(Value::symbol(Sym(cell)))?;
        self.arena_mut().set(cell, Symbol::ATOM, SYM_BIT | idx);
        Ok(Sym(cell))
    }

    /// The property key of a symbol.
    pub fn symbol_atom(&self, s: Sym) -> Atom {
        Atom(self.arena().get(s.0, Symbol::ATOM))
    }

    /// A string value or `undefined`.
    pub fn symbol_description(&self, s: Sym) -> Value {
        self.arena().get(s.0, Symbol::DESCRIPTION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_index_parsing() {
        let p = |s: &str| parse_array_index(s.bytes().map(u16::from));
        assert_eq!(p("0"), Some(0));
        assert_eq!(p("7"), Some(7));
        assert_eq!(p("4294967294"), Some(4294967294));
        assert_eq!(p("4294967295"), None);
        assert_eq!(p("10000000000"), None);
        assert_eq!(p("01"), None);
        assert_eq!(p("00"), None);
        assert_eq!(p(""), None);
        assert_eq!(p("-1"), None);
        assert_eq!(p("1.5"), None);
        assert_eq!(p("1e3"), None);
        assert_eq!(p(" 1"), None);
    }

    #[test]
    fn format() {
        let mut b = [0; 10];
        assert_eq!(format_u32(0, &mut b), b"0");
        assert_eq!(format_u32(u32::MAX, &mut b), b"4294967295");
        assert_eq!(format_u32(1050, &mut b), b"1050");
    }

    #[test]
    fn atom_bits() {
        let a = Atom::int(5).unwrap();
        assert!(a.is_int() && !a.is_string() && !a.is_symbol());
        assert_eq!(a.as_int(), Some(5));
        assert_eq!(
            Atom::int(MAX_INT_ATOM).unwrap().as_int(),
            Some(MAX_INT_ATOM)
        );
        assert_eq!(Atom::int(MAX_INT_ATOM + 1), None);
        assert!(atoms::LENGTH.is_string());
        assert!(atoms::SYM_ITERATOR.is_symbol());
        assert!(!Atom::NULL.is_string() && !Atom::NULL.is_symbol());
        assert_eq!(atoms::EMPTY.id(), 1);
    }
}
