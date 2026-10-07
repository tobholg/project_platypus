# Generates assets/art/strider_body.ron: an iron strider's hull seen from the
# side, pointing right: an armoured wedge with a glowing viewport slit, plate
# seams and rivets, an antenna, a gun slung under its nose. Shapes, shaded
# from above, written as text art.
W, H = 64, 40
def poly(x, y, pts):
    c = False
    for i in range(len(pts)):
        (x1, y1), (x2, y2) = pts[i], pts[(i + 1) % len(pts)]
        if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
            c = not c
    return c
HULL = [(6, 12), (4, 20), (8, 27), (14, 31), (34, 31), (42, 27), (48, 21), (49, 15), (44, 10), (12, 8)]
SLIT = [(38, 20), (46, 20), (47, 18), (38, 18)]
GUN = [(34, 7), (60, 7), (60, 10), (34, 10)]
MOUNT = [(30, 6), (38, 6), (38, 11), (30, 11)]
HIP = [(16, 4), (28, 4), (30, 9), (14, 9)]
def part(x, y):
    if poly(x, y, SLIT): return 'slit'
    if poly(x, y, HULL): return 'hull'
    if poly(x, y, MOUNT) or poly(x, y, GUN): return 'gun'
    if poly(x, y, HIP): return 'hip'
    if 15 <= x <= 16 and 31 <= y <= 38: return 'aerial'
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
            row += '.'; continue
        lo, hi = span(x, p)
        f = (y - lo) / max(1, hi - lo)
        if p == 'slit': ch = 'e'
        elif p == 'aerial': ch = 'd' if y < 37 else 'r'
        elif p == 'gun': ch = 'd' if f < 0.4 else 'g'
        elif p == 'hip': ch = 'd' if f < 0.5 else 'M'
        else:
            ch = 'M'
            if f > 0.8: ch = 'L'
            elif f < 0.25: ch = 'd'
            # plate seams, rivets along them
            if y in (14, 24) and 6 < x < 46: ch = 's' if x % 6 else 'v'
            if x in (20, 32) and 9 < y < 30: ch = 's'
            # a hazard band and a number
            if 24 < y < 29 and 22 <= x <= 29 and (x + y) % 4 < 2: ch = 'y'
        row += ch
    rows.append(row)
while set(rows[0]) == {'.'}: rows.pop(0)
while set(rows[-1]) == {'.'}: rows.pop()
left = min(len(r) - len(r.lstrip('.')) for r in rows)
right = max(len(r.rstrip('.')) for r in rows)
rows = ['.' + r[left:right] + '.' for r in rows]
rows = ['.' * len(rows[0])] + rows + ['.' * len(rows[0])]
w, h = len(rows[0]), len(rows)
top = max(y for y in range(H) if any(grid[y]))
gx, gy = 22 - left + 1, (top - 5) + 1
pal = {
 'M': ((118, 122, 128), 'hull'), 'L': ((160, 166, 172), 'hull, lit'), 'd': ((72, 74, 80), 'hull, shade'),
 's': ((84, 86, 92), 'plate seams'), 'v': ((196, 200, 206), 'rivets'), 'y': ((214, 170, 52), 'hazard band'),
 'e': ((120, 230, 255), 'viewport (it glows)'), 'g': ((96, 98, 104), 'gun'), 'r': ((230, 70, 60), 'aerial light'),
}
used = set(''.join(rows)) - {'.'}
out = '''// An iron strider's hull seen from the side, pointing right: an armoured
// wedge with a glowing viewport slit, plate seams and rivets, a hazard
// band, an aerial, a gun slung under its nose. Made by tools/strider_art.py
// (shapes, shaded from above), kept as text. Its two plated legs are its
// creature file's `legs`; `grip` is its hip joint.
(
    size: (%d, %d),
    feet: (%d, %d),
    outline: (26, 28, 32),
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
open('assets/art/strider_body.ron', 'w').write(out)
print(w, h, 'grip', gx, gy)

# The strider's laser gun, seen from the side, pointing right: a round
# mount (its pivot, the `grip`), a ribbed barrel, an emitter lens at the
# muzzle. Its heat's glow is drawn over these pixels (legs.rs).
G = [
    "..MMM.............",
    ".MLLLM............",
    "MLLLLLMMMMMMMMMM..",
    "MLLvLLssssssssssre",
    "MdddddddddddddddrE",
    ".MdddM......dddd..",
    "..MMM.............",
]
gpal = {
 'M': ((72, 74, 80), 'mount, shade'), 'L': ((160, 166, 172), 'mount, lit'), 'd': ((96, 98, 104), 'barrel'),
 's': ((132, 136, 142), 'barrel, lit'), 'v': ((196, 200, 206), 'pivot bolt'),
 'r': ((140, 30, 26), 'emitter ring'), 'e': ((255, 90, 70), 'emitter (it glows)'), 'E': ((210, 50, 40), 'emitter, lower'),
}
grows = ['.' * (len(G[0]) + 2)] + ['.' + r + '.' for r in G] + ['.' * (len(G[0]) + 2)]
used = set(''.join(grows)) - {'.'}
gout = '''// The iron strider's laser gun seen from the side, pointing right: a round
// mount (`grip`: its pivot), a ribbed barrel, an emitter lens at the muzzle.
// Made by tools/strider_art.py. A turret of its creature file's `legs`
// (legs.rs turns it to where it aims and glows it with its heat).
(
    size: (%d, %d),
    feet: (4, %d),
    outline: (26, 28, 32),
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
    anchors: { "grip": { "body": (4, 4) } },
)
''' % (len(grows[0]), len(grows), len(grows) - 1, "\n".join("        '%s': %s,   // %s" % (k, v[0], v[1]) for k, v in gpal.items() if k in used), "\n".join('            "%s",' % r for r in grows))
open('assets/art/strider_gun.ron', 'w').write(gout)
print('gun', len(grows[0]), len(grows))
