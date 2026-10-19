#!/usr/bin/env python3
"""Extract the humor text tables from the Dart sp_league into data/*.json.
Handles: static const NAME = ['str', ...];  maps of lists;  records like (keys: [...], injury: [...], trip: [...]);
surprise-bill records (title: '..', body: '..'). Re-run if the Dart text changes."""
import json, re, sys
from pathlib import Path

SRC = Path.home() / "Documents/Source/Semi-ProFB/packages/sp_league/lib/src"
OUT = Path(__file__).resolve().parent.parent / "data"

ESC = {"n": "\n", "t": "\t", "r": "\r", "'": "'", '"': '"', "\\": "\\", "$": "$"}

def tokenize(text):
    i, n, toks = 0, len(text), []
    while i < n:
        c = text[i]
        if c.isspace():
            i += 1
        elif text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j
        elif text.startswith("/*", i):
            i = text.index("*/", i) + 2
        elif c in "'\"" or (c == "r" and i + 1 < n and text[i + 1] in "'\""):
            raw = c == "r"
            if raw:
                i += 1
            q = text[i]
            triple = text.startswith(q * 3, i)
            i += 3 if triple else 1
            buf = []
            while True:
                if triple and text.startswith(q * 3, i):
                    i += 3
                    break
                if not triple and text[i] == q:
                    i += 1
                    break
                if text[i] == "\\" and not raw:
                    e = text[i + 1]
                    if e == "u":
                        if text[i + 2] == "{":
                            j = text.index("}", i)
                            buf.append(chr(int(text[i + 3:j], 16)))
                            i = j + 1
                        else:
                            buf.append(chr(int(text[i + 2:i + 6], 16)))
                            i += 6
                    else:
                        buf.append(ESC.get(e, e))
                        i += 2
                else:
                    buf.append(text[i])
                    i += 1
            toks.append(("str", "".join(buf)))
        elif c.isalpha() or c == "_":
            j = i
            while j < n and (text[j].isalnum() or text[j] in "_$"):
                j += 1
            toks.append(("id", text[i:j]))
            i = j
        elif c.isdigit() or (c == "-" and i + 1 < n and text[i + 1].isdigit()):
            j = i + 1
            while j < n and (text[j].isdigit() or text[j] == "."):
                j += 1
            toks.append(("num", text[i:j]))
            i = j
        else:
            toks.append(("p", c))
            i += 1
    return toks

class P:
    def __init__(self, toks):
        self.t, self.i = toks, 0

    def peek(self):
        return self.t[self.i] if self.i < len(self.t) else ("eof", "")

    def nxt(self):
        v = self.t[self.i]
        self.i += 1
        return v

    def skip_generics(self):
        if self.peek() == ("p", "<"):
            d = 0
            while True:
                k = self.nxt()
                if k == ("p", "<"):
                    d += 1
                elif k == ("p", ">"):
                    d -= 1
                    if d == 0:
                        break

    def value(self):
        self.skip_generics()
        k = self.peek()
        if k[0] == "str":
            s = self.nxt()[1]
            while self.peek()[0] == "str":  # adjacent literals concatenate
                s += self.nxt()[1]
            return s
        if k == ("p", "["):
            self.nxt()
            out = []
            while self.peek() != ("p", "]"):
                out.append(self.value())
                if self.peek() == ("p", ","):
                    self.nxt()
            self.nxt()
            return out
        if k == ("p", "{"):
            self.nxt()
            out = {}
            while self.peek() != ("p", "}"):
                key = self.value()
                assert self.nxt() == ("p", ":")
                out[key] = self.value()
                if self.peek() == ("p", ","):
                    self.nxt()
            self.nxt()
            return out
        if k == ("p", "("):
            self.nxt()
            out = {}
            while self.peek() != ("p", ")"):
                name = self.nxt()[1]
                assert self.nxt() == ("p", ":"), name
                out[name] = self.value()
                if self.peek() == ("p", ","):
                    self.nxt()
            self.nxt()
            return out
        if k[0] == "num":
            v = self.nxt()[1]
            return float(v) if "." in v else int(v)
        raise SystemExit(f"unexpected token {k} at {self.i}")

def statics(path):
    toks = tokenize(Path(path).read_text())
    out, i = {}, 0
    while i < len(toks) - 3:
        if toks[i] == ("id", "static") and toks[i + 1][1] in ("const", "final") and toks[i + 2][0] == "id" and toks[i + 3] == ("p", "="):
            name = toks[i + 2][1]
            p = P(toks)
            p.i = i + 4
            try:
                out[name] = p.value()
            except (SystemExit, IndexError, AssertionError):
                pass  # not a data table (e.g. a computed final)
            i = p.i
        elif toks[i] == ("id", "static") and toks[i + 1][1] in ("const", "final") and toks[i + 2][0] == "id" and toks[i + 3][1] in ("<",):
            # typed: static const NAME = <T>[...]  is handled above; this is `static const Type name = ...`
            i += 1
        else:
            i += 1
    return out

inbox = statics(SRC / "inbox_content.dart")
gaz = statics(SRC / "gazette_content.dart")
# `static const NAME = <T>{...}` with a leading type before the name is handled by skip_generics on the value side.
bills_toks = tokenize((SRC / "surprise_bills.dart").read_text())
bills = statics(SRC / "surprise_bills.dart")  # welfare/bills are `static const welfare = <SurpriseBill>[ (title:..., body:...) ]`

def need(d, names, label):
    miss = [n for n in names if n not in d]
    if miss:
        raise SystemExit(f"{label} missing {miss}")

conduct = statics(SRC / "conduct.dart")
need(conduct, ["asks", "incidents"], "conduct")
(OUT / "conduct_text.json").write_text(json.dumps(dict(asks=conduct["asks"], incidents=conduct["incidents"]), indent=1, ensure_ascii=False))
print(f"conduct: asks={len(conduct['asks'])} incidents={len(conduct['incidents'])}")
need(inbox, ["reportWinBig", "jobCategories", "sponsorClauseEvents", "draftNone", "pleas", "councilReviewAll"], "inbox")
need(gaz, ["rivalryHead", "moneyDoomed", "gossipPaidHead"], "gazette")
need(bills, ["welfare", "bills"], "bills")
(OUT / "inbox_text.json").write_text(json.dumps(inbox, indent=1, ensure_ascii=False))
(OUT / "gazette_text.json").write_text(json.dumps(gaz, indent=1, ensure_ascii=False))
(OUT / "surprise_bills.json").write_text(json.dumps(bills, indent=1, ensure_ascii=False))
print(f"inbox: {len(inbox)} tables, {sum(len(v) if isinstance(v, list) else 0 for v in inbox.values())} lines; jobCategories={len(inbox['jobCategories'])}; clauseEvents={len(inbox['sponsorClauseEvents'])}")
print(f"gazette: {len(gaz)} tables, {sum(len(v) for v in gaz.values())} lines")
print(f"bills: welfare={len(bills['welfare'])} bills={len(bills['bills'])}")
