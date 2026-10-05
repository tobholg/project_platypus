# Generates assets/art/tyrant_body.ron: a T-rex seen from the side, pointing
# right, built from shapes and shaded, written as text art.
import math
W, H = 156, 72          # sprite cells (x right, y up from the bottom)
def poly(x, y, pts):
    c = False
    n = len(pts)
    for i in range(n):
        (x1, y1), (x2, y2) = pts[i], pts[(i + 1) % n]
        if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
            c = not c
    return c
SKULL = [(106, 28), (106, 44), (110, 50), (118, 53), (128, 52), (140, 47), (149, 43), (152, 39), (152, 34), (116, 33), (110, 30)]
JAW = [(108, 30), (116, 33), (150, 33), (149, 30), (140, 27), (126, 25), (114, 23), (108, 25)]
BROW = [(119, 47), (124, 49), (130, 48), (129, 47), (122, 46)]
def inside(x, y):
    def ell(cx, cy, rx, ry): return ((x-cx)/rx)**2 + ((y-cy)/ry)**2 <= 1
    if poly(x, y, BROW): return 'brow'
    if poly(x, y, SKULL): return 'skull'
    if poly(x, y, JAW): return 'jaw'
    # deep chest and belly, hips higher behind
    if ell(80, 29, 30, 17) or ell(54, 35, 19, 14) or ell(66, 33, 22, 15): return 'body'
    # (its tail is its creature file's chain: only its root here)
    if 36 <= x <= 52:
        t = (52 - x) / 50
        cy = 37 + 5 * t
        half = 12.5 * (1 - t) ** 1.15 + 0.8
        if abs(y - cy) <= half: return 'body'
    # neck: an S from the chest up to the back of the skull
    if 94 <= x <= 112:
        t = (x - 94) / 18
        cy = 33 + 4 * (3 * t * t - 2 * t ** 3)
        half = 11 - 1 * t
        if abs(y - cy) <= half: return 'body'
    return None
grid = [[None] * W for _ in range(H)]
for y in range(H):
    for x in range(W):
        grid[y][x] = inside(x + 0.5, y + 0.5)
def col_span(x):
    ys = [y for y in range(H) if grid[y][x]]
    return (min(ys), max(ys)) if ys else None
rows = []
for y in range(H - 1, -1, -1):
    row = ''
    for x in range(W):
        part = grid[y][x]
        if not part:
            row += '.'; continue
        lo, hi = col_span(x)
        f = (y - lo) / max(1, hi - lo)     # 0 bottom of the column, 1 top
        ch = 'B'
        if f > 0.84: ch = 'L'
        elif f < 0.18: ch = 'b'
        if part == 'brow': ch = 'k'
        # a pale belly and throat
        if part in ('body', 'jaw') and f < 0.3 and 56 <= x <= 120: ch = 'p'
        # dark stripes down its back and tail, slanting
        if part == 'body' and f > 0.66 and ((x + y * 0.7) % 13) < 2.0 and 8 < x < 100: ch = 'k'
        # the head: an eye, a nostril, the mouth's line, teeth
        if part == 'skull':
            if (x - 124.5) ** 2 + (y - 44.5) ** 2 <= 2.3: ch = 'e'
            elif (x - 149) ** 2 + (y - 40) ** 2 <= 1: ch = 'n'
            elif y == 33 and x >= 114: ch = 't' if x % 3 == 1 else 'm'
        if part == 'jaw':
            if y == 32 and x >= 116: ch = 't' if x % 3 == 0 else 'm'
            elif y < 28 and x < 132: ch = 'p'
        row += ch
    rows.append(row)
# trim empty rows/cols, pad one round
while rows and set(rows[0]) == {'.'}: rows.pop(0)
while rows and set(rows[-1]) == {'.'}: rows.pop()
left = min(len(r) - len(r.lstrip('.')) for r in rows)
right = max(len(r.rstrip('.')) for r in rows)
rows = ['.' + r[left:right] + '.' for r in rows]
rows = ['.' * len(rows[0])] + rows + ['.' * len(rows[0])]
w, h = len(rows[0]), len(rows)
# its hip (grip): x 54 in shape space, y 25 up
top_y = max(y for y in range(H) if any(grid[y]))
gx = 54 - left + 1
gy = (top_y - 31) + 1
pal = {
 'B': ((96, 86, 64), 'body'), 'b': ((70, 62, 46), 'body, shade'), 'L': ((128, 114, 84), 'its back, lit'),
 'k': ((50, 44, 34), 'stripes'), 'p': ((168, 150, 112), 'belly, throat'), 'e': ((246, 196, 60), 'eye (it glows)'),
 'm': ((44, 26, 22), 'mouth'), 't': ((236, 228, 206), 'teeth'), 'n': ((30, 24, 20), 'nostril'),
}
used = set(''.join(rows)) - {'.'}
pal = {k: v for k, v in pal.items() if k in used}
out = '''// A ridge tyrant seen from the side, pointing right: a T-rex as big as one
// (about 150 cells nose to tail, its hips twice a person's height), a deep
// body held level (its heavy tail is its creature file's chain), a thick neck up to a great boxy
// skull with a yellow eye and a mouthful of teeth, dark stripes over its
// back, a pale belly. Made by a script (shapes, shaded from above), kept as
// text. Its legs and tiny arms are its creature file's `legs`; `grip` is
// its hip.
(
    size: (%d, %d),
    feet: (%d, %d),
    outline: (24, 20, 16),
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
''' % (w, h, gx, h - 1, "\n".join("        '%s': %s,   // %s" % (k, v[0], v[1]) for k, v in pal.items()), "\n".join('            "%s",' % r for r in rows), gx, gy)
open('assets/art/tyrant_body.ron', 'w').write(out)
print(w, h, 'grip', gx, gy)
