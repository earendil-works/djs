//! Completions in temporaries (D2733, docs/compiler.md 7.4): how `break`, `continue`,
//! `return` and exceptions leave a finally block, a `using` list's disposal or a for-of
//! loop whose iterator must be closed.
//!
//! Each such block (a `BlockEnv` of kind `Finally` or `Iter`) is code of its own,
//! entered with a jump after its completion is stored in two temporaries:
//! `completion.kind@S` and `completion.value@S` (S the construct's offset: the `finally`
//! keyword, the `for`, the `using` list's first binding). The kind is an int:
//!
//! - 0: normal (the block ends by falling out of it),
//! - 1: throw (the value is the exception),
//! - 2: return (the value is the return value),
//! - 3 + 2·o: `break` to the statement at offset o from the function's start, 4 + 2·o:
//!   `continue` to it (a loop's keyword, a labeled statement's first label).
//!
//! The block's end dispatches on the kind: falls through to the code after it, rethrows,
//! or continues the exit, which may enter the next block out (the kind of a break stays
//! the same all the way). So a suspended frame keeps its completion in named slots
//! (D2720) and no return address: this replaced `gosub`/`ret` (D2627).
//!
//! The kind is written last, after the value, and only abrupt completions with a value
//! write the value: for the suspension maps' liveness, a write of the kind defines the
//! value too (compiler/temps.rs), and the rethrow clears it.
use crate::PushOut;

use alloc::vec::Vec;

use super::func::ScopeId;
use super::parser::{BlockKind, Parser};
use crate::bytecode::Label;
use crate::heap::Atom;

/// Kind codes of the completions that are no exit (see the module docs).
const NORMAL: i32 = 0;
const THROW: i32 = 1;
const RETURN: i32 = 2;

/// The completion of a `Finally` or `Iter` block.
pub(super) struct Completion {
    /// The scope its temporaries are declared in and the offset that names them.
    scope: ScopeId,
    at: u32,
    /// The temporaries, made on first use (an `Iter` block has none until a jump
    /// leaves the loop, and a value only for a `return`).
    kind: Option<Atom>,
    value: Option<Atom>,
    /// The exits that entered it, by kind code, in order of first use.
    exits: Vec<(Exit, i32)>,
}

/// Where an abrupt completion goes once the blocks it leaves have run.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Exit {
    /// Return the value on top of the stack.
    Return,
    /// Jump to `blocks[block]`'s break target (`cont`: its continue target).
    Jump { block: usize, cont: bool },
}

impl Completion {
    /// A completion whose temporaries are named by `at` (absolute), declared in `scope`.
    pub fn new(scope: ScopeId, at: u32) -> Completion {
        Completion {
            scope,
            at,
            kind: None,
            value: None,
            exits: Vec::new(),
        }
    }

    /// Whether an exit entered the block.
    pub fn has_exits(&self) -> bool {
        !self.exits.is_empty()
    }
}

impl Parser<'_, '_> {
    /// The kind temporary of `c` (and the value's with `value`), made on first use.
    fn completion_temps(&mut self, c: &mut Completion, value: bool) -> (Atom, Option<Atom>) {
        if c.kind.is_none() {
            c.kind = Some(self.temp_in("completion.kind", c.at, c.scope));
        }
        if value && c.value.is_none() {
            c.value = Some(self.temp_in("completion.value", c.at, c.scope));
        }
        (c.kind.expect("kind"), c.value)
    }

    /// Stores completion `code` in `c`; with `value` the value on top of the stack is
    /// its value.
    fn set_completion(&mut self, c: &mut Completion, code: i32, value: bool) {
        let (kind, val) = self.completion_temps(c, value);
        if value {
            self.temp_put(val.expect("a value"));
        }
        self.emit_number(f64::from(code));
        self.temp_put(kind);
    }

    /// The normal completion of `blocks[b]` (a `Finally` block); its code follows or is
    /// jumped to.
    pub(super) fn complete_normally(&mut self, b: usize) {
        let mut c = self.blocks[b].completion.take().expect("a completion");
        self.set_completion(&mut c, NORMAL, false);
        self.blocks[b].completion = Some(c);
    }

    /// An exception handler of a `Finally` block (its completion `c`, its code at
    /// `entry`), with the exception on the stack: enters the block with it.
    pub(super) fn complete_throwing(&mut self, c: &mut Completion, entry: Label) {
        self.set_completion(c, THROW, true);
        self.code().jump(entry);
    }

    /// Leaves the finally blocks and for-of loops of `self.blocks[from..]` for `exit`,
    /// innermost first. For [`Exit::Return`] the value is on top of the stack, above
    /// `self.stmt_depth` values. Enters the innermost block with the completion, whose
    /// end continues the exit ([`Parser::end_completion`]); without one, it is taken
    /// here.
    pub(super) fn emit_unwind(&mut self, from: usize, exit: Exit) {
        let inner = (from..self.blocks.len())
            .rev()
            .find(|&i| matches!(self.blocks[i].kind, BlockKind::Finally | BlockKind::Iter));
        let Some(b) = inner else {
            self.take_exit(exit);
            return;
        };
        let code = match exit {
            Exit::Return => RETURN,
            Exit::Jump { block, cont } => {
                let off = self.blocks[block].pos.saturating_sub(self.f.src_start);
                3 + 2 * off as i32 + i32::from(cont)
            }
        };
        let mut c = self.blocks[b].completion.take().expect("a completion");
        if exit == Exit::Return {
            // The value goes into the completion before the stack is cut.
            let (_, val) = self.completion_temps(&mut c, true);
            self.temp_put(val.expect("a value"));
        }
        for _ in self.blocks[b].depth..self.stmt_depth {
            self.code().drop();
        }
        self.set_completion(&mut c, code, false);
        let entry = self.blocks[b].break_label;
        self.code().jump(entry);
        if !c.exits.iter().any(|e| e.1 == code) {
            c.exits.push_out((exit, code));
        }
        self.blocks[b].completion = Some(c);
    }

    /// `exit` once no block is left to run: the jump (cutting the stack to the target's
    /// depth) or the return.
    fn take_exit(&mut self, exit: Exit) {
        match exit {
            Exit::Return => {
                let depth = self.stmt_depth;
                self.emit_return_value_at(depth);
            }
            Exit::Jump { block, cont } => {
                let b = &self.blocks[block];
                let (depth, target) = if cont {
                    (b.depth, b.continue_label.expect("a continue target"))
                } else {
                    (b.depth, b.break_label)
                };
                for _ in depth..self.stmt_depth {
                    self.code().drop();
                }
                self.code().jump(target);
            }
        }
    }

    /// The end of the completion block of `c` (its env popped): dispatch on the kind. A
    /// normal completion goes to `normal` (`None`: the block has none); with `throws`
    /// the block may have been entered by an exception, which is rethrown. The operand
    /// depth is the block's.
    pub(super) fn end_completion(
        &mut self,
        c: &mut Completion,
        normal: Option<Label>,
        throws: bool,
    ) {
        if c.kind.is_none() && !throws {
            return;
        }
        let (kind, _) = self.completion_temps(c, throws);
        if let Some(l) = normal {
            self.temp_get(kind);
            self.code().jump_if_false(l);
        }
        let exits = core::mem::take(&mut c.exits);
        let n = exits.len();
        for (i, &(exit, code)) in exits.iter().enumerate() {
            // The last exit needs no test when nothing else can be left.
            let l_next = self.new_label();
            if throws || i + 1 < n {
                self.temp_get(kind);
                self.emit_number(f64::from(code));
                self.code().strict_eq();
                self.code().jump_if_false(l_next);
            }
            let from = match exit {
                Exit::Return => {
                    self.temp_get(c.value.expect("a return value"));
                    0
                }
                Exit::Jump { block, .. } => block + 1,
            };
            self.emit_unwind(from, exit);
            self.place(l_next);
        }
        if throws {
            let v = c.value.expect("a thrown value");
            self.temp_get(v);
            self.code().undefined();
            self.temp_put(v);
            self.code().throw();
        }
    }
}
