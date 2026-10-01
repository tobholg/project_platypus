"""A text sprite at 1.5x: coordinates scaled, every grid upscaled (old even
pixels become two, odd ones one). A starting point to redraw by hand."""
import math, re, sys
S = 1.5
def c(v): return math.ceil(S * v)
def cx(v):
    f = math.floor(v); return c(f) + (v - f)
def pair(m, fn=c):
    a, b = float(m.group(2)), float(m.group(3))
    fa, fb = fn(a), fn(b)
    fmt = lambda x: str(int(x)) if float(x).is_integer() else str(x)
    return f"{m.group(1)}({fmt(fa)}, {fmt(fb)})"
ROW = re.compile(r'^(\s*)"([^"]*)"(,?)(\s*//.*)?$')
def up(rows):
    w = max(len(r) for r in rows); rows = [r.ljust(w, '.') for r in rows]
    nw, nh = c(w), c(len(rows))
    return ["".join(rows[int(y // S)][int(x // S)] for x in range(nw)) for y in range(nh)]
src = open(sys.argv[1]).read().split("\n")
out, block = [], []
def flush():
    global block
    if block:
        ind, comma = block[0][0], block[0][2]
        for r in up([b[1] for b in block]): out.append(f'{ind}"{r}"{comma}')
        block = []
in_anchors = False
for line in src:
    m = ROW.match(line)
    if m and not in_anchors:
        block.append((m.group(1), m.group(2), m.group(3))); continue
    flush()
    if 'anchors' in line: in_anchors = True
    line = re.sub(r'(size: )\(([\d.]+), ([\d.]+)\)', lambda m: pair(m), line)
    line = re.sub(r'(feet: )\(([\d.]+), ([\d.]+)\)', lambda m: pair(m, lambda v: cx(v) if v != int(v) else c(v)), line)
    line = re.sub(r'((?:pivot|at|shift): )\((-?[\d.]+), (-?[\d.]+)\)', lambda m: pair(m), line)
    line = re.sub(r'("[\w]+": )\((-?\d+), (-?\d+)\)', lambda m: pair(m), line)
    out.append(line)
flush()
open(sys.argv[2], "w").write("\n".join(out))
