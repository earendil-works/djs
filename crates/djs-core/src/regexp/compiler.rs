//! Tree to bytecode.
//!
//! A program is a `Vec<u32>`: a header, the group names, then the code. Jump targets are
//! absolute word indices. An instruction is an opcode word (the low 8 bits are the opcode,
//! the bits above are flags) followed by its operands. The quantifier scheme follows
//! QuickJS: counters and saved positions live in registers next to the capture slots, and
//! empty iterations beyond the minimum fail (the spec's RepeatMatcher rule).
use crate::PushOut;

use alloc::vec::Vec;

use super::Flags;
use super::charset::{self, CharSet};
use super::parser::{Node, Parsed};

// Header words.
pub(crate) const H_FLAGS: usize = 0;
pub(crate) const H_CAPTURES: usize = 1;
pub(crate) const H_REGISTERS: usize = 2;
/// Entry point that searches forward from the start index.
pub(crate) const H_SEARCH: usize = 3;
/// Entry point that only matches at the start index.
pub(crate) const H_ANCHORED: usize = 4;
const H_LEN: usize = 5;

// Opcodes.
pub(crate) const MATCH: u32 = 0;
/// `CHAR c`
pub(crate) const CHAR: u32 = 1;
/// `CLASS n (lo hi){n}`: inclusive ranges, sorted.
pub(crate) const CLASS: u32 = 2;
/// Any character but a line terminator.
pub(crate) const DOT: u32 = 3;
/// Any character.
pub(crate) const ANY: u32 = 4;
pub(crate) const LINE_START: u32 = 5;
pub(crate) const LINE_END: u32 = 6;
pub(crate) const WORD_BOUNDARY: u32 = 7;
/// `GOTO target`
pub(crate) const GOTO: u32 = 8;
/// `SPLIT_NEXT target`: continue with the next instruction; on failure, `target`.
pub(crate) const SPLIT_NEXT: u32 = 9;
/// `SPLIT_GOTO target`: continue at `target`; on failure, the next instruction.
pub(crate) const SPLIT_GOTO: u32 = 10;
/// `SAVE slot`: the current position into a capture slot (2 * group + 0 or 1).
pub(crate) const SAVE: u32 = 11;
/// `RESET first last`: groups `first..=last` become undefined.
pub(crate) const RESET: u32 = 12;
/// `BACKREF n group{n}`: matches the first of the groups that is defined.
pub(crate) const BACKREF: u32 = 13;
/// `LOOK continuation`: starts a lookaround; `LOOK_END` ends its body.
pub(crate) const LOOK: u32 = 14;
pub(crate) const LOOK_END: u32 = 15;
/// `SET_REG r value`
pub(crate) const SET_REG: u32 = 16;
/// `SET_POS r`: the current position into register `r`.
pub(crate) const SET_POS: u32 = 17;
/// `CHECK_ADV r`: fails if the position equals register `r`.
pub(crate) const CHECK_ADV: u32 = 18;
/// `LOOP r target`: decrements register `r` and jumps to `target` unless it became 0.
pub(crate) const LOOP: u32 = 19;
/// `LOOP_SPLIT r limit target`: decrements the counter `r`. Above `limit` the iteration is
/// mandatory (jump to `target`); otherwise, with `F_CHECK`, an empty iteration fails (the
/// saved position is in `r + 1`), and while the counter is not 0, another iteration is
/// optional (greedy or lazy).
pub(crate) const LOOP_SPLIT: u32 = 20;

// Opcode flags.
pub(crate) const F_ICASE: u32 = 1 << 8;
pub(crate) const F_BACK: u32 = 1 << 9;
pub(crate) const F_NEG: u32 = 1 << 10;
pub(crate) const F_GREEDY: u32 = 1 << 11;
pub(crate) const F_CHECK: u32 = 1 << 12;
pub(crate) const F_MULTILINE: u32 = 1 << 13;
/// `\b` in a case-insensitive unicode pattern: U+017F and U+212A are word characters.
pub(crate) const F_UICASE: u32 = 1 << 14;

pub(crate) const OP_MASK: u32 = 0xFF;

/// `SCAN n (lo hi){n}`: advances to the next position whose character is in the set (without
/// consuming it); fails if there is none. Charges one unit of fuel per character examined
/// and can stop in the middle of a scan.
pub(crate) const SCAN: u32 = 21;

/// `STAR r min max len n body (lo hi){n}`: a quantifier over one character (`body`, a
/// `len`-word `CHAR`, `CLASS`, `DOT` or `ANY`, never executed itself), D1501. With
/// `F_CHECK`, the `n` ranges are the characters that can start the continuation: a
/// position followed by anything else is skipped without trying it. Register `r` counts
/// the iterations and `r + 1` holds the start position (both set before by `SET_REG` and
/// `SET_POS`). Greedy: matches as many characters as it can (resumably, one fuel unit
/// per character), then leaves one backtrack frame of kind `K_STAR` that gives back one
/// character per retry. Lazy: matches `min`, and each retry takes one more. Either way
/// the backtrack stack stays constant instead of growing by a frame per character.
pub(crate) const STAR: u32 = 22;

/// `SKIP r`: if register `r` (written by the leading `STAR` without the undo log, flag
/// `F_LEAD`) holds a position after the current one, moves there (D1501).
pub(crate) const SKIP: u32 = 23;
/// On a `STAR`: it starts the pattern; its run end goes into the skip register, the word
/// after its follow set.
pub(crate) const F_LEAD: u32 = 1 << 15;
/// On a `STAR` with a follow set: the end of the input can start the continuation too.
pub(crate) const F_END: u32 = 1 << 16;
/// On a greedy `STAR` with a follow set: no character the loop matches can start the
/// continuation, so giving characters back can never help (D1563).
pub(crate) const F_DISJOINT: u32 = 1 << 17;

/// Quantifier infinity.
const INF: u32 = u32::MAX;

/// What can start the rest of a match after some node (`Compiler::follow`): a character
/// in the set, or, with the flag, the end of the input.
type Follow = (CharSet, bool);

/// Whether a node can match the empty string: never, only at the end of the input (a
/// non-multiline `$`), or (as far as we know) anywhere.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Empty {
    No,
    AtEnd,
    Anywhere,
}

struct Compiler {
    code: Vec<u32>,
    reg_top: u32,
    reg_max: u32,
    /// The characters that can start what follows the node being compiled (forward
    /// direction), if that cannot match empty: `STAR` uses it to skip hopeless positions.
    follow: Option<Follow>,
    unicode: bool,
    /// The skip register while the leading `STAR` (see `SKIP`) is still to be compiled.
    lead: Option<u32>,
}

pub(crate) fn compile(parsed: &Parsed, flags: Flags) -> super::Program {
    // The words are copied into the heap afterwards: room for a typical program up
    // front saves the doubling reallocations, which showed in compile time.
    let mut words = crate::buf::vec_with_capacity(128);
    words.resize(H_LEN, 0u32);
    words[H_FLAGS] = flags.bits() as u32;
    words[H_CAPTURES] = parsed.capture_count;
    for name in &parsed.names[1..] {
        match name {
            Some(n) => {
                words.push_out(n.len() as u32);
                words.extend(n.iter().map(|&u| u as u32));
            }
            None => words.push_out(0),
        }
    }
    let mut c = Compiler {
        code: words,
        reg_top: 0,
        reg_max: 0,
        follow: None,
        unicode: flags.unicode_mode(),
        lead: None,
    };
    // A pattern that starts with a greedy unbounded loop over one character: when an
    // attempt fails, every start inside the run that loop consumed fails too, so the
    // search continues after the run (`SKIP`). A backreference could see the start
    // through group 0's neighbours, so patterns with one do without.
    let lead = leads_with_star(&parsed.node) && !has_backref(&parsed.node);
    let skip_reg = if lead { Some(c.alloc_regs(1)) } else { None };
    c.lead = skip_reg;
    // search: skip to a possible first character, try a match there; on failure advance
    // one character and retry.
    let search = c.here();
    c.code[H_SEARCH] = search;
    if let Some((first, false)) = first_chars(&parsed.node, flags.unicode_mode())
        && first.ranges() != [(0, charset::MAX_CP)]
    {
        c.class_op(SCAN, &first);
    }
    let split = c.emit2(SPLIT_GOTO, 0);
    if let Some(r) = skip_reg {
        c.emit2(SKIP, r);
    }
    c.emit(ANY);
    c.emit2(GOTO, search);
    let anchored = c.here();
    c.patch(split, anchored);
    c.code[H_ANCHORED] = anchored;
    c.emit2(SAVE, 0);
    c.node(&parsed.node, false);
    c.emit2(SAVE, 1);
    c.emit(MATCH);
    c.code[H_REGISTERS] = c.reg_max;
    super::Program { words: c.code }
}

/// The name of group `index` from a program's words.
pub(crate) fn group_name(words: &[u32], index: usize) -> Option<&[u32]> {
    let count = words[H_CAPTURES] as usize;
    if index == 0 || index >= count {
        return None;
    }
    let mut p = H_LEN;
    for i in 1..count {
        let len = words[p] as usize;
        if i == index {
            return (len > 0).then(|| &words[p + 1..p + 1 + len]);
        }
        p += 1 + len;
    }
    None
}

/// The input characters that can start a match of `node`, and whether it can match empty.
/// `None` if unknown (a backreference can start with anything). Assertions and lookarounds
/// are treated as matching empty, which over-approximates.
fn first_chars(node: &Node, unicode: bool) -> Option<(CharSet, bool)> {
    Some(match node {
        Node::Empty
        | Node::LineStart { .. }
        | Node::LineEnd { .. }
        | Node::WordBoundary { .. }
        | Node::Look { .. } => (CharSet::new(), true),
        Node::Char { c, icase } => {
            let mut s = CharSet::new();
            s.add(*c);
            (case_closure(s, *icase, unicode), false)
        }
        Node::Class {
            set,
            strings,
            icase,
        } => {
            let mut s = set.clone();
            for st in strings.iter().filter(|st| !st.is_empty()) {
                s.add_range(st[0], st[0]);
            }
            let nullable = strings.iter().any(|st| st.is_empty());
            (case_closure(s, *icase, unicode), nullable)
        }
        Node::Dot { all } => {
            let all_chars = CharSet::from_ranges(&[(0, charset::MAX_CP)]);
            if *all {
                (all_chars, false)
            } else {
                let lt = CharSet::from_ranges(&[(0x0A, 0x0A), (0x0D, 0x0D), (0x2028, 0x2029)]);
                (all_chars.subtract(&lt), false)
            }
        }
        Node::Backref { .. } => return None,
        Node::Group { body, .. } => first_chars(body, unicode)?,
        Node::Repeat { min, body, .. } => {
            let (f, n) = first_chars(body, unicode)?;
            (f, n || *min == 0)
        }
        Node::Concat(v) => {
            let mut acc = CharSet::new();
            for n in v {
                let (f, nullable) = first_chars(n, unicode)?;
                acc = acc.union(&f);
                if !nullable {
                    return Some((acc, false));
                }
            }
            (acc, true)
        }
        Node::Alt(v) => {
            let mut acc = CharSet::new();
            let mut nullable = false;
            for n in v {
                let (f, nn) = first_chars(n, unicode)?;
                acc = acc.union(&f);
                nullable |= nn;
            }
            (acc, nullable)
        }
    })
}

/// With `icase`, the input characters `c` with `canonicalize(c)` in `set`.
fn case_closure(set: CharSet, icase: bool, unicode: bool) -> CharSet {
    if icase {
        set.case_closure(unicode)
    } else {
        set
    }
}

/// `first_chars` with a finer notion of emptiness (for follow sets, D1563).
fn start_cond(node: &Node, unicode: bool) -> Option<(CharSet, Empty)> {
    Some(match node {
        Node::LineEnd { multiline: false } => (CharSet::new(), Empty::AtEnd),
        Node::Empty
        | Node::LineStart { .. }
        | Node::LineEnd { .. }
        | Node::WordBoundary { .. }
        | Node::Look { .. } => (CharSet::new(), Empty::Anywhere),
        Node::Char { .. } | Node::Class { .. } | Node::Dot { .. } => {
            let (f, nullable) = first_chars(node, unicode)?;
            (f, if nullable { Empty::Anywhere } else { Empty::No })
        }
        Node::Backref { .. } => return None,
        Node::Group { body, .. } => start_cond(body, unicode)?,
        Node::Repeat { min, body, .. } => {
            let (f, e) = start_cond(body, unicode)?;
            (f, if *min == 0 { Empty::Anywhere } else { e })
        }
        Node::Concat(v) => {
            let mut acc = CharSet::new();
            let mut empty = Empty::Anywhere;
            for n in v {
                let (f, e) = start_cond(n, unicode)?;
                acc = acc.union(&f);
                empty = empty.min(e);
                if e == Empty::No {
                    break;
                }
            }
            (acc, empty)
        }
        Node::Alt(v) => {
            let mut acc = CharSet::new();
            let mut empty = Empty::No;
            for n in v {
                let (f, e) = start_cond(n, unicode)?;
                acc = acc.union(&f);
                empty = empty.max(e);
            }
            (acc, empty)
        }
    })
}

/// A greedy, unbounded loop over one character, as the first thing a node matches.
fn leads_with_star(node: &Node) -> bool {
    match node {
        Node::Concat(v) => v.first().is_some_and(leads_with_star),
        Node::Repeat {
            max: INF,
            greedy: true,
            body,
            ..
        } => is_single_char(body),
        _ => false,
    }
}

fn has_backref(node: &Node) -> bool {
    match node {
        Node::Backref { .. } => true,
        Node::Group { body, .. } | Node::Look { body, .. } | Node::Repeat { body, .. } => {
            has_backref(body)
        }
        Node::Concat(v) | Node::Alt(v) => v.iter().any(has_backref),
        _ => false,
    }
}

/// Whether compiling `node` hands its follow set (`Compiler::follow`) to a `STAR`:
/// through groups, alternatives and concatenations, as `Compiler::node` passes it on.
fn wants_follow(node: &Node) -> bool {
    match node {
        Node::Repeat { body, .. } => is_single_char(body),
        Node::Group { body, .. } => wants_follow(body),
        Node::Concat(v) | Node::Alt(v) => v.iter().any(wants_follow),
        _ => false,
    }
}

/// A node that matches exactly one character (what `STAR` loops over).
fn is_single_char(node: &Node) -> bool {
    match node {
        Node::Char { .. } | Node::Dot { .. } => true,
        Node::Class { strings, .. } => strings.is_empty(),
        _ => false,
    }
}

/// The minimum number of characters `node` consumes (0 if it can match empty).
fn min_width(node: &Node) -> u32 {
    match node {
        Node::Char { .. } | Node::Dot { .. } => 1,
        Node::Class { strings, .. } => {
            let shortest = strings.iter().map(|s| s.len() as u32).min().unwrap_or(1);
            shortest.min(1)
        }
        Node::Group { body, .. } => min_width(body),
        Node::Repeat { min, body, .. } => min.saturating_mul(min_width(body)),
        Node::Concat(v) => v.iter().fold(0u32, |a, n| a.saturating_add(min_width(n))),
        Node::Alt(v) => v.iter().map(min_width).min().unwrap_or(0),
        _ => 0,
    }
}

impl Compiler {
    fn here(&self) -> u32 {
        self.code.len() as u32
    }

    fn emit(&mut self, op: u32) -> usize {
        self.code.push_out(op);
        self.code.len() - 1
    }

    /// Emits an instruction with one operand; returns the operand's index for patching.
    fn emit2(&mut self, op: u32, a: u32) -> usize {
        self.code.push_out(op);
        self.code.push_out(a);
        self.code.len() - 1
    }

    fn patch(&mut self, at: usize, target: u32) {
        self.code[at] = target;
    }

    fn alloc_regs(&mut self, n: u32) -> u32 {
        let r = self.reg_top;
        self.reg_top += n;
        self.reg_max = self.reg_max.max(self.reg_top);
        r
    }

    fn node(&mut self, node: &Node, back: bool) {
        let follow = self.follow.take();
        let dir = if back { F_BACK } else { 0 };
        let icase = |i: bool| if i { F_ICASE } else { 0 };
        match node {
            Node::Empty => {}
            Node::Char { c, icase: i } => {
                self.emit2(CHAR | dir | icase(*i), *c);
            }
            Node::Class {
                set,
                strings,
                icase: i,
            } => {
                if strings.is_empty() {
                    self.class(set, dir | icase(*i));
                } else {
                    self.class_with_strings(set, strings, *i, back);
                }
            }
            Node::Dot { all } => {
                self.emit(if *all { ANY } else { DOT } | dir);
            }
            Node::LineStart { multiline } => {
                self.emit(LINE_START | if *multiline { F_MULTILINE } else { 0 });
            }
            Node::LineEnd { multiline } => {
                self.emit(LINE_END | if *multiline { F_MULTILINE } else { 0 });
            }
            Node::WordBoundary {
                negate,
                unicode_icase,
            } => {
                self.emit(
                    WORD_BOUNDARY
                        | if *negate { F_NEG } else { 0 }
                        | if *unicode_icase { F_UICASE } else { 0 },
                );
            }
            Node::Group { index, body } => {
                let (first, second) = if back { (1, 0) } else { (0, 1) };
                self.emit2(SAVE, 2 * index + first);
                self.follow = follow;
                self.node(body, back);
                self.emit2(SAVE, 2 * index + second);
            }
            Node::Look {
                behind,
                negate,
                body,
            } => {
                let neg = if *negate { F_NEG } else { 0 };
                let cont = self.emit2(LOOK | neg, 0);
                self.node(body, *behind);
                self.emit(LOOK_END | neg);
                let here = self.here();
                self.patch(cont, here);
            }
            Node::Backref {
                groups, icase: i, ..
            } => {
                self.emit2(BACKREF | dir | icase(*i), groups.len() as u32);
                self.code.extend_from_slice(groups);
            }
            Node::Repeat { .. } => self.repeat(node, back, follow),
            Node::Concat(v) => {
                if back {
                    v.iter().rev().for_each(|n| self.node(n, back));
                } else if !v.iter().any(wants_follow) {
                    // No STAR to give a follow set to: skip computing them (they cost
                    // more than the rest of compiling a short pattern).
                    v.iter().for_each(|n| self.node(n, back));
                } else {
                    let follows = self.concat_follows(v, follow);
                    for (n, f) in v.iter().zip(follows) {
                        self.follow = f;
                        self.node(n, back);
                    }
                }
            }
            Node::Alt(v) => self.alternatives(v.iter(), back, follow),
        }
    }

    fn class(&mut self, set: &CharSet, flags: u32) {
        self.class_op(CLASS | flags, set);
    }

    fn class_op(&mut self, op: u32, set: &CharSet) {
        let ranges = set.ranges();
        self.emit2(op, ranges.len() as u32);
        for &(lo, hi) in ranges {
            self.code.push_out(lo);
            self.code.push_out(hi);
        }
    }

    /// The follow set of each element of a forward concatenation (see `follow`).
    fn concat_follows(&self, v: &[Node], outer: Option<Follow>) -> Vec<Option<Follow>> {
        let unicode = self.unicode;
        let mut out = alloc::vec![None; v.len()];
        // Only elements with a STAR in reach use their follow set, and the elements
        // before the first of them do not contribute to any.
        let first = v.iter().position(wants_follow).unwrap_or(v.len());
        let mut acc = outer;
        for i in (first..v.len()).rev() {
            // What can follow `v[i - 1]`, computed before `acc` moves into `out[i]`.
            let next = if i == first {
                None
            } else {
                match start_cond(&v[i], unicode) {
                    None => None,
                    Some((f, Empty::No)) => Some((f, false)),
                    // Empty only at the end, where no character follows: the rest must
                    // then accept the end too.
                    Some((f, Empty::AtEnd)) => Some((f, acc.as_ref().is_none_or(|(_, e)| *e))),
                    Some((f, Empty::Anywhere)) => acc.as_ref().map(|(a, e)| (f.union(a), *e)),
                }
            };
            if wants_follow(&v[i]) {
                out[i] = acc.filter(|(s, end)| !(*end && s.ranges() == [(0, charset::MAX_CP)]));
            }
            acc = next;
        }
        out
    }

    fn alternatives<'a>(
        &mut self,
        alts: impl ExactSizeIterator<Item = &'a Node>,
        back: bool,
        follow: Option<Follow>,
    ) {
        let n = alts.len();
        let mut gotos = Vec::new();
        for (i, alt) in alts.enumerate() {
            self.follow = follow.clone();
            if i + 1 < n {
                let split = self.emit2(SPLIT_NEXT, 0);
                self.node(alt, back);
                gotos.push_out(self.emit2(GOTO, 0));
                let next = self.here();
                self.patch(split, next);
            } else {
                self.node(alt, back);
            }
        }
        let end = self.here();
        for g in gotos {
            self.patch(g, end);
        }
    }

    /// A `v` class with strings: the strings longest first, then the single characters,
    /// then the empty string.
    fn class_with_strings(&mut self, set: &CharSet, strings: &[Vec<u32>], icase: bool, back: bool) {
        // Longest first (insertion sort: these lists are short, and it keeps the
        // standard library's sort code out of the binary).
        let mut sorted: Vec<&Vec<u32>> = Vec::new();
        for s in strings.iter().filter(|s| !s.is_empty()) {
            let at =
                sorted.partition_point(|t| t.len() > s.len() || (t.len() == s.len() && *t < s));
            sorted.insert(at, s);
        }
        let mut alts: Vec<Node> = sorted
            .iter()
            .map(|s| Node::Concat(s.iter().map(|&c| Node::Char { c, icase }).collect()))
            .collect();
        if !set.is_empty() {
            alts.push_out(Node::Class {
                set: set.clone(),
                strings: Vec::new(),
                icase,
            });
        }
        if strings.iter().any(|s| s.is_empty()) {
            alts.push_out(Node::Empty);
        }
        self.alternatives(alts.iter(), back, None);
    }

    /// Whether no character the one-character `body` matches is in `set` (without
    /// building the body's set where it is at hand).
    fn body_disjoint(&self, body: &Node, set: &CharSet) -> bool {
        match body {
            Node::Char { c, icase: false } => !set.contains(*c),
            Node::Class {
                set: chars,
                icase: false,
                ..
            } => chars.is_disjoint(set),
            _ => first_chars(body, self.unicode).is_some_and(|(b, _)| b.is_disjoint(set)),
        }
    }

    /// A `Node::Repeat`.
    fn repeat(&mut self, node: &Node, back: bool, follow: Option<Follow>) {
        let Node::Repeat {
            min,
            max,
            greedy,
            body,
            caps,
        } = node
        else {
            unreachable!("not a repeat")
        };
        let (min, max, greedy, caps) = (*min, *max, *greedy, *caps);
        if max == 0 {
            return;
        }
        if is_single_char(body) {
            let r = self.alloc_regs(2);
            self.emit2(SET_REG, r);
            self.code.push_out(0);
            self.emit2(SET_POS, r + 1);
            let follow = follow.filter(|_| !back);
            let mut check = match &follow {
                Some((_, true)) => F_CHECK | F_END,
                Some((_, false)) => F_CHECK,
                None => 0,
            };
            if greedy
                && let Some((f, _)) = &follow
                && self.body_disjoint(body, f)
            {
                check |= F_DISJOINT;
            }
            // The first STAR compiled is the leading one, if the pattern has one.
            let lead = if back { None } else { self.lead.take() };
            let flags = if back { F_BACK } else { 0 }
                | if greedy { F_GREEDY } else { 0 }
                | check
                | if lead.is_some() { F_LEAD } else { 0 };
            self.emit2(STAR | flags, r);
            self.code.push_out(min);
            self.code.push_out(max);
            let len_at = self.code.len();
            self.code.push_out(0);
            let follow_ranges = follow.as_ref().map_or(&[][..], |(f, _)| f.ranges());
            self.code.push_out(follow_ranges.len() as u32);
            let body_start = self.here();
            self.node(body, back);
            self.code[len_at] = self.here() - body_start;
            for &(lo, hi) in follow.as_ref().map_or(&[][..], |(f, _)| f.ranges()) {
                self.code.push_out(lo);
                self.code.push_out(hi);
            }
            if let Some(skip) = lead {
                self.code.push_out(skip);
            }
            self.reg_top -= 2;
            return;
        }
        let mut check = min_width(body) == 0;
        let g = if greedy { F_GREEDY } else { 0 };
        let split_op = if greedy { SPLIT_NEXT } else { SPLIT_GOTO };
        // Spec RepeatMatcher: the captures of the atom are cleared at each iteration.
        let reset = |c: &mut Compiler| {
            if caps.0 < caps.1 {
                c.emit(RESET);
                c.code.push_out(caps.0);
                c.code.push_out(caps.1 - 1);
            }
        };
        if min == 0 && (max == 1 || max == INF) {
            let top = self.here();
            let split = self.emit2(split_op, 0);
            let r = if check { self.alloc_regs(1) } else { 0 };
            if check {
                self.emit2(SET_POS, r);
            }
            reset(self);
            self.node(body, back);
            if check {
                self.emit2(CHECK_ADV, r);
                self.reg_top -= 1;
            }
            if max == INF {
                self.emit2(GOTO, top);
            }
            let end = self.here();
            self.patch(split, end);
            return;
        }
        if min == 1 && max == INF && !check {
            let top = self.here();
            reset(self);
            self.node(body, back);
            // Greedy: loop first, else exit first.
            self.emit2(if greedy { SPLIT_GOTO } else { SPLIT_NEXT }, top);
            return;
        }
        if min == max {
            check = false;
        }
        let split = (min == 0).then(|| self.emit2(split_op, 0));
        let nregs = if check { 2 } else { 1 };
        let r = self.alloc_regs(nregs);
        self.emit2(SET_REG, r);
        self.code.push_out(max);
        let top = self.here();
        if check {
            self.emit2(SET_POS, r + 1);
        }
        reset(self);
        self.node(body, back);
        if min == max {
            self.emit2(LOOP, r);
            self.code.push_out(top);
        } else {
            self.emit2(LOOP_SPLIT | g | if check { F_CHECK } else { 0 }, r);
            self.code.push_out(max - min);
            self.code.push_out(top);
        }
        self.reg_top -= nregs;
        if let Some(split) = split {
            let end = self.here();
            self.patch(split, end);
        }
    }
}
