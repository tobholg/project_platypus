# Generates assets/art/spider_side.ron: the cave spider seen from the side,
# pointing right: a big round abdomen with a red mark and bristles, a narrow
# waist, a smaller head with its eyes glowing at the front of it, fangs
# hanging under its face. Shapes, shaded from above, written as text art
# (the same palette as spider_body.ron, its view from above).
import math

W, H = 52, 30


def ellipse(x, y, cx, cy, rx, ry, turn=0.0):
    c, s = math.cos(turn), math.sin(turn)
    dx, dy = x - cx, y - cy
    u, v = dx * c + dy * s, -dx * s + dy * c
    return (u / rx) ** 2 + (v / ry) ** 2 <= 1.0


def poly(x, y, pts):
    c = False
    for i in range(len(pts)):
        (x1, y1), (x2, y2) = pts[i], pts[(i + 1) % len(pts)]
        if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
            c = not c
    return c


# (y up.) The abdomen tipped up a little at its back, the head low and
# forward, the fangs under the face.
FANG = [(41, 9), (45, 8), (46, 3), (44, 1), (43, 4), (40, 6)]
WAIST = [(25, 9), (29, 10), (29, 14), (25, 15)]


def part(x, y):
    if poly(x, y, FANG):
        return 'fang'
    if ellipse(x, y, 35, 11, 9.5, 6.2, -0.12):
        return 'head'
    if poly(x, y, WAIST):
        return 'waist'
    if ellipse(x, y, 14, 15, 13.5, 10.5, 0.18):
        return 'abdomen'
    return None


grid = [[part(x + 0.5, y + 0.5) for x in range(W)] for y in range(H)]


def span(x, kinds):
    ys = [y for y in range(H) if grid[y][x] in kinds]
    return (min(ys), max(ys)) if ys else (0, 1)


rows = []
for y in range(H - 1, -1, -1):
    row = ''
    for x in range(W):
        p = grid[y][x]
        if not p:
            row += '.'
            continue
        if p == 'eye':
            row += 'e'
            continue
        lo, hi = span(x, {p, 'eye'})
        f = (y - lo) / max(1, hi - lo)
        if p == 'fang':
            ch = 'f' if y > 3 else 'F'
            if x < 42 and y > 5:
                ch = 'c'
        elif p in ('head', 'waist'):
            ch = 'C' if f > 0.62 else 'c'
            # its eyes: a cluster at the front of its head, apart
            if p == 'head' and (x in (38, 41) and y == hi - 1 or x == 42 and y == hi - 3):
                ch = 'e'
        else:
            ch = 'B'
            if f > 0.8:
                ch = 'L'
            elif f < 0.3:
                ch = 'b'
            # its mark: a red hourglass on its back
            if 8 <= x <= 18 and hi - 4 <= y <= hi - 1 and abs(x - 13) <= 1 + abs(y - (hi - 2.5)) * 1.4:
                ch = 'x' if (x + y) % 3 else 'X'
            # bristles along its back
            if y == hi and x % 4 == 1:
                ch = 'h'
        row += ch
    rows.append(row)
# (Bristles stood up off its back.)
for x in range(2, 26, 4):
    lo, hi = span(x, {'abdomen'})
    r = H - 1 - (hi + 1)
    if 0 <= r < len(rows) and rows[r][x] == '.':
        rows[r] = rows[r][:x] + 'h' + rows[r][x + 1:]
while set(rows[0]) == {'.'}:
    rows.pop(0)
while set(rows[-1]) == {'.'}:
    rows.pop()
left = min(len(r) - len(r.lstrip('.')) for r in rows)
right = max(len(r.rstrip('.')) for r in rows)
rows = ['.' + r[left:right] + '.' for r in rows]
rows = ['.' * len(rows[0])] + rows + ['.' * len(rows[0])]
w, h = len(rows[0]), len(rows)
top = max(y for y in range(H) if any(grid[y][x] for x in range(W)))
# (Its grip: at its waist, the middle of it; its legs meet under its head,
# ahead of that.)
gx, gy = 28 - left + 1, (top - 8) + 1
pal = {
    'b': ((36, 26, 32), 'abdomen'), 'B': ((62, 46, 56), 'abdomen, lit'), 'L': ((84, 64, 78), 'abdomen, the light on its back'),
    'h': ((96, 78, 86), 'bristles'), 'x': ((150, 26, 26), 'its mark'), 'X': ((110, 18, 22), 'its mark, deeper'),
    'c': ((44, 32, 40), 'head'), 'C': ((70, 54, 64), 'head, lit'), 'e': ((255, 52, 38), 'eyes (they glow: the creature\'s light)'),
    'f': ((208, 196, 172), 'fangs'), 'F': ((150, 138, 120), 'fangs, shade'),
}
used = set(''.join(rows)) - {'.'}
out = '''// A cave spider seen from the side, pointing right: a big round abdomen
// with a red mark and bristles, a narrow waist, a smaller head with its eyes
// glowing at its front, fangs under its face. Made by tools/spider_side_art.py
// (shapes, shaded from above), kept as text. Its legs are its creature
// file's `legs` (from the side; spider_body.ron is it from above, on the
// wall behind); `grip` is its waist, the middle of it.
(
    size: (%d, %d),
    feet: (%d, %d),
    outline: (14, 8, 12),
    palette: {
%s
    },
    frames: {
        "body": [
%s
        ],
    },
    clips: {
        "idle": (frames: ["body"], fps: 1),
    },
    anchors: { "grip": { "body": (%d, %d) } },
)
''' % (w, h, gx, h - 1, "\n".join("        '%s': %s,   // %s" % (k, v[0], v[1]) for k, v in pal.items() if k in used), "\n".join('            "%s",' % r for r in rows), gx, gy)
open('assets/art/spider_side.ron', 'w').write(out)
print(w, h, 'grip', gx, gy)
print('\n'.join(rows))
