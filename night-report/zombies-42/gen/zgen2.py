# Generates assets/art/zombie.ron (and, with `crimson`, zombie_crimson.ron)
# from the user's reference itself (zombie-side-view-02.png): cut out,
# mirrored to face right, shrunk to 39 cells high, its colours clustered
# (refq.py), cut into its head, torso, near arm (sleeve and all) and far
# arm (zseg.py's rules), cleaned by hand where the shrinking smudged it;
# the poses move and turn those parts.
import json, sys, math
sys.path.insert(0, '/private/tmp/claude-501/-Users-tobias-Projects/071d8f99-2676-46a7-a86e-8237a5f7a424/scratchpad/py')
S_ = '/private/tmp/claude-501/-Users-tobias-Projects/071d8f99-2676-46a7-a86e-8237a5f7a424/scratchpad/'
CRIMSON = len(sys.argv) > 2 and sys.argv[2] == 'crimson'
d = json.load(open(S_ + 'zw/zq.json'))
REF, RPAL = [list(r) for r in d['rows']], d['pal']
RW, RH = d['size']

W, H = 72, 50
OX, OY = 10, 11            # the reference's top-left on the canvas
FEET = (31.5, 50)
OUTLINE = (16, 13, 14)
S = 1.0

# Cleaning: what the shrinking smudged. Its eye: one burning pixel, a
# dark red ring; its jaw hanging, teeth in it.
FIX = {(41, 5): 'e', (40, 6): 'D', (41, 6): 'A', (40, 5): 'A'}
for p, c in FIX.items():
    REF[p[1]][p[0]] = c

SKIN = set('IJNPQR')
SHIRT = set('FHKMO')


def label(x, y):
    c = REF[y][x]
    if c == '.':
        return None
    if (x >= 34 and y <= 12) or (x >= 38 and y <= 16):
        return 'head'
    if x <= 13 and 16 <= y <= 27:
        return 'back'
    if x >= 31 and y >= 13:
        return 'front'
    if y >= 29 and x <= 13:
        return None
    if y >= 31:
        return None
    if y >= 27 and 18 <= x <= 30 and c not in SHIRT:
        return None
    if y >= 22 and c in SKIN:
        return None
    # (Crumbs the far arm held on to.)
    if x <= 8 and y >= 26:
        return None
    return 'torso'


def lift(c, k=1.18):
    return tuple(min(255, int(v * k)) for v in c)


# The palette: the reference's own colours (lifted a little: it's drawn on
# black, the game's night is darker).
NOTES = {'A': 'the deepest shade', 'B': 'shade', 'C': 'shade', 'D': 'its eye\'s ring, dark red', 'E': 'trousers, shade',
         'F': 'the shirt, deep', 'G': 'trousers: purple-black', 'H': 'the shirt, shade', 'I': 'skin, deep',
         'J': 'skin, shade', 'K': 'the shirt', 'L': 'skin and cloth, dull', 'M': 'the shirt, lit', 'N': 'skin',
         'O': 'the shirt, lit', 'P': 'skin, lit', 'Q': 'skin, lit', 'R': 'skin, highlight: grey, a blue-green in it'}
PAL = [(k, lift(RPAL[k]), NOTES[k]) for k in sorted(RPAL)]
PAL.append(('e', (255, 50, 40), 'its one eye, burning red'))
if CRIMSON:
    # Flushed raw red where the grey was; the shirt greyer.
    swap = {'I': (78, 30, 34), 'J': (104, 42, 44), 'L': (120, 56, 52), 'N': (136, 62, 56), 'P': (160, 80, 68), 'Q': (184, 100, 86), 'R': (204, 124, 108),
            'F': (52, 50, 56), 'H': (62, 60, 68), 'K': (74, 72, 82), 'M': (86, 84, 94), 'O': (98, 96, 106)}
    PAL = [(k, swap.get(k, c), n.replace('grey, a blue-green in it', 'flushed, raw') if k not in swap or k in 'IJLNPQR' else n) for k, c, n in PAL]
    PAL = [(k, (255, 214, 168), 'its eye, burning') if k == 'e' else (k, c, n) for k, c, n in PAL]
    PAL += [('r', (176, 34, 44), 'the maw, wet red'), ('X', (92, 12, 20), 'the maw, deep'), ('w', (234, 222, 186), 'teeth'),
            ('T', (214, 150, 140), 'tendrils'), ('x', (126, 18, 20), 'blood'), ('h', (36, 26, 24), 'hair')]
COL = {k: c for k, c, _ in PAL}


def part_of(name):
    pts = [(x, y) for y in range(RH) for x in range(RW) if label(x, y) == name]
    x0, x1 = min(p[0] for p in pts), max(p[0] for p in pts)
    y0, y1 = min(p[1] for p in pts), max(p[1] for p in pts)
    rows = [''.join(REF[y][x] if label(x, y) == name else '.' for x in range(x0, x1 + 1)) for y in range(y0, y1 + 1)]
    return rows, (x0, y0)


parts, poses = {}, {}


def add_part(name, rows, pivot=(0, 0), points=None):
    w = max(len(r) for r in rows)
    parts[name] = (pivot, points or {}, [r.ljust(w, '.') for r in rows])


HEAD, HEAD0 = part_of('head')
TORSO, TORSO0 = part_of('torso')
FRONT, FRONT0 = part_of('front')
BACK, BACK0 = part_of('back')
# pivots (the reference's pixels): the near shoulder, under its head; the
# far arm out of its back.
SH_F = (33, 14)
SH_B = (13, 17)
HAND_F = (41, 37)
HAND_B = (7, 25)
MOUTH = (42, 14)


def rel(p, o):
    return (p[0] - o[0], p[1] - o[1])


if CRIMSON:
    # Split down the middle: a seam of raw red from its crown to its
    # brow, tendrils out of it; blood round its jaw.
    HEAD = [list(r) for r in HEAD]
    for j in range(0, 4):
        for i, c in ((5, 'A'), (6, 'r'), (7, 'X' if j % 2 else 'A')):
            if i < len(HEAD[j]) and HEAD[j][i] != '.':
                HEAD[j][i] = c
    for (i, j) in [(7, 12), (8, 13), (6, 11), (8, 11)]:
        if j < len(HEAD) and i < len(HEAD[j]) and HEAD[j][i] != '.':
            HEAD[j][i] = 'x'
    HEAD = [''.join(r) for r in HEAD]
    HEAD = ["......T.T..", ".......TT.."] + HEAD
    HEAD0 = (HEAD0[0], HEAD0[1] - 2)
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
    # (Its letters into these: skin, maw, teeth, tendrils, blood.)
    tr = {'t': 'T', 'G': 'N', 'g': 'Q', 'R': 'X', 'k': 'A', 'h': 'h', 'r': 'r', 'w': 'w', 'x': 'x', '.': '.'}
    BLOOM = [''.join(tr[c] for c in r) for r in BLOOM]
    BLOOM = [''.join(row[int(i / 1.2)] for i in range(int(len(row) * 1.2))) for row in [BLOOM[int(j / 1.2)] for j in range(int(len(BLOOM) * 1.2))]]

BLINK = [r.replace('e', 'A') for r in HEAD]
add_part('head', HEAD, (0, 0), {'mouth': rel(MOUTH, HEAD0)})
add_part('blink', BLINK, (0, 0), {'mouth': rel(MOUTH, HEAD0)})
if CRIMSON:
    add_part('head_bloom', BLOOM, (0, 0), {'mouth': (13, 8)})
add_part('torso', TORSO)
add_part('arm_front', FRONT, rel(SH_F, FRONT0), {'hand': rel(HAND_F, FRONT0)})
add_part('arm_back', BACK, rel(SH_B, BACK0), {'hand': rel(HAND_B, BACK0)})
# The near arm's angle as drawn (degrees from level, facing right: down
# and ahead), for its fan.
ARM_ANGLE = -math.degrees(math.atan2(HAND_F[1] - SH_F[1], HAND_F[0] - SH_F[0]))


def at(p, o=(0, 0)):
    return (p[0] + OX + o[0], p[1] + OY + o[1])


def pose(name, body=(0, 0), head=(0, 0), front=0.0, back=0.0, head_part='head', fdx=(0, 0), bdx=(0, 0)):
    """The body shifted; the head shifted on it; each arm turned about
    its shoulder (degrees, + up and ahead), its shoulder moved too."""
    b = body
    poses[name] = [
        dict(part='arm_back', at=at(SH_B, (b[0] + bdx[0], b[1] + bdx[1])), shade=0.85, turn=back, tag='back_arm'),
        dict(part='torso', at=at(TORSO0, b)),
        dict(part=head_part, at=at(HEAD0, (b[0] + head[0], b[1] + head[1]))),
        dict(part='arm_front', at=at(SH_F, (b[0] + fdx[0], b[1] + fdx[1])), outline=True, turn=front, tag='front_arm'),
    ]


# Standing: the near hand down on the ground ahead, the far hanging back.
pose('stand')
pose('breathe', body=(0, -1), front=2, back=2)
pose('blink', head_part='blink')
pose('twitch', head=(1, 1), front=-3)
# The lurch, in step with its legs (frame k: k/8 through its gait, the
# near foot down at 0): the near hand planted as the far foot is (its
# turn going back as the body goes over it), then lifted and swung ahead;
# the far arm swinging against it. It rises on its good (near) leg and
# drops on its bad one, its head lolling after.
BOB = [0, -1, -1, 0, 0, 1, 2, 1]
HEADB = [(0, 0), (0, 0), (0, -1), (0, 0), (0, 0), (1, 0), (1, 1), (0, 1)]
FR = [-10, 6, 18, 12, 4, 0, -4, -8]
BK = [8, 4, 0, -6, -12, -6, 4, 10]
for k in range(8):
    pose(f'run_{k}', body=(0, BOB[k]), head=HEADB[k], front=FR[k], back=BK[k])
# The grab's tell: it sinks back on its haunches, its head coming up, the
# near arm drawn back and up to spring, the far one too. The lunge: off its
# haunches at you, long, both arms flung out ahead. The bite: its head
# down and in.
pose('tell', body=(-2, 2), head=(0, -2), front=-35, back=-15)
pose('lunge', body=(4, -2), head=(2, 0), front=62, back=10, fdx=(1, 0))
pose('bite', body=(4, -1), head=(4, 2), front=50, back=10, fdx=(1, 0))
# Coming up out of the ground: both arms straight up.
pose('climb', head=(0, -1), front=150, back=160, bdx=(16, -4))
# Rising: arms flung up. Falling: arms out. Dashing: arms back. On a wall:
# a hand up on it.
pose('rise', body=(0, -1), front=120, back=140, bdx=(14, -3))
pose('drop', front=30, back=-20)
pose('dash', body=(2, 1), head=(1, 1), front=-60, back=-10)
pose('wall', front=125, back=0)
if CRIMSON:
    # Its head bloomed open (the vomit's tell), the head thrown up, arms
    # dropped; spewing, leaning in at you.
    pose('bloom', body=(-1, 0), head=(-3, -5), front=-6, back=-8, head_part='head_bloom')
    pose('spew', body=(2, 1), head=(0, -2), front=-30, back=-20, head_part='head_bloom')


def ron():
    out = []
    w = out.append
    if CRIMSON:
        w("""// A crimson head: a zombie left to get up a second time, worse. The
// zombie's body (assets/art/zombie.ron: the user's reference, shrunk and
// cut into parts), its skin flushed raw red, its shirt greyer, and its own
// head split down the middle, something's tendrils showing through the
// seam (a Plaga); to vomit, the head blooms open: flaps of skull peeled
// back from a red maw ringed with teeth (`bloom`, `spew`). Its legs are
// its creature file's (legs.rs). Facing right; the game mirrors it.
(""")
    else:
        w("""// A zombie: the dead, gone feral. Drawn from the user's reference
// (zombie-side-view-02.png) itself: cut out, mirrored to face right,
// shrunk to 39 cells high (its back at a man's shoulder), its colours
// clustered, cut into its head (a long bald skull, a heavy brow, one red
// eye, the jaw hanging open, lank hair behind), its torso (its spine
// through a torn brown-grey shirt hanging in long tatters), its near arm
// (sleeve and all, reaching down to the ground ahead, fingers splayed) and
// its far arm (hanging back); then cleaned where the shrinking smudged it.
// The poses move and turn those parts. Its legs aren't drawn here:
// they're its creature file's `legs` (legs.rs, `rig`: bent wide in
// purple-black trousers, bare grey shins and feet, the far one trailing),
// and its run keeps in step with them (`in_step`: frame k is k/8 through
// its gait). The upright zombie, kept: zombie_classic. Facing right; the
// game mirrors it.
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
            if l.get('turn'):
                bits.append(f'turn: {l["turn"]}')
            if l.get('tag'):
                bits.append(f'tag: "{l["tag"]}"')
            w(f'            ({", ".join(bits)}),')
        w("        ],")
    w("    },")
    w(f"""    // The near arm at angles (raking with its nails): the arm as drawn
    // (reaching down ahead, {ARM_ANGLE:.0} degrees), turned; shown instead of the
    // pose's own near arm (`front_arm`) while it aims. (The far arm is
    // tagged too: its hand, `back_arm.hand`, holds a torch.)
    fans: {{
        "front_arm": [""")
    for a in (-90, -60, -30, 0, 30, 60, 90):
        w(f'            (angle: {a}, part: "arm_front", from: {ARM_ANGLE:.0f}),')
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


if __name__ == '__main__':
    open(sys.argv[1], 'w').write(ron())
