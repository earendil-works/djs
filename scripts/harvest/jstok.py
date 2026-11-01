"""A small JavaScript tokenizer for the codemode harvest (0118): enough to tell code from
comments, string literals, template literal text and regular expression literals, so
that redaction touches only literal text and comments can be dropped.

Tokens are (kind, text) with kind one of: ws, comment, string, template (one chunk of a
template literal's text, with its delimiters), regex, word, number, punct. Joining the
texts of all tokens gives back the source exactly.
"""

import re

_WORD = re.compile(r"[A-Za-z_$\u00a0-\uffff][\w$\u00a0-\uffff]*")
_NUMBER = re.compile(r"(?:0[xXoObB][0-9a-fA-F_]+|(?:\d[\d_]*\.?[\d_]*|\.\d[\d_]*)(?:[eE][+-]?\d+)?)n?")
_WS = re.compile(r"\s+")
# After these words a `/` starts a regular expression, not a division.
_REGEX_AFTER_WORDS = {
    "return", "typeof", "instanceof", "in", "of", "new", "delete", "void", "throw",
    "case", "do", "else", "yield", "await",
}
_PUNCT = [
    ">>>=", "...", "===", "!==", "**=", "<<=", ">>=", ">>>", "&&=", "||=", "??=",
    "=>", "==", "!=", "<=", ">=", "&&", "||", "??", "?.", "++", "--", "+=", "-=", "*=",
    "/=", "%=", "&=", "|=", "^=", "**", "<<", ">>",
]


class TokenizeError(Exception):
    pass


def tokenize(src):
    tokens = []
    i = 0
    n = len(src)
    # Brace depths at which an open template literal continues: `${` pushes.
    template_stack = []
    brace_depth = 0

    def prev_significant():
        for kind, text in reversed(tokens):
            if kind not in ("ws", "comment"):
                return kind, text
        return None, None

    def regex_allowed():
        kind, text = prev_significant()
        if kind is None:
            return True
        if kind in ("number", "string", "regex"):
            return False
        if kind == "template":
            return text.endswith("${")
        if kind == "word":
            return text in _REGEX_AFTER_WORDS
        if kind == "punct":
            return text not in (")", "]", "}")
        return True

    def read_template_chunk(start):
        # From `start` (just after a backquote or a closing `}` of `${`), up to the
        # closing backquote (inclusive) or a `${` (inclusive).
        j = start
        while j < n:
            c = src[j]
            if c == "\\":
                j += 2
                continue
            if c == "`":
                return j + 1, False
            if c == "$" and j + 1 < n and src[j + 1] == "{":
                return j + 2, True
            j += 1
        raise TokenizeError("unterminated template literal")

    while i < n:
        c = src[i]
        m = _WS.match(src, i)
        if m:
            tokens.append(("ws", m.group()))
            i = m.end()
            continue
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            tokens.append(("comment", src[i:j]))
            i = j
            continue
        if src.startswith("/*", i):
            j = src.find("*/", i + 2)
            if j < 0:
                raise TokenizeError("unterminated comment")
            tokens.append(("comment", src[i : j + 2]))
            i = j + 2
            continue
        if c in "'\"":
            j = i + 1
            while j < n and src[j] != c:
                if src[j] == "\\":
                    j += 1
                elif src[j] == "\n":
                    raise TokenizeError("newline in string")
                j += 1
            if j >= n:
                raise TokenizeError("unterminated string")
            tokens.append(("string", src[i : j + 1]))
            i = j + 1
            continue
        if c == "`":
            j, opened = read_template_chunk(i + 1)
            tokens.append(("template", src[i:j]))
            if opened:
                template_stack.append(brace_depth)
                brace_depth += 1
            i = j
            continue
        if c == "}" and template_stack and template_stack[-1] == brace_depth - 1:
            brace_depth -= 1
            template_stack.pop()
            j, opened = read_template_chunk(i + 1)
            tokens.append(("template", src[i:j]))
            if opened:
                template_stack.append(brace_depth)
                brace_depth += 1
            i = j
            continue
        if c == "/" and regex_allowed():
            j = i + 1
            in_class = False
            while j < n:
                d = src[j]
                if d == "\\":
                    j += 2
                    continue
                if d == "\n":
                    raise TokenizeError("newline in regex")
                if d == "[":
                    in_class = True
                elif d == "]":
                    in_class = False
                elif d == "/" and not in_class:
                    break
                j += 1
            if j >= n:
                raise TokenizeError("unterminated regex")
            j += 1
            while j < n and (src[j].isalnum() or src[j] == "_"):
                j += 1
            tokens.append(("regex", src[i:j]))
            i = j
            continue
        m = _WORD.match(src, i)
        if m:
            tokens.append(("word", m.group()))
            i = m.end()
            continue
        if c.isdigit() or (c == "." and i + 1 < n and src[i + 1].isdigit()):
            m = _NUMBER.match(src, i)
            tokens.append(("number", m.group()))
            i = m.end()
            continue
        for p in _PUNCT:
            if src.startswith(p, i):
                tokens.append(("punct", p))
                i += len(p)
                break
        else:
            if c == "{":
                brace_depth += 1
            elif c == "}":
                brace_depth -= 1
            tokens.append(("punct", c))
            i += 1
    if template_stack:
        raise TokenizeError("unterminated template substitution")
    assert "".join(t for _, t in tokens) == src
    return tokens
