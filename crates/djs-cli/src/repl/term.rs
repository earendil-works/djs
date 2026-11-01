//! A minimal line editor for the REPL, with no dependencies (D1640).
//!
//! The terminal is switched to non-canonical mode without echo and without signal keys
//! (`stty -icanon -echo -isig -iexten`), so the editor sees every key, and Ctrl-C arrives
//! as a byte instead of a SIGINT: std has no signal handling. One thread reads stdin
//! into a channel. The editor takes keys from it, and while code runs the REPL polls it
//! between fuel slices for Ctrl-C ([`Terminal::interrupted`]). Keys typed meanwhile are
//! kept for the next prompt. Output post-processing stays on, so `\n` still starts a new
//! line.
//!
//! Keys: printable text, Enter, Backspace, Delete, Left/Right, Home/End, Up/Down
//! (history), Ctrl-A/E/B/F/K/U/W/L/P/N, Alt-B/F, Ctrl-C (cancel the line), Ctrl-D (exit on
//! an empty line). Every character counts as one column; lines longer than the
//! terminal wrap and are redrawn imperfectly.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// What [`Terminal::read_line`] read.
pub enum Input {
    Line(String),
    /// Ctrl-C.
    Interrupt,
    /// Ctrl-D on an empty line, or stdin closed.
    Eof,
}

pub struct Terminal {
    /// `stty -g` from before, restored on drop.
    saved: String,
    rx: Receiver<u8>,
    /// Keys read while code ran, for the next prompt.
    pending: VecDeque<u8>,
}

fn stty(args: &[&str]) -> Option<String> {
    let out = Command::new("stty")
        .args(args)
        .stdin(Stdio::inherit())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

impl Terminal {
    /// Takes over the terminal; `None` if stdin is not a terminal `stty` can configure.
    pub fn new() -> Option<Terminal> {
        let saved = stty(&["-g"])?;
        stty(&[
            "-icanon", "-echo", "-isig", "-iexten", "min", "1", "time", "0",
        ])?;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let mut stdin = std::io::stdin().lock();
            let mut buf = [0u8; 256];
            loop {
                match stdin.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        for &b in &buf[..n] {
                            if tx.send(b).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });
        Some(Terminal {
            saved,
            rx,
            pending: VecDeque::new(),
        })
    }

    fn key(&mut self) -> Option<u8> {
        if let Some(b) = self.pending.pop_front() {
            return Some(b);
        }
        self.rx.recv().ok()
    }

    /// Ctrl-C was pressed since the last check. Other keys are kept for the next prompt.
    pub fn interrupted(&mut self) -> bool {
        let mut hit = false;
        loop {
            match self.rx.try_recv() {
                Ok(3) => hit = true,
                Ok(b) => self.pending.push_back(b),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return hit,
            }
        }
    }

    /// Reads one line with editing and history.
    /// Reads a line with editing. `complete` gives the completions for the text before
    /// the cursor: the length of the word it replaces (in characters) and the candidates.
    pub fn read_line(
        &mut self,
        prompt: &str,
        history: &[String],
        complete: &mut dyn FnMut(&str) -> (usize, Vec<String>),
    ) -> Input {
        let mut ed = Editor {
            prompt,
            line: Vec::new(),
            cursor: 0,
        };
        // Browsing history: the position, and the line being edited before.
        let mut hist = history.len();
        let mut draft: Vec<char> = Vec::new();
        ed.redraw();
        loop {
            let Some(b) = self.key() else {
                println!();
                return Input::Eof;
            };
            match b {
                b'\r' | b'\n' => {
                    ed.cursor = ed.line.len();
                    ed.redraw();
                    print!("\r\n");
                    let _ = std::io::stdout().flush();
                    return Input::Line(ed.line.iter().collect());
                }
                3 => {
                    print!("^C\r\n");
                    let _ = std::io::stdout().flush();
                    return Input::Interrupt;
                }
                4 if ed.line.is_empty() => {
                    print!("\r\n");
                    let _ = std::io::stdout().flush();
                    return Input::Eof;
                }
                4 => ed.delete(),
                0x7f | 8 => ed.backspace(),
                1 => ed.cursor = 0,
                5 => ed.cursor = ed.line.len(),
                2 => ed.cursor = ed.cursor.saturating_sub(1),
                6 => ed.cursor = (ed.cursor + 1).min(ed.line.len()),
                0x0b => ed.line.truncate(ed.cursor),
                0x15 => {
                    ed.line.drain(..ed.cursor);
                    ed.cursor = 0;
                }
                0x17 => {
                    let start = ed.word_left();
                    ed.line.drain(start..ed.cursor);
                    ed.cursor = start;
                }
                0x0c => print!("\x1b[H\x1b[2J"),
                0x10 => self.history_move(&mut ed, history, &mut hist, &mut draft, -1),
                0x0e => self.history_move(&mut ed, history, &mut hist, &mut draft, 1),
                b'\t' => {
                    let before: String = ed.line[..ed.cursor].iter().collect();
                    let (n, cands) = complete(&before);
                    let word: String = ed.line[ed.cursor - n.min(ed.cursor)..ed.cursor]
                        .iter()
                        .collect();
                    let prefix = common_prefix(&cands);
                    if cands.len() == 1 || prefix.chars().count() > n {
                        let with = if cands.len() == 1 { &cands[0] } else { &prefix };
                        ed.line.drain(ed.cursor - n..ed.cursor);
                        ed.cursor -= n;
                        with.chars().for_each(|c| ed.insert(c));
                    } else if cands.len() > 1 {
                        // Nothing to add: list them under the line.
                        print!("\r\n{}\r\n", cands.join("  "));
                    } else if word.is_empty() && cands.is_empty() {
                        ed.insert(' ');
                    }
                }
                0x1b => match self.key() {
                    Some(b'[' | b'O') => {
                        let mut param = Vec::new();
                        let fin = loop {
                            match self.key() {
                                Some(c @ 0x40..=0x7e) => break c,
                                Some(c) => param.push(c),
                                None => break 0,
                            }
                        };
                        match (fin, param.as_slice()) {
                            (b'A', _) => {
                                self.history_move(&mut ed, history, &mut hist, &mut draft, -1)
                            }
                            (b'B', _) => {
                                self.history_move(&mut ed, history, &mut hist, &mut draft, 1)
                            }
                            (b'C', _) => ed.cursor = (ed.cursor + 1).min(ed.line.len()),
                            (b'D', _) => ed.cursor = ed.cursor.saturating_sub(1),
                            (b'H', _) | (b'~', b"1" | b"7") => ed.cursor = 0,
                            (b'F', _) | (b'~', b"4" | b"8") => ed.cursor = ed.line.len(),
                            (b'~', b"3") => ed.delete(),
                            _ => {}
                        }
                    }
                    Some(b'b') => ed.cursor = ed.word_left(),
                    Some(b'f') => ed.cursor = ed.word_right(),
                    _ => {}
                },
                0x20..=0x7e => ed.insert(b as char),
                0x80.. => {
                    // A UTF-8 sequence: its length from the lead byte.
                    let n = match b {
                        0xc0..=0xdf => 1,
                        0xe0..=0xef => 2,
                        0xf0..=0xf7 => 3,
                        _ => 0,
                    };
                    let mut bytes = vec![b];
                    for _ in 0..n {
                        bytes.extend(self.key());
                    }
                    if let Ok(s) = std::str::from_utf8(&bytes) {
                        s.chars().for_each(|c| ed.insert(c));
                    }
                }
                _ => {}
            }
            ed.redraw();
        }
    }

    fn history_move(
        &mut self,
        ed: &mut Editor,
        history: &[String],
        hist: &mut usize,
        draft: &mut Vec<char>,
        dir: isize,
    ) {
        let next = *hist as isize + dir;
        if next < 0 || next as usize > history.len() {
            return;
        }
        if *hist == history.len() {
            *draft = ed.line.clone();
        }
        *hist = next as usize;
        ed.line = match history.get(*hist) {
            Some(h) => h.chars().collect(),
            None => draft.clone(),
        };
        ed.cursor = ed.line.len();
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = stty(&[&self.saved]);
    }
}

struct Editor<'a> {
    prompt: &'a str,
    line: Vec<char>,
    cursor: usize,
}

impl Editor<'_> {
    fn insert(&mut self, c: char) {
        self.line.insert(self.cursor, c);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.line.remove(self.cursor);
        }
    }

    fn delete(&mut self) {
        if self.cursor < self.line.len() {
            self.line.remove(self.cursor);
        }
    }

    fn word_left(&self) -> usize {
        let mut i = self.cursor;
        while i > 0 && !self.line[i - 1].is_alphanumeric() {
            i -= 1;
        }
        while i > 0 && self.line[i - 1].is_alphanumeric() {
            i -= 1;
        }
        i
    }

    fn word_right(&self) -> usize {
        let mut i = self.cursor;
        let n = self.line.len();
        while i < n && !self.line[i].is_alphanumeric() {
            i += 1;
        }
        while i < n && self.line[i].is_alphanumeric() {
            i += 1;
        }
        i
    }

    fn redraw(&self) {
        let text: String = self.line.iter().collect();
        let mut s = format!("\r{}{text}\x1b[K", self.prompt);
        let back = self.line.len() - self.cursor;
        if back > 0 {
            s.push_str(&format!("\x1b[{back}D"));
        }
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(s.as_bytes());
        let _ = out.flush();
    }
}

/// The longest common prefix of the candidates.
fn common_prefix(cands: &[String]) -> String {
    let Some(first) = cands.first() else {
        return String::new();
    };
    let mut n = first.len();
    for c in &cands[1..] {
        n = first
            .char_indices()
            .zip(c.chars())
            .take_while(|((_, a), b)| a == b)
            .map(|((i, a), _)| i + a.len_utf8())
            .last()
            .unwrap_or(0)
            .min(n);
    }
    first[..n].to_string()
}

#[cfg(test)]
mod tests {
    use super::common_prefix;

    #[test]
    fn common_prefixes() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(common_prefix(&s(&[])), "");
        assert_eq!(common_prefix(&s(&["max"])), "max");
        assert_eq!(common_prefix(&s(&["max", "maxi", "match"])), "ma");
        assert_eq!(common_prefix(&s(&["abc", "xyz"])), "");
        assert_eq!(common_prefix(&s(&["éa", "éb"])), "é");
    }
}
