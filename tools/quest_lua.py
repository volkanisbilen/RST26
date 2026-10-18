"""Tiny Lua 5.1 lexer / call + event-block extractor used by tools/quest_audit.py.

It is NOT a full parser: it understands just enough of the shipped quest scripts
(comments, strings, numbers, identifiers, keywords, block nesting) to answer
"which functions are called with which arguments inside which `EVENT == N`
handler".
"""
from __future__ import annotations

import bisect
import re
from dataclasses import dataclass, field

KEYWORDS = {
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function",
    "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true",
    "until", "while",
}

_NUM_RE = re.compile(r"0[xX][0-9a-fA-F]+|\d+\.?\d*(?:[eE][+-]?\d+)?|\.\d+(?:[eE][+-]?\d+)?")
_ID_RE = re.compile(r"[A-Za-z_][A-Za-z_0-9]*")
_LONG_OPEN_RE = re.compile(r"\[(=*)\[")


@dataclass
class Tok:
    kind: str  # id | kw | num | str | op | eof
    val: str
    line: int


def tokenize(src: str) -> list[Tok]:
    if src.startswith("﻿"):
        src = src[1:]
    toks: list[Tok] = []
    i, n, line = 0, len(src), 1
    while i < n:
        c = src[i]
        if c == "\n":
            line += 1
            i += 1
            continue
        if c in " \t\r":
            i += 1
            continue
        if src.startswith("--", i):
            m = _LONG_OPEN_RE.match(src, i + 2)
            if m:
                close = "]" + m.group(1) + "]"
                j = src.find(close, m.end())
                j = n if j < 0 else j + len(close)
                line += src.count("\n", i, j)
                i = j
            else:
                j = src.find("\n", i)
                i = n if j < 0 else j
            continue
        if c in "\"'":
            j = i + 1
            while j < n and src[j] != c:
                if src[j] == "\\":
                    j += 1
                if j < n and src[j] == "\n":
                    line += 1
                j += 1
            toks.append(Tok("str", src[i + 1:j], line))
            i = j + 1
            continue
        m = _LONG_OPEN_RE.match(src, i)
        if m:
            close = "]" + m.group(1) + "]"
            j = src.find(close, m.end())
            j = n if j < 0 else j + len(close)
            toks.append(Tok("str", src[i:j], line))
            line += src.count("\n", i, j)
            i = j
            continue
        if c.isdigit() or (c == "." and i + 1 < n and src[i + 1].isdigit()):
            m = _NUM_RE.match(src, i)
            toks.append(Tok("num", m.group(0), line))
            i = m.end()
            continue
        m = _ID_RE.match(src, i)
        if m:
            w = m.group(0)
            toks.append(Tok("kw" if w in KEYWORDS else "id", w, line))
            i = m.end()
            continue
        for op in ("...", "..", "==", "~=", "<=", ">="):
            if src.startswith(op, i):
                toks.append(Tok("op", op, line))
                i += len(op)
                break
        else:
            toks.append(Tok("op", c, line))
            i += 1
    toks.append(Tok("eof", "", line))
    return toks


def to_int(text: str):
    try:
        if text.lower().startswith("0x"):
            return int(text, 16)
        f = float(text)
        return int(f) if f == int(f) else None
    except ValueError:
        return None


@dataclass
class Call:
    name: str
    args: list[list[Tok]]
    line: int
    pos: int  # token index of the function name
    event: int | None = None  # innermost top-level EVENT handler this call sits in

    def arg_text(self, i: int) -> str:
        return " ".join(t.val for t in self.args[i]) if i < len(self.args) else ""

    def arg_const(self, i: int, env: dict[str, set[int]]):
        """Resolve argument i to a set of ints (literal or a numeric variable), else None."""
        if i >= len(self.args):
            return None
        a = self.args[i]
        if len(a) == 1 and a[0].kind == "num":
            v = to_int(a[0].val)
            return {v} if v is not None else None
        if len(a) == 2 and a[0].val == "-" and a[1].kind == "num":
            v = to_int(a[1].val)
            return {-v} if v is not None else None
        if len(a) == 1 and a[0].kind == "id" and a[0].val in env:
            return set(env[a[0].val])
        return None


@dataclass
class Handler:
    """A top-level `if (EVENT == N [or EVENT == M]) then ... end` block."""
    events: list[int]
    start: int  # token index of `if`
    end: int  # token index of matching `end`
    line: int
    calls: list[Call] = field(default_factory=list)
    reassign: list[int] = field(default_factory=list)  # `EVENT = X` inside


@dataclass
class LuaScript:
    name: str
    toks: list[Tok]
    calls: list[Call]
    handlers: list[Handler]
    env: dict[str, set[int]]
    npc_const: int | None


def _matching_end(toks: list[Tok], start: int) -> int:
    """Index of the `end` closing the block opened by toks[start] (`if`/`function`/`for`/`while`/`do`)."""
    depth = 0
    pending_do = 0
    i = start
    n = len(toks)
    while i < n:
        t = toks[i]
        if t.kind == "kw":
            v = t.val
            if v in ("if", "function"):
                depth += 1
            elif v in ("for", "while"):
                depth += 1
                pending_do += 1
            elif v == "do":
                if pending_do:
                    pending_do -= 1
                else:
                    depth += 1
            elif v == "repeat":
                depth += 1
            elif v == "until":
                depth -= 1
                if depth == 0:
                    return i
            elif v == "end":
                depth -= 1
                if depth == 0:
                    return i
        i += 1
    return n - 1


def _split_args(toks: list[Tok], open_idx: int):
    """Return (args, close_idx) for the call whose `(` is toks[open_idx]."""
    depth = 0
    args: list[list[Tok]] = []
    cur: list[Tok] = []
    i = open_idx
    n = len(toks)
    while i < n:
        t = toks[i]
        if t.kind == "op" and t.val in "([{":
            depth += 1
            if depth > 1:
                cur.append(t)
        elif t.kind == "op" and t.val in ")]}":
            depth -= 1
            if depth == 0:
                if cur or args:
                    args.append(cur)
                return args, i
            cur.append(t)
        elif t.kind == "op" and t.val == "," and depth == 1:
            args.append(cur)
            cur = []
        else:
            cur.append(t)
        i += 1
    return args, n - 1


def _event_values(toks: list[Tok], lo: int, hi: int):
    """Event numbers from a condition like `(EVENT == 1 or EVENT == 2)` in toks[lo:hi]."""
    vals = []
    for k in range(lo, hi - 2):
        if toks[k].kind == "id" and toks[k].val == "EVENT" and toks[k + 1].val == "==" and toks[k + 2].kind == "num":
            v = to_int(toks[k + 2].val)
            if v is not None:
                vals.append(v)
    return vals


def parse(name: str, src: str) -> LuaScript:
    toks = tokenize(src)
    calls: list[Call] = []
    n = len(toks)
    for i in range(n - 1):
        t = toks[i]
        if t.kind == "id" and toks[i + 1].kind == "op" and toks[i + 1].val == "(":
            # skip `function name(` definitions and method/field access like a.b(
            prev = toks[i - 1] if i else None
            if prev is not None and prev.kind == "kw" and prev.val == "function":
                continue
            if prev is not None and prev.kind == "op" and prev.val in (".", ":"):
                continue
            args, _ = _split_args(toks, i + 1)
            calls.append(Call(t.val, args, t.line, i))

    # numeric constants (`NAME = 123`), used to resolve variables passed to bindings
    env: dict[str, set[int]] = {}
    for i in range(n - 2):
        if (toks[i].kind == "id" and toks[i + 1].kind == "op" and toks[i + 1].val == "="
                and toks[i + 2].kind == "num"
                and not (i and toks[i - 1].kind == "op" and toks[i - 1].val in ("=", "<", ">", "~", "!"))):
            nxt = toks[i + 3] if i + 3 < n else None
            if nxt is not None and nxt.kind in ("num", "id") and nxt.line == toks[i + 2].line and not (nxt.kind == "id" and nxt.val in ()):
                # `A = 1 B ...` is fine; `A = 1 + x` is not a constant
                pass
            if nxt is not None and nxt.kind == "op" and nxt.val in ("+", "-", "*", "/", "%", "..", "^"):
                continue
            v = to_int(toks[i + 2].val)
            if v is not None:
                env.setdefault(toks[i].val, set()).add(v)

    # top-level EVENT handlers
    handlers: list[Handler] = []
    i = 0
    while i < n:
        t = toks[i]
        if t.kind == "kw" and t.val == "if":
            end = _matching_end(toks, i)
            # condition = tokens up to the first `then` at this level
            j = i + 1
            while j < end and not (toks[j].kind == "kw" and toks[j].val == "then"):
                j += 1
            evs = _event_values(toks, i + 1, j)
            # `elseif (EVENT == N) then` clauses belonging to this chain (depth 1)
            depth = 0
            pend = 0
            for k in range(i, end + 1):
                tk = toks[k]
                if tk.kind != "kw":
                    continue
                if tk.val in ("if", "function", "repeat"):
                    depth += 1
                elif tk.val in ("for", "while"):
                    depth += 1
                    pend += 1
                elif tk.val == "do":
                    if pend:
                        pend -= 1
                    else:
                        depth += 1
                elif tk.val in ("end", "until"):
                    depth -= 1
                elif tk.val == "elseif" and depth == 1:
                    m = k + 1
                    while m < end and not (toks[m].kind == "kw" and toks[m].val == "then"):
                        m += 1
                    evs += _event_values(toks, k + 1, m)
            if evs:
                h = Handler(evs, i, end, t.line)
                handlers.append(h)
            i = end + 1
            continue
        if t.kind == "kw" and t.val in ("function", "for", "while", "do", "repeat"):
            i = _matching_end(toks, i) + 1
            continue
        i += 1
    # attach calls and EVENT reassignments to handlers
    positions = [c.pos for c in calls]
    for h in handlers:
        lo = bisect.bisect_right(positions, h.start)
        hi = bisect.bisect_left(positions, h.end)
        for c in calls[lo:hi]:
            c.event = h.events[0]
            h.calls.append(c)
        for k in range(h.start, h.end - 1):
            if (toks[k].kind == "id" and toks[k].val == "EVENT" and toks[k + 1].val == "="
                    and toks[k + 2].kind == "num" and not (k and toks[k - 1].val in ("=", "<", ">", "~"))):
                v = to_int(toks[k + 2].val)
                if v is not None:
                    h.reassign.append(v)

    npc_const = None
    if "NPC" in env and len(env["NPC"]) == 1:
        npc_const = next(iter(env["NPC"]))
    return LuaScript(name, toks, calls, handlers, env, npc_const)
