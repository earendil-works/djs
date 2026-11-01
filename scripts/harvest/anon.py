"""Word-level anonymization for the codemode harvest (0118, D2281).

Every word part (a run of letters, split at camelCase and snake_case boundaries) of a
literal or identifier is kept only if it is a common lowercase English word (the system
dictionary), a JavaScript or shell term, or at most two letters long. Any other part is
replaced by letters of the same length and case pattern, consistently within a case, so
the script still behaves the same: the fake tools ignore their arguments, and a word
that appears twice is replaced the same way twice. A short denylist of names that are
ordinary words but name private services or people is always replaced.
"""

import hashlib
import os
import random
import re

_HERE = os.path.dirname(__file__)

PROGRAMMING = set("""
src lib cmd args arg str num idx fn cb err json html css utf http https url urls api cli npm npx
git gh rg jq js ts tsx jsx mjs cjs rs md dir dirs tmp env id ids ok obj arr len max min pos res
req ctx config async sync util utils readme todo fixme toml yaml yml node deno bun sed awk grep
wc ls cat echo printf mkdir cd rm mv cp xargs fd sh bash zsh stdout stderr stdin regex regexp
eof exit const let var async await typeof instanceof params param ret val vals func funcs init
impl struct enum mod pub usize isize println eprintln dev prod repo repos uri uuid timestamp
metadata subagent subagents todos info warn log logs debug png jpg jpeg svg gif webp pdf csv tsv
diff stat stats cwd pwd ssh http2 tcp udp dns ip ipv4 ipv6 lsof ps kill pid pids cpu gpu ram
utf8 ascii unicode stringify fetch href hostname pathname querystring sqlite postgres mysql
pnpm yarn vite vitest jest eslint prettier tsc rustc cargo clippy rustfmt wasm wasi inline
offset limit cancelled fulfilled rejected settled truncated nullish lookup lookups unhandled
""".split())

# Ordinary words that name private services, products or people in these sessions.
DENY = set("""
mitsuhiko armin ronacher earendil radius inkling ledger relay gondolin luna jev typesafe
""".split())


def _load_words():
    words = set()
    try:
        with open("/usr/share/dict/words") as f:
            for line in f:
                w = line.strip()
                if w.islower() and w.isalpha():
                    words.add(w)
    except OSError:
        pass
    with open(os.path.join(_HERE, "js-names.txt")) as f:
        for line in f:
            for part in split_parts(line.strip()):
                words.add(part.lower())
    return words


_PART_RE = re.compile(r"[A-Z]+(?![a-z])|[A-Z]?[a-z]+|[A-Z]+")


def _split_keep(word):
    """The parts of a letter run, joined back they give the run."""
    return _PART_RE.findall(word) or [word]


def split_parts(word):
    return _PART_RE.findall(word)


_WORDS = None
# An escape sequence (kept whole) or a run of letters.
_ESC_OR_WORD = re.compile(r"(\\.)|([A-Za-z]+)")


def known(part):
    global _WORDS
    if _WORDS is None:
        _WORDS = _load_words() | PROGRAMMING
    low = part.lower()
    if low in DENY:
        return False
    if len(part) <= 2 or low in _WORDS:
        return True
    # The dictionary has no inflections: try the stem.
    for suffix, repl in _SUFFIXES:
        if low.endswith(suffix) and len(low) - len(suffix) >= 3:
            stem = low[: -len(suffix)] + repl
            if stem in _WORDS and stem not in DENY:
                return True
    return False


_SUFFIXES = [
    ("ies", "y"), ("es", ""), ("s", ""), ("ed", ""), ("ed", "e"), ("ing", ""), ("ing", "e"),
    ("er", ""), ("er", "e"), ("ers", ""), ("ers", "e"), ("ly", ""), ("d", ""), ("ion", "e"),
    ("ions", "e"), ("ment", ""), ("ments", ""), ("able", ""), ("ness", ""),
]


class Anonymizer:
    def __init__(self, seed):
        self.seed = seed
        self.mapping = {}
        self.count = 0

    def _synthetic(self, part):
        low = part.lower()
        if low not in self.mapping:
            rng = random.Random(hashlib.sha256((self.seed + low).encode()).digest())
            # Consonant-vowel pairs read as made-up words and never hit a keyword.
            consonants, vowels = "bdfgklmnprstvz", "aeiou"
            new = "".join(rng.choice(consonants if i % 2 == 0 else vowels) for i in range(len(part)))
            if known(new):
                new = "q" + new[1:]
            self.mapping[low] = new
        new = self.mapping[low]
        return "".join(n.upper() if o.isupper() else n for o, n in zip(part, new))

    def word(self, word):
        """A letters-only run (an identifier part, or a word in text)."""
        out = []
        pos = 0
        for m in _PART_RE.finditer(word):
            out.append(word[pos : m.start()])
            part = m.group()
            if known(part):
                out.append(part)
            else:
                self.count += 1
                out.append(self._synthetic(part))
            pos = m.end()
        out.append(word[pos:])
        return "".join(out)

    def text(self, s):
        """Free text (string contents, template text): every letter run. A letter after a
        backslash is an escape (`\\n`, `\\u00e9`) and stays."""
        return _ESC_OR_WORD.sub(self._sub, s)

    def regex(self, lit):
        """A regular expression literal: the pattern's letter runs, not escapes or flags."""
        end = lit.rfind("/")
        body, flags = lit[1:end], lit[end + 1 :]
        body = _ESC_OR_WORD.sub(self._sub, body)
        return "/" + body + "/" + flags

    def _sub(self, m):
        return m.group(1) if m.group(1) is not None else self.word(m.group(2))

    def scrub(self, s):
        """Every letter run replaced, dictionary words too: for literals whose content
        only matters as data (tool arguments, shell commands). Escapes stay."""

        def sub(m):
            if m.group(1) is not None:
                return m.group(1)
            self.count += 1
            return "".join(self._synthetic(p) if len(p) > 2 else p for p in _split_keep(m.group(2)))

        return _ESC_OR_WORD.sub(sub, s)

    def identifier(self, name):
        return re.sub(r"[A-Za-z]+", lambda m: self.word(m.group()), name)
