//! Construct temporaries (D2732): the state a construct keeps while its body runs (a
//! loop's iterator record, a switch's discriminant, a destructuring pattern's source, a
//! `using` list's resource Record) lives in a local of its own, not on the operand
//! stack: nothing stays on the operand stack across statements, in any function.
//!
//! Each is a binding of kind `VarKind::Temp` with an internal name (the integer atom of
//! the temporary's number in its function, which no source name can be and which needs
//! no interning), reached with `name_get`/`name_put` like any local.

use super::func::VarKind;
use super::parser::Parser;
use crate::heap::Atom;

impl Parser<'_, '_> {
    /// A new temporary, declared in the current scope.
    #[inline(never)]
    pub(super) fn temp(&mut self) -> Atom {
        // An integer atom: no source name is one, and it needs no interning.
        let atom = Atom::int(self.f.temps).unwrap_or(Atom::NULL);
        self.f.temps += 1;
        self.f.add_var(atom, VarKind::Temp, self.scope);
        atom
    }

    /// Pushes temporary `t`.
    pub(super) fn temp_get(&mut self, t: Atom) {
        self.temp_op(t, false);
    }

    /// Pops into temporary `t`.
    pub(super) fn temp_put(&mut self, t: Atom) {
        self.temp_op(t, true);
    }

    #[inline(never)]
    fn temp_op(&mut self, t: Atom, put: bool) {
        let r = self.name_ref(t, 0);
        if put {
            self.code().name_put(r);
        } else {
            self.code().name_get(r);
        }
    }
}
