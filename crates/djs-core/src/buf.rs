//! Container growth out of line (D2521).
//!
//! `Vec::push` inlines its capacity check and a call to the grow path at every site,
//! and the call takes the `Vec` by reference, so the vector is spilled to the shadow
//! stack around it even where it could live in locals. At ~370 sites that was ~13 KB of
//! wasm. `push_out` is one small function per element type instead: use it wherever a
//! push is not in a loop that the benchmarks show (D2521 lists the measurements).

use alloc::string::String;
use alloc::vec::Vec;

pub(crate) trait PushOut<T> {
    /// `push`, out of line.
    fn push_out(&mut self, x: T);
}

impl<T> PushOut<T> for Vec<T> {
    #[inline(never)]
    fn push_out(&mut self, x: T) {
        self.push(x)
    }
}

impl PushOut<char> for String {
    #[inline(never)]
    fn push_out(&mut self, c: char) {
        self.push(c)
    }
}
