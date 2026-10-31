#!/usr/bin/env python3
# =============================================================================
# EH3D-DRILLPRO - tools/make_hotkeys.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""Writes the hot-key marks `~X~` into the files of the languages that are read from `locales/`.

Every label that has a mark in English gets one letter in each language: the first letter of its word that no other label of the same
screen already uses (Latin, Cyrillic or Greek, in the alphabet of the text). The letters are chosen so that no two controls of a screen
share one, the way the eight main languages do, and so that the keys the teach window keeps for the arrows stay free.

Typing: a Latin keyboard types a Latin letter; the program reads it as the letter that sits on that key in the standard Russian, Ukrainian or
Greek layout (`Lang::hot_key` in src/i18n.rs, the same table as LAYOUTS below), so a hot letter of those alphabets can be reached from either
keyboard. The other Cyrillic languages (Bulgarian, Serbian) need their own layout for the Cyrillic letters. A Greek accent is ignored.

The tool is repeatable: it removes the marks it finds and writes them again. `--check` writes nothing and fails if a file would change.
Run it with `py -3 -I tools/make_hotkeys.py` from the root of the repository.
"""
from __future__ import annotations

import re
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# the 8 languages of the program, in the order of the rows of the tables of src/**/texts.rs
CORE = ["en", "es", "it", "de", "fr", "pt", "zh", "ja"]
VARIANT_OF = {"pt-br": "pt", "fr-ca": "fr"}

# the Latin key and the letter on it in the standard layout of the language (the same pairs as src/i18n.rs)
LAYOUTS = {
    "ru": "qйwцeуrкtеyнuгiшoщpзaфsыdвfаgпhрjоkлlдzяxчcсvмbиnтmь",
    "uk": "qйwцeуrкtеyнuгiшoщpзaфsіdвfаgпhрjоkлlдzяxчcсvмbиnтmь",
    "el": "eεrρtτyυuθiιoοpπaασsσdδfφgγhηjξkκlλzζxχcψvωbβnνmμ",
}
GREEK_ACCENT = {"ά": "α", "έ": "ε", "ή": "η", "ί": "ι", "ϊ": "ι", "ΐ": "ι", "ό": "ο", "ύ": "υ", "ϋ": "υ", "ΰ": "υ", "ώ": "ω", "ς": "σ"}
# the keys of the teach window that mean a place (the arrows), so no hot letter of that window may use them
TEACH_FIXED = "esdxrc"

MAIN = ["LblScale", "LblOffset", "LblClip", "LblVectors", "LblBoard", "LblBase", "BtnConfig", "BtnTools", "BtnStart", "BtnLoad",
        "BtnPanelz", "BtnOffset", "BtnQuit", "BtnAbout"]
PARAMETER = ["PFile", "PCoordinates", "PSorting", "PSpeed", "PRamp", "PStroke", "PCorrect", "PSpindle", "PLanguage", "BLoad", "BOk", "BAbort"]
# the controls that share a screen: (name, texts, keys that the screen keeps for itself)
CLASSIC_SCREENS = [
    ("main", MAIN, ""),
    ("parameter", PARAMETER, ""),
    ("parameter_route", [("PChord" if k == "PSorting" else k) for k in PARAMETER], ""),
    ("tools", ["BLoad", "BOk", "BAbort"], ""),
    ("panel", ["PzCalc", "BOk", "BAbort"], ""),
    ("files", ["FName", "FFiles", "BOkUpper", "BCancel"], ""),
    ("quit", ["BYes", "BNo"], ""),
    ("interrupted", ["BCont", "BAbort"], ""),
    ("question", ["BOk", "BAbort"], ""),
    ("teach", ["BTeach", "BAbort", "BOk", "BMod"], TEACH_FIXED),
    ("teach_write", ["BYes", "BNo"], ""),
]


def parse_rust(path: Path, table: str) -> dict[str, list[str]]:
    """The rows of a `translations!` table: name -> the eight texts."""
    out: dict[str, list[str]] = {}
    if not path.exists():
        return out
    entry = re.compile(r"^\s+(\w+) => \[(.*)\],?\s*$")
    string = re.compile(r'"((?:[^"\\]|\\.)*)"')
    for line in path.read_text(encoding="utf-8").splitlines():
        m = entry.match(line)
        if m:
            texts = [s.replace('\\"', '"').replace("\\\\", "\\") for s in string.findall(m.group(2))]
            if len(texts) == 8:
                out[f"{table}.{m.group(1)}"] = texts
    return out


def fold(ch: str) -> str:
    ch = ch.lower()
    return GREEK_ACCENT.get(ch, ch)


def canon(lang: str, ch: str) -> str:
    """The letter a key stands for in a language: the layout letter of a Latin key in Russian, Ukrainian or Greek."""
    f = fold(ch)
    lay = LAYOUTS.get(lang, "")
    if "a" <= f <= "z":
        for k, v in zip(lay[0::2], lay[1::2]):
            if k == f:
                return v
    return f


LOOKALIKES = dict(zip("авекмнорстху" "αβεζικμνορτυχ", "abekmhopctxy" "abezikmnoptyx"))


def idents(lang: str, ch: str) -> set[str]:
    """What a letter is taken for: the key it needs and the Latin letter it looks like (a Cyrillic O and a Latin O must not share a screen)."""
    f = fold(ch)
    return {canon(lang, ch), LOOKALIKES.get(f, f)}


def hot_of(text: str) -> str | None:
    i = text.find("~")
    return text[i + 1] if i >= 0 and i + 1 < len(text) else None


def script_of(ch: str) -> str:
    if not ch.isalpha():
        return ""
    name = unicodedata.name(ch, "")
    for s in ("CYRILLIC", "GREEK", "LATIN"):
        if name.startswith(s):
            return s
    return "OTHER"


def candidates(text: str) -> list[int]:
    """The positions of the letters a hot key may use, the best first: the alphabet of the text, not inside {} or after a backslash."""
    scripts = {script_of(c) for c in text} - {""}
    script = "CYRILLIC" if "CYRILLIC" in scripts else "GREEK" if "GREEK" in scripts else "LATIN"
    pos, depth, prev = [], 0, ""
    for i, c in enumerate(text):
        if c == "{":
            depth += 1
        elif c == "}":
            depth = max(0, depth - 1)
        elif depth == 0 and prev != "\\" and script_of(c) == script and c not in "ъь":
            pos.append(i)
        prev = c
    if script == "LATIN":
        # a letter every keyboard has first
        pos.sort(key=lambda i: (not text[i].isascii(), i))
    tail = [i for i, c in enumerate(text) if c in "ъь" and script == "CYRILLIC"]
    return pos + tail


def assign(lang: str, own: dict[str, str], fixed: dict[str, set[str]], groups: list[tuple[str, list[str], str]],
           symbols: dict[str, str]) -> tuple[dict[str, int], list[str]]:
    """The position of the hot letter of every own label so that no screen repeats a letter."""
    cands = {k: candidates(t) for k, t in own.items() if k not in symbols}
    banned = {k: {canon(lang, c) for g, ks, f in groups if k in ks for c in f} for k in own}
    neighbours: dict[str, set[str]] = {k: set() for k in own}
    for _, ks, _ in groups:
        for a in ks:
            if a in own:
                neighbours[a].update(b for b in ks if b != a)
    order = sorted(cands, key=lambda k: (-len(neighbours[k]), len(cands[k]), k))
    chosen: dict[str, int] = {}
    letters: dict[str, set[str]] = dict(fixed)  # label -> what its letter is taken for

    def free(k: str, mine: set[str]) -> bool:
        if mine & banned[k]:
            return False
        return all(not (letters.get(n, set()) & mine) for n in neighbours[k])

    def solve(i: int) -> bool:
        if i == len(order):
            return True
        k = order[i]
        for pos in cands[k]:
            mine = idents(lang, own[k][pos])
            if free(k, mine):
                chosen[k] = pos
                letters[k] = mine
                if solve(i + 1):
                    return True
                del letters[k]
                del chosen[k]
        return False

    for k, sym in symbols.items():
        letters[k] = {sym}
    ok = solve(0)
    return chosen, ([] if ok else ["no free letter for " + ", ".join(order)])


def process(lang: str, path: Path, core: dict[str, list[str]], english_hot: dict[str, str], groups, tsvs: dict[str, dict[str, str]],
            lines_out: dict[str, list[str]]) -> list[str]:
    problems: list[str] = []
    raw = path.read_text(encoding="utf-8").split("\n")
    values: dict[str, str] = {}
    for line in raw:
        if line.startswith("#") or "\t" not in line:
            continue
        k, v = line.split("\t", 1)
        values[k.strip()] = v
    labels = [k for k in english_hot if k in values]
    own = {k: values[k].replace("~", "") for k in labels}
    symbols = {k: canon(lang, english_hot[k]) for k in own if not english_hot[k].isalnum()}
    # the texts the file does not have are taken from the main language of the variant (they carry its marks)
    fixed: dict[str, set[str]] = {}
    base = VARIANT_OF.get(lang)
    if base:
        for k in english_hot:
            if k in own:
                continue
            text = None
            if base in tsvs and k in tsvs[base]:
                text = tsvs[base][k]
            elif k in core:
                text = core[k][CORE.index(base)]
            h = hot_of(text) if text else None
            if h:
                fixed[k] = idents(lang, h)
    chosen, bad = assign(lang, own, fixed, groups, symbols)
    problems += [f"{lang}: {b}" for b in bad]
    result = dict(values)
    for k, t in own.items():
        if k in symbols:
            i = t.find(english_hot[k])
            if i < 0:
                problems.append(f"{lang}: {k} has no {english_hot[k]!r}")
                continue
        elif k in chosen:
            i = chosen[k]
        else:
            continue
        result[k] = t[:i] + "~" + t[i] + "~" + t[i + 1:]
    out = []
    for line in raw:
        if line.startswith("#"):
            line = line.replace("The hot-key markers ~X~ are not used in these languages.",
                                "A ~X~ marks the hot key of a text (written by tools/make_hotkeys.py).")
            out.append(line)
        elif "\t" in line:
            k, v = line.split("\t", 1)
            out.append(f"{k}\t{result.get(k.strip(), v)}")
        else:
            out.append(line)
    lines_out[lang] = out
    tsvs[lang] = result
    return problems


def main() -> int:
    check = "--check" in sys.argv
    classic = parse_rust(ROOT / "src" / "classic" / "texts.rs", "CText")
    plain = parse_rust(ROOT / "src" / "texts.rs", "Text")
    core = {**plain, **classic}
    english_hot = {k: hot_of(v[0]) for k, v in core.items() if hot_of(v[0])}
    groups: list[tuple[str, list[str], str]] = []
    if classic:
        groups += [(n, ["CText." + k for k in ks], f) for n, ks, f in CLASSIC_SCREENS]
    groups.append(("menu", [k for k in english_hot if k.startswith("Text.Menu")], ""))
    files = sorted((ROOT / "locales").glob("*.tsv"), key=lambda p: (p.stem in VARIANT_OF, p.stem))
    tsvs: dict[str, dict[str, str]] = {}
    lines_out: dict[str, list[str]] = {}
    problems: list[str] = []
    for p in files:
        problems += process(p.stem, p, core, english_hot, groups, tsvs, lines_out)
    changed = 0
    for p in files:
        new = "\n".join(lines_out[p.stem])
        if new != p.read_bytes().decode("utf-8"):
            changed += 1
            if not check:
                p.write_bytes(new.encode("utf-8"))
    for line in problems:
        print("PROBLEM", line)
    print(f"files changed: {changed}; problems: {len(problems)}")
    return 1 if problems or (check and changed) else 0


if __name__ == "__main__":
    sys.exit(main())
