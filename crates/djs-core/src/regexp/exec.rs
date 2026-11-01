//! The backtracking matcher.
//!
//! All state is integers: the program counter, the position, capture and register slots,
//! and a backtrack stack of `u32` words. The stack holds frames (`pc, pos, prev_bp << 2 |
//! kind, generation`, pushed at choice points and lookarounds) with an undo log of slot
//! writes above each frame. A slot is logged at most once per frame: `slot_gen` records the
//! generation of the frame that last logged it. Nothing lives on the Rust stack between
//! instructions, so running out of fuel at any instruction returns an [`ExecState`] that
//! [`resume`] continues exactly where it stopped.

use alloc::vec::Vec;

use super::Program;
use super::charset::{canonicalize, is_line_terminator, is_word_char};
use super::compiler::*;

/// Slot value of an undefined capture.
const UNSET: u32 = u32::MAX;

/// Frame kinds.
const K_SPLIT: u32 = 0;
const K_LOOK: u32 = 1;
const K_NEG_LOOK: u32 = 2;
const FRAME: usize = 4;

/// Binary search in sorted inclusive `(lo, hi)` pairs.
fn in_ranges(ranges: &[u32], c: u32) -> bool {
    let (mut lo, mut hi) = (0, ranges.len() / 2);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if c < ranges[2 * mid] {
            hi = mid;
        } else if c > ranges[2 * mid + 1] {
            lo = mid + 1;
        } else {
            return true;
        }
    }
    false
}

/// The string being matched.
#[derive(Clone, Copy, Debug)]
pub enum Input<'a> {
    Latin1(&'a [u8]),
    Utf16(&'a [u16]),
    /// UTF-16 as little-endian byte pairs: a two-byte heap string, read in place.
    Utf16Le(&'a [u8]),
}

impl Input<'_> {
    pub fn len(&self) -> usize {
        match self {
            Input::Latin1(s) => s.len(),
            Input::Utf16(s) => s.len(),
            Input::Utf16Le(s) => s.len() / 2,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn unit(&self, i: usize) -> u32 {
        match self {
            Input::Latin1(s) => s[i] as u32,
            Input::Utf16(s) => s[i] as u32,
            Input::Utf16Le(s) => u16::from_le_bytes([s[2 * i], s[2 * i + 1]]) as u32,
        }
    }
}

/// Resource limits for one run.
#[derive(Clone, Copy, Debug)]
pub struct ExecLimits {
    /// Maximum backtrack stack size in `u32` words.
    pub max_stack: usize,
}

impl Default for ExecLimits {
    fn default() -> ExecLimits {
        ExecLimits { max_stack: 1 << 20 }
    }
}

/// Capture positions of a match, as UTF-16 (or Latin-1) indices.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Captures {
    slots: Vec<u32>,
}

impl Captures {
    /// Number of groups, including group 0.
    pub fn len(&self) -> usize {
        self.slots.len() / 2
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The `(start, end)` of group `i`, or `None` if it did not participate.
    pub fn get(&self, i: usize) -> Option<(usize, usize)> {
        let (s, e) = (self.slots[2 * i], self.slots[2 * i + 1]);
        (s != UNSET && e != UNSET).then_some((s as usize, e as usize))
    }
}

pub enum ExecResult {
    Match(Captures),
    NoMatch,
    /// The fuel ran out; pass the state to [`resume`] (with the same program and input).
    OutOfFuel(ExecState),
    /// The backtrack stack would exceed [`ExecLimits::max_stack`].
    StackOverflow,
}

/// A suspended match.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExecState {
    pc: u32,
    pos: u32,
    bp: u32,
    next_gen: u32,
    slots: Vec<u32>,
    slot_gen: Vec<u32>,
    stack: Vec<u32>,
}

impl ExecState {
    /// The state as plain words, for storing it in the arena.
    pub fn to_words(&self) -> Vec<u32> {
        let mut w = alloc::vec![
            self.pc,
            self.pos,
            self.bp,
            self.next_gen,
            self.slots.len() as u32
        ];
        w.extend_from_slice(&self.slots);
        w.extend_from_slice(&self.slot_gen);
        w.extend_from_slice(&self.stack);
        w
    }

    /// Rebuilds a state from [`ExecState::to_words`].
    pub fn from_words(w: &[u32]) -> Option<ExecState> {
        let n = *w.get(4)? as usize;
        if w.len() < 5 + 2 * n {
            return None;
        }
        Some(ExecState {
            pc: w[0],
            pos: w[1],
            bp: w[2],
            next_gen: w[3],
            slots: w[5..5 + n].to_vec(),
            slot_gen: w[5 + n..5 + 2 * n].to_vec(),
            stack: w[5 + 2 * n..].to_vec(),
        })
    }
}

/// Runs `prog` on `input` starting at `start` (a UTF-16 index). Unless the program is
/// sticky, the match is searched for from `start` on. Each instruction costs one unit of
/// `fuel` (a backreference costs its length).
pub fn exec(
    prog: &Program,
    input: Input<'_>,
    start: usize,
    fuel: &mut u64,
    limits: ExecLimits,
) -> ExecResult {
    let w = prog.words();
    if start > input.len() {
        return ExecResult::NoMatch;
    }
    let unicode = prog.flags().unicode_mode();
    let mut start = start;
    if unicode && start > 0 && start < input.len() {
        // Starting inside a surrogate pair starts at the pair (RegExpBuiltinExec).
        if (0xDC00..=0xDFFF).contains(&input.unit(start))
            && (0xD800..=0xDBFF).contains(&input.unit(start - 1))
        {
            start -= 1;
        }
    }
    let nslots = 2 * w[H_CAPTURES] as usize + w[H_REGISTERS] as usize;
    let entry = if prog.flags().contains(super::Flags::STICKY) {
        w[H_ANCHORED]
    } else {
        w[H_SEARCH]
    };
    let state = ExecState {
        pc: entry,
        pos: start as u32,
        bp: 0,
        next_gen: 0,
        slots: alloc::vec![UNSET; nslots],
        slot_gen: alloc::vec![UNSET; nslots],
        stack: Vec::new(),
    };
    resume(prog, input, state, fuel, limits)
}

/// Continues a match suspended by running out of fuel.
pub fn resume(
    prog: &Program,
    input: Input<'_>,
    state: ExecState,
    fuel: &mut u64,
    limits: ExecLimits,
) -> ExecResult {
    let mut m = Machine {
        code: prog.words(),
        input,
        unicode: prog.flags().unicode_mode(),
        regs: 2 * prog.words()[H_CAPTURES] as usize,
        limits,
        s: state,
    };
    m.run(fuel)
}

struct Machine<'a> {
    code: &'a [u32],
    input: Input<'a>,
    unicode: bool,
    /// Index of the first register slot.
    regs: usize,
    limits: ExecLimits,
    s: ExecState,
}

/// Why an instruction did not continue normally.
enum Stop {
    Fail,
    Overflow,
}

impl Machine<'_> {
    fn next_char(&self, pos: usize) -> Option<(u32, usize)> {
        if pos >= self.input.len() {
            return None;
        }
        let c = self.input.unit(pos);
        if self.unicode && (0xD800..=0xDBFF).contains(&c) && pos + 1 < self.input.len() {
            let d = self.input.unit(pos + 1);
            if (0xDC00..=0xDFFF).contains(&d) {
                return Some((0x10000 + ((c - 0xD800) << 10) + (d - 0xDC00), pos + 2));
            }
        }
        Some((c, pos + 1))
    }

    fn prev_char(&self, pos: usize) -> Option<(u32, usize)> {
        if pos == 0 {
            return None;
        }
        let c = self.input.unit(pos - 1);
        if self.unicode && (0xDC00..=0xDFFF).contains(&c) && pos >= 2 {
            let h = self.input.unit(pos - 2);
            if (0xD800..=0xDBFF).contains(&h) {
                return Some((0x10000 + ((h - 0xD800) << 10) + (c - 0xDC00), pos - 2));
            }
        }
        Some((c, pos - 1))
    }

    /// Reads one character forward or backward from the current position and advances.
    fn read(&mut self, back: bool) -> Option<u32> {
        let pos = self.s.pos as usize;
        let (c, p) = if back {
            self.prev_char(pos)?
        } else {
            self.next_char(pos)?
        };
        self.s.pos = p as u32;
        Some(c)
    }

    fn push(&mut self, words: &[u32]) -> Result<(), Stop> {
        if self.s.stack.len() + words.len() > self.limits.max_stack {
            return Err(Stop::Overflow);
        }
        self.s.stack.extend_from_slice(words);
        Ok(())
    }

    fn push_frame(&mut self, pc: u32, kind: u32) -> Result<(), Stop> {
        let g = self.s.next_gen;
        self.s.next_gen = g.wrapping_add(1);
        self.push(&[pc, self.s.pos, self.s.bp << 2 | kind, g])?;
        self.s.bp = self.s.stack.len() as u32;
        Ok(())
    }

    fn write_slot(&mut self, slot: usize, value: u32) -> Result<(), Stop> {
        let bp = self.s.bp as usize;
        if bp > 0 {
            let g = self.s.stack[bp - 1];
            if self.s.slot_gen[slot] != g {
                let old = self.s.slots[slot];
                self.push(&[slot as u32, old])?;
                self.s.slot_gen[slot] = g;
            }
        }
        self.s.slots[slot] = value;
        Ok(())
    }

    /// Undoes the slot writes logged above the current frame.
    fn undo_to_frame(&mut self) {
        let bp = self.s.bp as usize;
        while self.s.stack.len() > bp {
            let n = self.s.stack.len();
            let (slot, old) = (self.s.stack[n - 2], self.s.stack[n - 1]);
            self.s.slots[slot as usize] = old;
            self.s.stack.truncate(n - 2);
        }
    }

    /// Pops the top frame (after undoing its log); returns its kind.
    fn pop_frame(&mut self) -> u32 {
        self.undo_to_frame();
        let f = self.s.bp as usize - FRAME;
        let st = &self.s.stack;
        self.s.pc = st[f];
        self.s.pos = st[f + 1];
        let kind = st[f + 2] & 3;
        self.s.bp = st[f + 2] >> 2;
        self.s.stack.truncate(f);
        kind
    }

    /// Backtracks to the most recent alternative. Returns false if there is none.
    fn backtrack(&mut self) -> bool {
        loop {
            if self.s.bp == 0 {
                return false;
            }
            // A failed positive lookahead keeps failing; a failed negative one succeeds
            // (its frame's pc is the continuation).
            if self.pop_frame() != K_LOOK {
                return true;
            }
        }
    }

    /// The body of a positive lookaround matched: drop the frames back to and including the
    /// lookaround's own, keeping their slot writes (moved under the frame below).
    fn look_succeeded(&mut self) {
        let mut tops = Vec::new(); // frame start indices, top first
        let mut bp = self.s.bp as usize;
        let look_start;
        loop {
            let f = bp - FRAME;
            let w = self.s.stack[f + 2];
            if w & 3 == K_LOOK {
                look_start = f;
                bp = (w >> 2) as usize;
                break;
            }
            tops.push(f);
            bp = (w >> 2) as usize;
        }
        // A lookaround does not consume input.
        self.s.pos = self.s.stack[look_start + 1];
        let st = &mut self.s.stack;
        let mut out = look_start;
        if bp > 0 {
            // Copy the logs: [look_start + FRAME, next frame), ... , [last frame + FRAME, len).
            let mut from = look_start + FRAME;
            let len = st.len();
            for &f in tops.iter().rev().chain(core::iter::once(&len)) {
                while from < f {
                    st[out] = st[from];
                    out += 1;
                    from += 1;
                }
                from = f + FRAME;
            }
        }
        st.truncate(out);
        self.s.bp = bp as u32;
    }

    /// The body of a negative lookaround matched: undo everything back to its frame, which
    /// is then dropped.
    fn neg_look_matched(&mut self) {
        while self.pop_frame() != K_NEG_LOOK {}
    }

    fn captures(&self) -> Captures {
        Captures {
            slots: self.s.slots[..self.regs].to_vec(),
        }
    }

    fn run(&mut self, fuel: &mut u64) -> ExecResult {
        loop {
            if *fuel == 0 {
                return ExecResult::OutOfFuel(core::mem::replace(
                    &mut self.s,
                    ExecState {
                        pc: 0,
                        pos: 0,
                        bp: 0,
                        next_gen: 0,
                        slots: Vec::new(),
                        slot_gen: Vec::new(),
                        stack: Vec::new(),
                    },
                ));
            }
            *fuel -= 1;
            match self.step(fuel) {
                Ok(true) => return ExecResult::Match(self.captures()),
                Ok(false) => {}
                Err(Stop::Fail) => {
                    if !self.backtrack() {
                        return ExecResult::NoMatch;
                    }
                }
                Err(Stop::Overflow) => return ExecResult::StackOverflow,
            }
        }
    }

    /// Executes one instruction. `Ok(true)` is a match.
    fn step(&mut self, fuel: &mut u64) -> Result<bool, Stop> {
        let code = self.code;
        let pc = self.s.pc as usize;
        let word = code[pc];
        let op = word & OP_MASK;
        let back = word & F_BACK != 0;
        let icase = word & F_ICASE != 0;
        let fail = Err(Stop::Fail);
        match op {
            MATCH => return Ok(true),
            CHAR => {
                let Some(mut c) = self.read(back) else {
                    return fail;
                };
                if icase {
                    c = canonicalize(c, self.unicode);
                }
                if c != code[pc + 1] {
                    return fail;
                }
                self.s.pc += 2;
            }
            CLASS => {
                let n = code[pc + 1] as usize;
                let Some(mut c) = self.read(back) else {
                    return fail;
                };
                if icase {
                    c = canonicalize(c, self.unicode);
                }
                if !in_ranges(&code[pc + 2..pc + 2 + 2 * n], c) {
                    return fail;
                }
                self.s.pc += 2 + 2 * n as u32;
            }
            DOT => {
                match self.read(back) {
                    Some(c) if !is_line_terminator(c) => {}
                    _ => return fail,
                }
                self.s.pc += 1;
            }
            ANY => {
                if self.read(back).is_none() {
                    return fail;
                }
                self.s.pc += 1;
            }
            LINE_START => {
                let pos = self.s.pos as usize;
                if pos > 0 {
                    let ok = word & F_MULTILINE != 0
                        && self
                            .prev_char(pos)
                            .is_some_and(|(c, _)| is_line_terminator(c));
                    if !ok {
                        return fail;
                    }
                }
                self.s.pc += 1;
            }
            LINE_END => {
                let pos = self.s.pos as usize;
                if pos < self.input.len() {
                    let ok = word & F_MULTILINE != 0
                        && self
                            .next_char(pos)
                            .is_some_and(|(c, _)| is_line_terminator(c));
                    if !ok {
                        return fail;
                    }
                }
                self.s.pc += 1;
            }
            WORD_BOUNDARY => {
                let pos = self.s.pos as usize;
                let ui = word & F_UICASE != 0;
                let a = self
                    .prev_char(pos)
                    .is_some_and(|(c, _)| is_word_char(c, ui));
                let b = self
                    .next_char(pos)
                    .is_some_and(|(c, _)| is_word_char(c, ui));
                if (a != b) == (word & F_NEG != 0) {
                    return fail;
                }
                self.s.pc += 1;
            }
            SCAN => {
                let n = code[pc + 1] as usize;
                let ranges = &code[pc + 2..pc + 2 + 2 * n];
                loop {
                    let pos = self.s.pos as usize;
                    let Some((c, next)) = self.next_char(pos) else {
                        return fail;
                    };
                    if in_ranges(ranges, c) {
                        self.s.pc += 2 + 2 * n as u32;
                        break;
                    }
                    self.s.pos = next as u32;
                    if *fuel == 0 {
                        // Out of fuel mid-scan: resume at this instruction.
                        break;
                    }
                    *fuel -= 1;
                }
            }
            GOTO => self.s.pc = code[pc + 1],
            SPLIT_NEXT => {
                self.push_frame(code[pc + 1], K_SPLIT)?;
                self.s.pc += 2;
            }
            SPLIT_GOTO => {
                self.push_frame(pc as u32 + 2, K_SPLIT)?;
                self.s.pc = code[pc + 1];
            }
            SAVE => {
                self.write_slot(code[pc + 1] as usize, self.s.pos)?;
                self.s.pc += 2;
            }
            RESET => {
                for g in code[pc + 1]..=code[pc + 2] {
                    self.write_slot(2 * g as usize, UNSET)?;
                    self.write_slot(2 * g as usize + 1, UNSET)?;
                }
                self.s.pc += 3;
            }
            BACKREF => {
                let n = code[pc + 1] as usize;
                let groups = &code[pc + 2..pc + 2 + n];
                let span = groups.iter().find_map(|&g| {
                    let (s, e) = (
                        self.s.slots[2 * g as usize],
                        self.s.slots[2 * g as usize + 1],
                    );
                    (s != UNSET && e != UNSET).then_some((s as usize, e as usize))
                });
                if let Some((s, e)) = span {
                    *fuel = fuel.saturating_sub((e - s) as u64);
                    if !self.match_backref(s, e, back, icase) {
                        return fail;
                    }
                }
                self.s.pc += 2 + n as u32;
            }
            LOOK => {
                let kind = if word & F_NEG != 0 {
                    K_NEG_LOOK
                } else {
                    K_LOOK
                };
                self.push_frame(code[pc + 1], kind)?;
                self.s.pc += 2;
            }
            LOOK_END => {
                if word & F_NEG != 0 {
                    self.neg_look_matched();
                    return fail;
                }
                self.look_succeeded();
                self.s.pc += 1;
            }
            SET_REG => {
                self.write_slot(self.regs + code[pc + 1] as usize, code[pc + 2])?;
                self.s.pc += 3;
            }
            SET_POS => {
                self.write_slot(self.regs + code[pc + 1] as usize, self.s.pos)?;
                self.s.pc += 2;
            }
            CHECK_ADV => {
                if self.s.slots[self.regs + code[pc + 1] as usize] == self.s.pos {
                    return fail;
                }
                self.s.pc += 2;
            }
            LOOP => {
                let r = self.regs + code[pc + 1] as usize;
                let v = self.s.slots[r].wrapping_sub(1);
                self.write_slot(r, v)?;
                self.s.pc = if v != 0 { code[pc + 2] } else { pc as u32 + 3 };
            }
            LOOP_SPLIT => {
                let r = self.regs + code[pc + 1] as usize;
                let limit = code[pc + 2];
                let target = code[pc + 3];
                let next = pc as u32 + 4;
                let v = self.s.slots[r].wrapping_sub(1);
                self.write_slot(r, v)?;
                if v > limit {
                    // Still below the minimum: iterate.
                    self.s.pc = target;
                } else {
                    if word & F_CHECK != 0 && self.s.slots[r + 1] == self.s.pos && v != limit {
                        // An optional iteration that matched empty.
                        return fail;
                    }
                    if v == 0 {
                        self.s.pc = next;
                    } else if word & F_GREEDY != 0 {
                        self.push_frame(next, K_SPLIT)?;
                        self.s.pc = target;
                    } else {
                        self.push_frame(target, K_SPLIT)?;
                        self.s.pc = next;
                    }
                }
            }
            _ => unreachable!("bad regexp opcode {op}"),
        }
        Ok(false)
    }

    /// Matches the text `[s, e)` at the current position (backward: ending there).
    fn match_backref(&mut self, s: usize, e: usize, back: bool, icase: bool) -> bool {
        let canon = |c: u32| {
            if icase {
                canonicalize(c, self.unicode)
            } else {
                c
            }
        };
        if !back {
            let mut p = s;
            let mut pos = self.s.pos as usize;
            while p < e {
                // Characters of the capture, limited to the capture's end.
                let (c1, np) = match self.next_char(p) {
                    Some((c, np)) if np <= e => (c, np),
                    _ => (self.input.unit(p), p + 1),
                };
                let Some((c2, npos)) = self.next_char(pos) else {
                    return false;
                };
                if canon(c1) != canon(c2) {
                    return false;
                }
                p = np;
                pos = npos;
            }
            self.s.pos = pos as u32;
        } else {
            let mut p = e;
            let mut pos = self.s.pos as usize;
            while p > s {
                let (c1, np) = match self.prev_char(p) {
                    Some((c, np)) if np >= s => (c, np),
                    _ => (self.input.unit(p - 1), p - 1),
                };
                let Some((c2, npos)) = self.prev_char(pos) else {
                    return false;
                };
                if canon(c1) != canon(c2) {
                    return false;
                }
                p = np;
                pos = npos;
            }
            self.s.pos = pos as u32;
        }
        true
    }
}
