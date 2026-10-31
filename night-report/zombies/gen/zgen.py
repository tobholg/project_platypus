# Generates assets/art/zombie.ron (the hunched zombie) from a few drawn
# parts and arms laid along a two-bone reach for each pose.
import math, sys

W, H = 56, 40
FEET = (21.5, 40)
OUTLINE = (20, 16, 20)
GROUND = 39  # the lowest row

PAL = [
    ('g', (152, 166, 168), 'skin, lit: grey-blue, dead'),
    ('G', (112, 124, 130), 'skin'),
    ('D', (78, 86, 98), 'skin, shade'),
    ('v', (60, 64, 78), 'hollows, the deep shade'),
    ('k', (24, 18, 22), 'sockets, its mouth'),
    ('e', (255, 54, 40), 'its one eye, burning red'),
    ('t', (206, 192, 156), 'teeth'),
    ('n', (206, 194, 164), 'bone: its spine through its back'),
    ('h', (38, 34, 34), 'hair, lank'),
    ('q', (128, 118, 98), 'the shirt, lit: torn, brown-grey'),
    ('c', (98, 88, 74), 'the shirt'),
    ('C', (70, 62, 54), 'the shirt, shade'),
    ('b', (46, 40, 38), 'the shirt, deep'),
    ('p', (72, 60, 92), 'trousers, dark purple'),
    ('P', (48, 40, 62), 'trousers, shade'),
]
CRIMSON = len(sys.argv) > 2 and sys.argv[2] == 'crimson'
if CRIMSON:
    PAL = [
        ('g', (196, 112, 96), 'skin, lit: flushed, raw'),
        ('G', (160, 78, 68), 'skin'),
        ('D', (108, 44, 48), 'skin, shade'),
        ('v', (92, 26, 40), 'veins, swollen; the hollows'),
        ('k', (30, 16, 18), 'sockets, its mouth'),
        ('e', (255, 214, 168), 'its eye, burning'),
        ('w', (234, 222, 186), 'teeth'),
        ('t', (214, 150, 140), 'tendrils'),
        ('r', (176, 34, 44), 'the maw, wet red'),
        ('R', (92, 12, 20), 'the maw, deep'),
        ('x', (126, 18, 20), 'blood'),
        ('n', (222, 196, 172), 'bone: its spine through its back'),
        ('h', (36, 26, 24), 'hair, lank'),
        ('q', (112, 110, 118), 'the shirt, lit: torn and filthy'),
        ('c', (88, 86, 96), 'the shirt'),
        ('C', (60, 58, 68), 'the shirt, shade'),
        ('b', (40, 34, 42), 'the shirt, deep'),
        ('p', (58, 50, 66), 'trousers'),
        ('P', (38, 32, 44), 'trousers, shade'),
    ]
COL = {k: c for k, c, _ in PAL}


def blank():
    return [['.'] * W for _ in range(H)]


def put(g, x, y, c):
    x, y = int(round(x)), int(round(y))
    if 0 <= x < W and 0 <= y < H:
        g[y][x] = c


# ---- the torso, drawn in place (the canvas's own pixels) ----
# Its back: a hump from its seat (left) up to its shoulders (right); the
# shirt over it hangs in a ragged flap behind (over its seat) and another
# in front (off its chest); its trousers show between.
TOP = {16: 28, 17: 26, 18: 24, 19: 22, 20: 20, 21: 19, 22: 18, 23: 17, 24: 17, 25: 16, 26: 16, 27: 15, 28: 15, 29: 15, 30: 15, 31: 15, 32: 16, 33: 17, 34: 18, 35: 19}
# where the shirt ends (the flaps' ragged hems)
HEM = {16: 30, 17: 32, 18: 31, 19: 33, 20: 30, 21: 31, 22: 26, 23: 25, 24: 25, 25: 25, 26: 25, 27: 25, 28: 25, 29: 28, 30: 30, 31: 29, 32: 31, 33: 28, 34: 25, 35: 22}
# the shirt's bottom over the body (above the flaps' hanging part)
BODY = {x: min(h, 26 if x <= 21 else (24 if x <= 28 else 23 - (x - 29) // 2)) for x, h in HEM.items()}
# trousers: its seat and the tops of its thighs, between the flaps
SEAT = {20: (27, 29), 21: (26, 30), 22: (26, 30), 23: (26, 30), 24: (26, 30), 25: (25, 30), 26: (25, 29), 27: (25, 29), 28: (26, 29)}
NUBS = {20: 1, 23: 2, 26: 2, 29: 2, 32: 1}
HOLES = [(18, 29), (21, 29), (31, 27), (32, 29)]
RIBS = [(29, 19), (29, 21), (31, 19), (31, 21)]


def torso():
    g = blank()
    for x, (y0, y1) in SEAT.items():
        for y in range(y0, y1 + 1):
            put(g, x, y, 'P' if y == y1 or x == 20 or (x + y) % 5 == 0 else 'p')
    for x, t in TOP.items():
        hem, body = HEM[x], BODY[x]
        for y in range(t, hem + 1):
            dt = y - t
            if y > body:
                # the flaps: cloth hanging in strands, darker to the hem
                c = 'b' if y == hem else ('C' if x % 2 else ('q' if y == body + 1 else 'c'))
            elif dt == 0:
                c = 'q'
            elif dt == 1:
                c = 'q' if x % 3 else 'c'
            elif body - y == 0:
                c = 'C'
            elif (x - y) % 5 == 0 and dt >= 2:
                # folds, down its side
                c = 'C'
            else:
                c = 'c'
            put(g, x, y, c)
    for (x, y) in HOLES:
        put(g, x, y, '.')
    # Its ribs through a tear in its side.
    for (x, y) in RIBS:
        put(g, x, y, 'D')
        put(g, x + 1, y, 'G')
    # Its spine through its back.
    for x, n in NUBS.items():
        for k in range(1, n + 1):
            put(g, x, TOP[x] - k, 'n')
    # The neck: grey, under where the head goes.
    for (x, y) in [(34, 18), (35, 19), (35, 20), (36, 20), (36, 21)]:
        put(g, x, y, 'D')
    return g


HEAD = [
    "...DGGGD...",
    "..hGgggggD.",
    ".hhGggggggG",
    "hhDGgggvvvG",
    "hhDGggvkekG",
    "h.hDGgDvkGg",
    "h.hDGgggGDk",
    "h..DGGDkttk",
    "...vDGDkkkk",
    "h...vDvkkt.",
    ".....DGGGt.",
    "......DDD..",
]
BLINK = list(HEAD)
BLINK[4] = "hhDGggvvvvG"
BLINK[5] = "h.hDGgDkkkg"
# the head's top-left in the stand pose, and its mouth
HEAD_AT = (32, 11)
MOUTH = (9, 8)
if CRIMSON:
    # Split down the middle: a seam of raw red from its crown to its brow,
    # tendril tips curling out of it; blood round its mouth.
    def split(rows):
        rows = [r.replace('t', 'w') for r in rows]
        out = ["....t.t....", ".....tht..."]
        for j, r in enumerate(rows):
            r = list(r)
            if j <= 5:
                r[4], r[5], r[6] = 'k', 'r', 'k'
            if j in (8, 9, 10):
                r = [('x' if c == 'k' and (i + j) % 2 else c) for i, c in enumerate(r)]
            out.append(''.join(r))
        return out
    HEAD, BLINK = split(HEAD), split(BLINK)
    HEAD_AT = (32, 9)
    MOUTH = (9, 10)
    # Bloomed open (the vomit's tell): four flaps of skull peeled back
    # from a red maw ringed with teeth, tendrils thrashing.
    BLOOM = [
        "t......hh....t..",
        ".t..hhGgg.hh.t..",
        "..tGggg...ggGt..",
        "GG.Ggg.rr.gG..GG",
        ".GgGgrRRRRrgGgG.",
        "..GgrRwRRwRrgG..",
        "...grRRRRRRrg...",
        "..GgrRwRRwRrgG..",
        ".GgGgrRRRRrgGgG.",
        "GG.GggrrrrggG.GG",
        "..t.GgggxggG.t..",
        "....GGgxxgGG....",
        ".....GxkxkG.....",
        "......GxGx......",
    ]


def crop(g):
    pts = [(x, y) for y in range(H) for x in range(W) if g[y][x] != '.']
    x0, x1 = min(p[0] for p in pts), max(p[0] for p in pts)
    y0, y1 = min(p[1] for p in pts), max(p[1] for p in pts)
    rows = [''.join(g[y][x0:x1 + 1]) for y in range(y0, y1 + 1)]
    return rows, (x0, y0)


# ---- arms, laid along a reach: shoulder, elbow, wrist; the hand's shape ----
UPPER, FORE = 9.0, 8.6


def elbow(s, w, bend=1):
    """The elbow of an arm from s reaching w (bend: which side)."""
    dx, dy = w[0] - s[0], w[1] - s[1]
    d = math.hypot(dx, dy)
    d = min(d, UPPER + FORE - 0.01)
    a = math.acos(max(-1, min(1, (UPPER ** 2 + d ** 2 - FORE ** 2) / (2 * UPPER * d))))
    base = math.atan2(dy, dx)
    ang = base + a * bend
    return (s[0] + UPPER * math.cos(ang), s[1] + UPPER * math.sin(ang))


# Hands, from the wrist (facing right, y down): what each pixel is.
HANDS = {
    # Flat on the ground, fingers spread ahead, nails down.
    'plant': ["Gg...", "DGgg.", "DGGgG", ".D.DD"],
    # Hanging, curled: claws under.
    'claw': ["Gg.", "GGg", "DGG", "D.D", "k.k"],
    # Thrown open ahead, reaching: long fingers, nails.
    'reach': [".Ggg.", "DGGGgk", ".DDGk.", "..D.."],
    # Up, fingers spread.
    'up': ["D.G.k", "D.G.D", "DGG.D", ".DGG.", "..Gg."],
}
# where on each hand's grid the wrist is
HAND_WRIST = {'plant': (0, 0), 'claw': (1, 0), 'reach': (0, 1), 'up': (2, 4)}


def stroke(g, a, b, wa, wb, lit_dir):
    """A limb bone a→b, wa..wb across, lit on the side toward lit_dir."""
    n = int(max(abs(b[0] - a[0]), abs(b[1] - a[1])) * 3) + 1
    dx, dy = b[0] - a[0], b[1] - a[1]
    L = math.hypot(dx, dy) or 1
    nx, ny = -dy / L, dx / L
    if nx * lit_dir[0] + ny * lit_dir[1] < 0:
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
                    s = across
                    old = cells.get((px, py))
                    if old is None or abs(s) < abs(old):
                        cells[(px, py)] = s
    for (px, py), s in cells.items():
        c = 'g' if s > 0.25 else ('G' if s > -0.45 else 'D')
        put(g, px, py, c)


def arm(s, w, hand, bend=1, lit=(-1, -1)):
    """An arm from shoulder s to wrist w, a hand of that shape; its grid
    in canvas pixels (cropped by the caller) and its hand point."""
    g = blank()
    e = elbow(s, w, bend)
    stroke(g, s, e, 2.6, 2.1, lit)
    stroke(g, e, w, 2.1, 1.7, lit)
    put(g, e[0], e[1], 'G')
    hx, hy = HAND_WRIST[hand]
    for j, row in enumerate(HANDS[hand]):
        for i, c in enumerate(row):
            if c != '.':
                put(g, w[0] + i - hx, w[1] + j - hy, c)
    return g, e


def place_part(g, pivot_canvas):
    rows, (x0, y0) = crop(g)
    px, py = int(round(pivot_canvas[0])) - x0, int(round(pivot_canvas[1])) - y0
    return rows, (px, py), (x0, y0)


parts = {}
poses = {}
NOTES = {}


def add_part(name, rows, pivot=(0, 0), points=None):
    parts[name] = (pivot, points or {}, rows)


T_ROWS, T_AT = crop(torso())
add_part('torso', T_ROWS)
add_part('head', HEAD, (0, 0), {'mouth': MOUTH})
add_part('blink', BLINK, (0, 0), {'mouth': MOUTH})
if CRIMSON:
    add_part('head_bloom', BLOOM, (0, 0), {'mouth': (11, 7)})

SH_FRONT = (32.5, 20.5)
SH_BACK = (29.5, 19.5)


def pose(name, body=(0, 0), head=(0, 0), front=None, back=None, head_part='head'):
    """A pose: the body shifted, the head shifted on it, each arm (wrist,
    hand) reached for (the canvas, before the body's shift is added)."""
    bx, by = body
    layers = []
    for tag, sh, spec, shade in (('back_arm', SH_BACK, back, 0.72), ('front_arm', SH_FRONT, front, 1.0)):
        wrist, hand = spec[0], spec[1]
        bend = spec[2] if len(spec) > 2 else 1
        s = (sh[0] + bx, sh[1] + by)
        w = (wrist[0], wrist[1])
        g, e = arm(s, w, hand, bend)
        rows, pivot, _ = place_part(g, s)
        pname = f"{tag.split('_')[0]}_{name}"
        hx, hy = HAND_WRIST[hand]
        # the hand point: the middle of the hand
        hp = (int(round(w[0])) - int(round(s[0])) + pivot[0] + 1, int(round(w[1])) - int(round(s[1])) + pivot[1] + 1)
        add_part(pname, rows, pivot, {'hand': hp})
        layers.append((tag, pname, (int(round(s[0])), int(round(s[1]))), shade))
    back_l, front_l = layers
    out = [
        dict(part=back_l[1], at=back_l[2], shade=back_l[3], tag='back_arm'),
        dict(part='torso', at=(T_AT[0] + bx, T_AT[1] + by)),
        dict(part=head_part, at=(HEAD_AT[0] + bx + head[0], HEAD_AT[1] + by + head[1])),
        dict(part=front_l[1], at=front_l[2], outline=True, tag='front_arm'),
    ]
    poses[name] = out


# Standing: hunched, the near hand planted ahead, knuckles down; the far
# one hanging.
pose('stand', front=((37, 36), 'plant'), back=((32, 35), 'claw'))
pose('breathe', body=(0, -1), head=(0, 0), front=((37, 36), 'plant'), back=((32, 34), 'claw'))
pose('blink', front=((37, 36), 'plant'), back=((32, 35), 'claw'), head_part='blink')
pose('twitch', head=(1, 1), front=((37, 36), 'plant'), back=((32, 35), 'claw'))

# The lurch: in step with its legs (frame k: its gait k/8 through its
# cycle, the near foot down at 0). A hand's planted as the opposite foot
# is (its far hand with its near foot), going back along the ground as the
# body goes on; then lifted, swung ahead, put down. It rises on its good
# (near) leg, and drops on its bad one, its head lolling after.
BOB = [0, -1, -1, 0, 0, 1, 2, 1]
HEADB = [(0, 0), (0, 0), (0, -1), (0, 0), (0, 0), (1, 0), (1, 1), (0, 1)]
FRONT = [((34, 35), 'claw'), ((35.5, 31), 'claw'), ((38.5, 31), 'reach'), ((40, 34), 'claw'),
         ((38.5, 36), 'plant'), ((37.2, 36), 'plant'), ((35.9, 36), 'plant'), ((34.6, 36), 'plant')]
BACK = [((35, 36), 'plant'), ((33.7, 36), 'plant'), ((32.4, 36), 'plant'), ((31.1, 36), 'plant'),
        ((31, 34), 'claw'), ((32.5, 30), 'claw'), ((35.5, 30), 'reach'), ((36.5, 33), 'claw')]
for k in range(8):
    pose(f'run_{k}', body=(0, BOB[k]), head=HEADB[k], front=FRONT[k], back=BACK[k])

# The grab's tell: it sinks back on its haunches, its head coming up,
# jaw wide, both hands drawn back to spring. The lunge: off its haunches
# at you, long, arms flung straight out. The bite: its head down and in.
pose('tell', body=(-1, 2), head=(0, -2), front=((28, 27), 'claw'), back=((25, 26), 'claw'))
pose('lunge', body=(3, -2), head=(2, 0), front=((48, 21), 'reach'), back=((45, 20), 'reach'))
pose('bite', body=(3, -1), head=(4, 2), front=((47, 25), 'reach'), back=((44, 24), 'reach'))
# Coming up out of the ground: arms straight up, reaching.
pose('climb', head=(0, -1), front=((36, 3), 'up', -1), back=((32, 4), 'up', -1))
# Rising: arms flung up and back. Falling: arms out, flailing. Dashing:
# arms back. On a wall (on its right): a hand up on it.
pose('rise', body=(0, -1), front=((28, 5), 'up', -1), back=((25, 7), 'up', -1))
pose('drop', front=((41, 29), 'claw'), back=((24, 30), 'claw'))
pose('dash', body=(2, 1), head=(1, 1), front=((24, 29), 'claw'), back=((22, 28), 'claw'))
pose('wall', front=((43, 8), 'up', 1), back=((32, 34), 'claw'))
if CRIMSON:
    # Its head bloomed open (the vomit's tell): up on its haunches, arms
    # dropped, the head thrown up. Spewing: leaning in at you, arms back.
    pose('bloom', body=(-1, 0), head=(-3, -5), front=((34, 34), 'claw'), back=((31, 34), 'claw'), head_part='head_bloom')
    pose('spew', body=(2, 1), head=(0, -2), front=((27, 30), 'claw'), back=((25, 29), 'claw'), head_part='head_bloom')

# The front arm at angles (raking): straight out, turned.
g, _ = arm((10, 10), (10 + UPPER + FORE - 0.5, 10.5), 'reach', -1)
rows, pivot, (x0, y0) = place_part(g, (10, 10))
hp = (int(round(10 + UPPER + FORE - 0.5)) - x0 + 1, 11 - y0)
add_part('aim', rows, pivot, {'hand': hp})


def ron():
    out = []
    w = out.append
    if CRIMSON:
        w("""// A crimson head: a zombie left to get up a second time, worse: the
// hunched zombie's drawing and reach (assets/art/zombie.ron, made by the
// same hand), its skin flushed dark raw red, veins swollen, its eye
// burning, and its head split down the middle, something's tendrils
// showing through the seam (a Plaga). To vomit, its head blooms open:
// four flaps of skull and scalp peeling back from a red maw ringed with
// teeth (`bloom`, `spew`). Its legs are its creature file's (legs.rs).
// The upright one, kept: zombie_crimson_classic. Facing right; the game
// mirrors it.
(""")
    else:
        w("""// A zombie: the dead, gone feral. Deeply hunched, almost on all fours:
// a bald grey-blue skull of a head thrust low ahead of it, one eye burning
// red, its jaw hanging open, lank hair; its spine through the back of a
// torn brown shirt; long thin arms, a hand planted on the ground ahead of
// it as it goes, knuckles down. Its legs aren't drawn here: they're its
// creature file's `legs` (legs.rs, `rig`: two bent legs in dark purple
// trousers, bare grey shins, from its seat 12 cells up, stepping on
// whatever's under them). Its run is in step with them (`in_step`): frame
// k is k/8 through its gait. Its arms are laid along a reach (shoulder,
// elbow, wrist) for each pose; a planted hand stays where it was put down
// as the body goes on over it. The upright zombie, kept: zombie_classic.
// Facing right; the game mirrors it.
(""")
    w(f"    size: ({W}, {H}),")
    w(f"    feet: ({FEET[0]}, {FEET[1]}),")
    w("    outline: (20, 16, 20),")
    w("    palette: {")
    for k, c, note in PAL:
        w(f"        '{k}': ({c[0]}, {c[1]}, {c[2]}),".ljust(36) + f"// {note}")
    w("    },")
    w("    parts: {")
    for name, (pivot, points, rows) in parts.items():
        pts = ', '.join(f'"{n}": ({x}, {y})' for n, (x, y) in points.items())
        head = f'        "{name}": (pivot: ({pivot[0]}, {pivot[1]}), points: {{ {pts} }}, rows: [' if points else f'        "{name}": (pivot: ({pivot[0]}, {pivot[1]}), rows: ['
        if NOTES.get(name):
            for line in NOTES[name]:
                w(f"        // {line}")
        w(head)
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
    w("""    // The front arm at angles (raking with its nails): the straight arm,
    // turned; shown instead of the pose's own front arm (`front_arm`) while
    // it aims. (The back arm is tagged too: its hand, `back_arm.hand`,
    // holds a torch.)
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
        "idle": (frames: ["stand", "stand", "breathe", "breathe", "stand", "stand", "breathe", "breathe", "stand", "blink", "stand", "twitch", "stand", "breathe", "breathe"], fps: 5),
        "run": (frames: ["run_0", "run_1", "run_2", "run_3", "run_4", "run_5", "run_6", "run_7"], fps: 10),
        "jump": (frames: ["rise"], fps: 5),
        "fall": (frames: ["drop"], fps: 5),
        "dash": (frames: ["dash"], fps: 5),
        "wall": (frames: ["wall"], fps: 5),
        "hurt": (frames: ["hurt"], fps: 5),
        "tell": (frames: ["tell"], fps: 5),
        "climb": (frames: ["climb"], fps: 5),
        "lunge": (frames: ["lunge"], fps: 5),
        "bite": (frames: ["lunge", "bite", "bite", "lunge"], fps: 6),""")
    if CRIMSON:
        w("""        "bloom": (frames: ["bloom", "bloom", "tell", "bloom"], fps: 8),
        "spew": (frames: ["spew"], fps: 5),""")
    w("""    },
)""")
    return '\n'.join(out) + '\n'


open(sys.argv[1], 'w').write(ron())
