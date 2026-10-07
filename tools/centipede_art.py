# Generates assets/art/centipede_head.ron: a cave centipede's head seen from
# the side, pointing right: a flat armoured head, lit along its top, two
# long antennae swept forward and up, the venom claws (forcipules) curled
# under its face, a small eye. Its body is its creature file's `segments`
# (drawn by legs.rs, a plate a segment); this is only the head.
import math

W, H = 30, 18


def ellipse(x, y, cx, cy, rx, ry):
    return ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1.0


def near_line(x, y, a, b, r):
    (ax, ay), (bx, by) = a, b
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
    px, py = ax + t * dx, ay + t * dy
    return math.hypot(x - px, y - py) <= r


# (y up.) The head a flat dome, x 4..18; the antennae from its front top.
ANTENNAE = [((16, 9), (22, 13)), ((22, 13), (28, 14)), ((15, 9), (20, 15)), ((20, 15), (24, 17))]
# The venom claws: down and forward from under its face, then hooked back.
CLAW = [((15, 4), (19, 2)), ((19, 2), (20, 0.5)), ((20, 0.5), (18, 0.2))]


def part(x, y):
    if ellipse(x, y, 11, 6, 7.5, 4.2) and y >= 3:
        return 'head'
    if any(near_line(x, y, a, b, 0.75) for a, b in CLAW):
        return 'claw'
    if any(near_line(x, y, a, b, 0.55) for a, b in ANTENNAE):
        return 'antenna'
    return None


grid = [[part(x + 0.5, y + 0.5) for x in range(W)] for y in range(H)]


def span(x, kind):
    ys = [y for y in range(H) if grid[y][x] == kind]
    return (min(ys), max(ys)) if ys else (0, 1)


rows = []
for y in range(H - 1, -1, -1):
    row = ''
    for x in range(W):
        p = grid[y][x]
        if not p:
            row += '.'
            continue
        if p == 'head':
            lo, hi = span(x, 'head')
            f = (y - lo) / max(1, hi - lo)
            ch = 'H' if f > 0.7 else ('h' if f > 0.3 else 'd')
            # the plate's seam across its back, its eye at the front
            if x == 9 and y > lo + 1:
                ch = 'd'
            if (x, y) == (15, 7):
                ch = 'e'
        elif p == 'claw':
            ch = 'k' if x >= 19 else 'c'
        else:
            ch = 'a'
        row += ch
    rows.append(row)
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
# (Its grip: the middle of its head, its underside.)
gx, gy = 11 - left + 1, (top - 4) + 1
pal = {
    'H': ((196, 104, 52), 'head, lit'), 'h': ((150, 70, 36), 'head'), 'd': ((92, 40, 22), 'head, shade'),
    'a': ((132, 76, 46), 'antennae'), 'c': ((110, 48, 26), 'venom claws'), 'k': ((34, 18, 12), 'their black tips'),
    'e': ((244, 208, 110), 'its eye (it glows a little)'),
}
used = set(''.join(rows)) - {'.'}
out = '''// A cave centipede's head seen from the side, pointing right: a flat
// armoured head, lit along its top, two long antennae swept forward, the
// venom claws curled under its face, a small eye. Made by
// tools/centipede_art.py, kept as text. Its body is its creature file's
// `segments` (legs.rs draws them, a plate a segment); `grip` is the middle
// of its head.
(
    size: (%d, %d),
    feet: (%d, %d),
    outline: (30, 14, 10),
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
open('assets/art/centipede_head.ron', 'w').write(out)
print(w, h, 'grip', gx, gy)
print('\n'.join(rows))
