# Generates assets/art/zombie_warlock.ron: a robed zombie caster, its robe
# drawn in place (its hem swaying by the frame), its hood (its head), and
# sleeved arms laid along a two-bone reach for each pose.
import math, sys

W, H = 44, 46
FEET = (19.5, 46)
OUTLINE = (14, 10, 18)

PAL = [
    ('L', (104, 88, 128), 'the robe, lit'),
    ('l', (80, 66, 100), 'the robe: faded purple'),
    ('j', (62, 52, 80), 'the robe'),
    ('h', (46, 38, 60), 'the robe, shade'),
    ('b', (30, 24, 40), 'the robe, deep; the hood\'s edge'),
    ('a', (14, 10, 18), 'the dark in its hood'),
    ('s', (132, 136, 112), 'bone-grey skin, lit'),
    ('S', (98, 102, 84), 'skin'),
    ('D', (70, 72, 60), 'skin, shade'),
    ('k', (34, 30, 28), 'sockets, its mouth'),
    ('w', (196, 190, 160), 'teeth'),
    ('e', (150, 255, 100), 'its eyes, and the grave-light it gathers: green, burning'),
    ('r', (112, 76, 48), 'a rope belt'),
    ('R', (76, 50, 34), 'the rope, shade'),
]
COL = {k: c for k, c, _ in PAL}


def blank():
    return [['.'] * W for _ in range(H)]


def put(g, x, y, c):
    x, y = int(round(x)), int(round(y))
    if 0 <= x < W and 0 <= y < H:
        g[y][x] = c


def crop(g):
    pts = [(x, y) for y in range(H) for x in range(W) if g[y][x] != '.']
    x0, x1 = min(p[0] for p in pts), max(p[0] for p in pts)
    y0, y1 = min(p[1] for p in pts), max(p[1] for p in pts)
    return [''.join(g[y][x0:x1 + 1]) for y in range(y0, y1 + 1)], (x0, y0)


# ---- the robe: from its shoulders to its hem (canvas pixels) ----
# each row: the robe's back and front edge
ROBE = {19: (17, 23), 20: (16, 23), 21: (15, 23), 22: (15, 23), 23: (15, 23), 24: (14, 23), 25: (14, 24), 26: (14, 24),
        27: (14, 23), 28: (14, 23), 29: (13, 23), 30: (13, 23), 31: (13, 24), 32: (13, 24), 33: (13, 24), 34: (13, 25), 35: (13, 25)}
# the hem's ragged strands: how far below the last row each column hangs
STRANDS = {13: 2, 14: 2, 15: 0, 16: 3, 17: 1, 18: 2, 19: 0, 20: 2, 21: 1, 22: 3, 23: 1, 24: 2, 25: 0}
BELT = 26
FOLDS = [15, 18, 21]


def robe(sway=0, lift=0):
    """The robe, its lower half swung `sway` cells at the hem (the way it
    swings as it walks), its strands `lift` shorter (the hem kicked up)."""
    g = blank()
    last = max(ROBE)
    for y, (x0, x1) in ROBE.items():
        k = max(0, y - 28) / (last - 28)
        dx = int(round(sway * k))
        for x in range(x0, x1 + 1):
            if x == x1:
                c = 'L' if y < 31 else 'l'
            elif x == x0:
                c = 'b'
            elif x == x1 - 1:
                c = 'l'
            elif x - (y - 19) // 6 in FOLDS:
                c = 'h'
            elif x <= x0 + 2:
                c = 'h'
            else:
                c = 'j'
            if y in (BELT, BELT + 1) and x > x0:
                c = 'r' if (x + y) % 3 else 'R'
            put(g, x + dx, y, c)
    # The rope's end hanging off it in front.
    for y in range(BELT + 1, BELT + 5):
        put(g, ROBE[BELT][1] - 1 + (y > BELT + 2), y, 'r' if y % 2 else 'R')
    # The hem: rags hanging in strands.
    for x, n in STRANDS.items():
        n = max(0, n - lift)
        for j in range(1, n + 1):
            put(g, x + sway, last + j, 'h' if j < n else 'b')
    # Holes torn in it.
    for (x, y) in [(15, 31), (20, 33), (17, 28)]:
        put(g, x + int(round(sway * max(0, y - 28) / 7)), y, '.')
    return g


HOOD = [
    "....bbhhb...",
    "...bhjjjhb..",
    "..bhjlLljhb.",
    ".bhjlljjjjhb",
    ".hjljjhbaaab",
    "bhjjjhbaSssb",
    "bhjjjhbaeSeb",
    "bhjjjhbaSssa",
    "bhjjjjhaSkkb",
    "bhjjjjhaswsb",
    ".bhjjjjhaSb.",
    ".bhjjjjjhhb.",
    "..bhhjjjhb..",
]
# its eyes flaring (the tell before it casts or calls)
FLARE = list(HOOD)
FLARE[5] = "bhjjjhbaeSeb"
FLARE[6] = "bhjjjhbeeSee"
FLARE[7] = "bhjjjhbaeSea"
BLINK = list(HOOD)
BLINK[6] = "bhjjjhbakSkb"
HOOD_AT = (15, 9)

UPPER, FORE = 7.0, 7.0


def elbow(s, w, bend=1):
    dx, dy = w[0] - s[0], w[1] - s[1]
    d = min(math.hypot(dx, dy), UPPER + FORE - 0.01)
    a = math.acos(max(-1, min(1, (UPPER ** 2 + d ** 2 - FORE ** 2) / (2 * UPPER * d))))
    ang = math.atan2(dy, dx) + a * bend
    return (s[0] + UPPER * math.cos(ang), s[1] + UPPER * math.sin(ang))


HANDS = {
    # Reaching ahead, clawed: long fingers, nails.
    'reach': [".SSs.", "DSSSsk", ".DDSk.", "..D.."],
    # Up, fingers spread.
    'up': ["D.S.k", "D.S.D", "DSS.D", ".DSS.", "..Ss."],
    # Palm down, fingers spread at the ground.
    'down': [".SS..", "DSSSs", "D.S.S", "k.k.k"],
}
HAND_WRIST = {'reach': (0, 1), 'up': (2, 4), 'down': (1, 0)}


def stroke(g, a, b, wa, wb, cols):
    """A bone a→b, wa..wb across, `cols` (lit, mid, shade) across it, lit
    on its upper side."""
    n = int(max(abs(b[0] - a[0]), abs(b[1] - a[1])) * 3) + 1
    dx, dy = b[0] - a[0], b[1] - a[1]
    L = math.hypot(dx, dy) or 1
    nx, ny = -dy / L, dx / L
    if ny > 0 or (ny == 0 and nx > 0):
        nx, ny = -nx, -ny
    cells = {}
    for i in range(n + 1):
        t = i / n
        cx, cy = a[0] + dx * t, a[1] + dy * t
        r = (wa + (wb - wa) * t) / 2
        for oy in range(-3, 4):
            for ox in range(-3, 4):
                px, py = math.floor(cx) + ox, math.floor(cy) + oy
                qx, qy = px + 0.5 - cx, py + 0.5 - cy
                along = qx * dx / L + qy * dy / L
                across = qx * nx + qy * ny
                if abs(across) <= r and abs(along) < 0.7:
                    old = cells.get((px, py))
                    if old is None or abs(across) < abs(old):
                        cells[(px, py)] = across
    for (px, py), s in cells.items():
        put(g, px, py, cols[0] if s > 0.3 else (cols[1] if s > -0.5 else cols[2]))
    return cells


def arm(s, w, hand, bend=1, rags=True):
    """A sleeved arm, its rags hanging under the forearm, a bony hand."""
    g = blank()
    e = elbow(s, w, bend)
    stroke(g, s, e, 3.2, 3.0, ('l', 'j', 'h'))
    fore = stroke(g, e, w, 3.0, 2.6, ('l', 'j', 'h'))
    # The sleeve's rags, hanging off the underside of the forearm.
    if rags:
        n = 0
        for t in (0.25, 0.55, 0.85):
            x, y = e[0] + (w[0] - e[0]) * t, e[1] + (w[1] - e[1]) * t
            low = max((py for (px, py) in fore if px == int(math.floor(x))), default=int(y))
            for j in range(1, 2 + (n % 2) + 1):
                put(g, math.floor(x), low + j, 'h' if j == 1 else 'b')
            n += 1
    hx, hy = HAND_WRIST[hand]
    for j, row in enumerate(HANDS[hand]):
        for i, c in enumerate(row):
            if c != '.':
                put(g, w[0] + i - hx, w[1] + j - hy, c)
    return g


parts = {}
poses = {}


def add_part(name, rows, pivot=(0, 0), points=None):
    w = max(len(r) for r in rows)
    parts[name] = (pivot, points or {}, [r.ljust(w, '.') for r in rows])


add_part('head', HOOD)
add_part('head_flare', FLARE)
add_part('blink', BLINK)
robes = {}


def robe_part(sway, lift):
    key = (sway, lift)
    if key not in robes:
        rows, at = crop(robe(sway, lift))
        name = 'robe' if key == (0, 0) else f"robe_{'l' if sway < 0 else 'r'}{abs(sway)}" + (f"_up{lift}" if lift else '')
        add_part(name, rows)
        robes[key] = (name, at)
    return robes[key]


SH_FRONT = (20.5, 21.5)
SH_BACK = (17.5, 21.5)
ORB = []


def pose(name, body=(0, 0), head=(0, 0), front=None, back=None, head_part='head', sway=0, lift=0, orb=None):
    bx, by = body
    rname, rat = robe_part(sway, lift)
    layers = []
    for tag, sh, spec, shade in (('back_arm', SH_BACK, back, 0.7), ('front_arm', SH_FRONT, front, 1.0)):
        wrist, hand = spec[0], spec[1]
        bend = spec[2] if len(spec) > 2 else 1
        s = (sh[0] + bx, sh[1] + by)
        g = arm(s, wrist, hand, bend)
        rows, (x0, y0) = crop(g)
        pivot = (int(round(s[0])) - x0, int(round(s[1])) - y0)
        hp = (int(round(wrist[0])) - x0 + 1, int(round(wrist[1])) - y0)
        pname = f"{tag.split('_')[0]}_{name}"
        add_part(pname, rows, pivot, {'hand': hp})
        layers.append((pname, (int(round(s[0])), int(round(s[1])))))
    out = [
        dict(part=layers[0][0], at=layers[0][1], shade=0.7, tag='back_arm'),
        dict(part=rname, at=(rat[0] + bx, rat[1] + by)),
        dict(part=head_part, at=(HOOD_AT[0] + bx + head[0], HOOD_AT[1] + by + head[1])),
        dict(part=layers[1][0], at=layers[1][1], outline=True, tag='front_arm'),
    ]
    if orb:
        out.append(dict(part='orb', at=orb))
    poses[name] = out


# A ball of grave-light, held between its hands.
add_part('orb', [".eee.", "eeeee", "eeeee", "eeeee", ".eee."], (2, 2))

# Standing: upright, both clawed hands held out ahead of it, reaching.
pose('stand', front=((29, 25), 'reach'), back=((27, 24), 'reach'))
pose('breathe', body=(0, -1), front=((29, 24), 'reach'), back=((27, 23), 'reach'))
pose('blink', front=((29, 25), 'reach'), back=((27, 24), 'reach'), head_part='blink')
# Its shamble, in step with its legs (frame k: k/8 through its gait, the
# near foot down at 0): it bobs as each foot passes under it, its hands
# rising and dipping out ahead of it, each against its own side's foot;
# its robe swinging after its legs, the hem kicked up as a foot goes by.
BOB = [0, -1, -1, 0, 0, -1, -1, 0]
SWAY = [1, 1, 0, -1, -1, -1, 0, 1]
LIFT = [0, 1, 1, 0, 0, 1, 1, 0]
FR = [(29, 26), (29.5, 25), (30, 24), (30, 24), (29.5, 25), (29, 26), (28.5, 26), (28.5, 26)]
BK = [(27, 23), (26.5, 24), (26, 25), (26, 25), (26.5, 24), (27, 23), (27.5, 23), (27.5, 23)]
for k in range(8):
    pose(f'run_{k}', body=(0, BOB[k]), head=((1 if k in (2, 6) else 0), 0), front=(FR[k], 'reach'), back=(BK[k], 'reach'), sway=SWAY[k], lift=LIFT[k])
# Casting (the tell): both hands up ahead of its hood, the grave-light
# gathering between them, its eyes flaring; then thrust out at you.
pose('gather', body=(0, -1), head=(-1, 0), front=((31, 8), 'up', 1), back=((28, 7), 'up', 1), head_part='head_flare', orb=(30, 5))
pose('gather_b', body=(0, -1), head=(-1, 0), front=((31, 7), 'up', 1), back=((28, 8), 'up', 1), head_part='head_flare', orb=(30, 4))
pose('throw', body=(1, 0), head=(1, 0), front=((33, 21), 'reach'), back=((31, 20), 'reach'), head_part='head_flare')
# Calling up the dead: arms flung up and wide, eyes flaring; then swept
# down, palms at the ground either side of it.
pose('raise', body=(0, -1), head=(0, -1), front=((32, 9), 'up', 1), back=((10, 9), 'up', 1), head_part='head_flare', sway=-1)
pose('beckon', body=(0, 1), head=(1, 1), front=((31, 32), 'down'), back=((9, 31), 'down', -1), head_part='head_flare', sway=1)
# Drawn in (a shove's tell): hands to its chest.
pose('draw', body=(-1, 0), front=((23, 25), 'reach', 1), back=((21, 24), 'reach', 1))
# In the air: arms up; falling: arms out.
pose('rise', front=((28, 12), 'up', -1), back=((24, 12), 'up', -1))
pose('drop', front=((30, 30), 'reach'), back=((10, 29), 'reach', -1), sway=-1)

# The front arm at angles (raking): the arm straight out, turned.
g = arm((10, 10), (10 + UPPER + FORE - 0.3, 10.5), 'reach', 1, rags=False)
rows, (x0, y0) = crop(g)
add_part('aim', rows, (10 - x0, 10 - y0), {'hand': (int(round(10 + UPPER + FORE - 0.3)) - x0 + 1, 11 - y0)})


def ron():
    out = []
    w = out.append
    w("""// A zombie warlock: one of the dead that kept its craft. Upright, in a
// tattered hooded robe of faded purple with a rope belt; a grey skull of a
// face in the dark of its hood, its eyes burning green; both clawed grey
// hands held out ahead of it as it shambles. Its legs aren't drawn here:
// they're its creature file's `legs` (legs.rs, `rig`: bandaged shins and
// wrapped boots from under its hem, stepping), and its walk keeps in step
// with them (`in_step`: frame k is k/8 through its gait), the robe
// swinging after its legs. Casting, its hands go up over its hood and the
// grave-light gathers between them, its eyes flaring (`gather`: the tell),
// then it thrusts them at you (`throw`); calling up the dead, its arms go
// up wide (`raise`), then sweep down at the ground (`beckon`). Its arms
// are laid along a reach (shoulder, elbow, wrist) for each pose. Facing
// right; the game mirrors it.
(""")
    w(f"    size: ({W}, {H}),")
    w(f"    feet: ({FEET[0]}, {FEET[1]}),")
    w(f"    outline: {OUTLINE},")
    w("    palette: {")
    for k, c, note in PAL:
        w(f"        '{k}': ({c[0]}, {c[1]}, {c[2]}),".ljust(36) + f"// {note}")
    w("    },")
    w("    parts: {")
    for name, (pivot, points, rows) in parts.items():
        pts = ', '.join(f'"{n}": ({x}, {y})' for n, (x, y) in points.items())
        if points:
            w(f'        "{name}": (pivot: ({pivot[0]}, {pivot[1]}), points: {{ {pts} }}, rows: [')
        else:
            w(f'        "{name}": (pivot: ({pivot[0]}, {pivot[1]}), rows: [')
        for r in rows:
            w(f'            "{r}",')
        w("        ]),")
    w("    },")
    w("    poses: {")
    for name, layers in poses.items():
        w(f'        "{name}": [')
        for l in layers:
            bits = [f'part: "{l["part"]}"', f'at: ({l["at"][0]}, {l["at"][1]})']
            if l.get('shade', 1.0) != 1.0:
                bits.append(f'shade: {l["shade"]}')
            if l.get('outline'):
                bits.append('outline: true')
            if l.get('tag'):
                bits.append(f'tag: "{l["tag"]}"')
            w(f'            ({", ".join(bits)}),')
        w("        ],")
    w("    },")
    w("""    // The front arm at angles (raking): the straight arm, turned; shown
    // instead of the pose's own front arm (`front_arm`) while it aims.
    fans: {
        "front_arm": [""")
    for a in (-90, -60, -30, 0, 30, 60, 90):
        w(f'            (angle: {a}, part: "aim", from: 0),')
    w("""        ],
    },
    derived: {
        // Hit: knocked back a pixel.
        "hurt": (from: "drop", shift: (-1, 0)),
    },
    clips: {
        "idle": (frames: ["stand", "stand", "breathe", "breathe", "stand", "stand", "breathe", "breathe", "stand", "blink", "stand", "breathe", "breathe"], fps: 4),
        "run": (frames: ["run_0", "run_1", "run_2", "run_3", "run_4", "run_5", "run_6", "run_7"], fps: 8),
        "jump": (frames: ["rise"], fps: 5),
        "fall": (frames: ["drop"], fps: 5),
        "hurt": (frames: ["hurt"], fps: 5),
        "gather": (frames: ["gather", "gather_b"], fps: 8),
        "throw": (frames: ["throw"], fps: 5),
        "raise": (frames: ["raise"], fps: 5),
        "beckon": (frames: ["beckon"], fps: 5),
        "draw": (frames: ["draw"], fps: 5),
    },
)""")
    return '\n'.join(out) + '\n'


open(sys.argv[1], 'w').write(ron())
