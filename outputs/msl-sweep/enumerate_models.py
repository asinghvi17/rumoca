#!/usr/bin/env python3
"""List every simulatable MSL example: a non-partial class under
Modelica.*.Examples whose own annotation has an experiment(...).

    python3 -I enumerate_models.py "<msl>/Modelica 4.1.0" > models.txt

The scan is textual: it tracks class nesting via `<kind> Name` ... `end Name;`
lines, strips comments and string literals first, and attributes each
`experiment(` to the innermost open class. Short class definitions
(`model A = B(...) annotation(experiment(...))`) are counted too.
"""
import os
import re
import sys

KINDS = r"(?:model|block|class|package|record|connector|function|type|operator\s+record|expandable\s+connector)"
OPEN = re.compile(
    r"^\s*(?:(?:redeclare|final|inner|outer|replaceable|encapsulated|partial|impure|pure)\s+)*"
    r"(" + KINDS + r")\s+(?:extends\s+)?(\w+)\b(?!\.|\s*\()(.*)$")
END = re.compile(r"^\s*end\s+(\w+)\s*;")


def strip(text):
    """Remove comments and string contents, keeping line structure."""
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append('""' + "\n" * text.count("\n", i, j))
            i = j + 1
        elif text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("\n" * text.count("\n", i, j))
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


def file_prefix(root, path):
    rel = os.path.relpath(path, root)
    parts = rel[:-3].split(os.sep)
    if parts[-1] == "package":
        parts = parts[:-1]
    else:
        parts = parts[:-1]  # the file's own class name comes from its text
    return ["Modelica"] + parts


def scan(root):
    found = []
    for dirpath, _, files in os.walk(root):
        for f in sorted(files):
            if not f.endswith(".mo"):
                continue
            path = os.path.join(dirpath, f)
            with open(path, encoding="utf-8", errors="replace") as fh:
                text = strip(fh.read())
            prefix = file_prefix(root, path)
            if f == "package.mo":
                prefix = prefix[:-1]  # the package's own `package X` line re-adds it
            stack = []  # (name, partial)
            for line in text.split("\n"):
                m = OPEN.match(line)
                if m:
                    kind, name, rest = m.groups()
                    partial = re.search(r"\bpartial\b", line[: m.start(1)]) is not None
                    short = re.match(r"\s*=", rest) is not None
                    if short:
                        if "experiment(" in rest.replace(" ", ""):
                            found.append((prefix + [s for s, _, _ in stack] + [name], partial, kind))
                        continue
                    stack.append((name, partial, kind))
                    if "experiment(" in rest.replace(" ", "") and not partial:
                        found.append((prefix + [s for s, _, _ in stack], partial, kind))
                    continue
                m = END.match(line)
                if m and stack and stack[-1][0] == m.group(1):
                    stack.pop()
                    continue
                if stack and "experiment(" in line.replace(" ", ""):
                    name, partial, kind = stack[-1]
                    if not partial:
                        found.append((prefix + [s for s, _, _ in stack], partial, kind))
    seen, out = set(), []
    for parts, partial, kind in found:
        if partial:
            continue
        q = ".".join(parts)
        if q not in seen:
            seen.add(q)
            out.append(q)
    return sorted(out)


if __name__ == "__main__":
    for q in scan(sys.argv[1]):
        if ".Examples." in q + ".":
            print(q)
