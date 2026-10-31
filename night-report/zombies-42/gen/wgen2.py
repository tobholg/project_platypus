# Generates assets/art/zombie_warlock.ron from the user's reference
# (zombie-side-view-04.png) itself: cut out, mirrored to face right, shrunk
# to 44 cells high, its colours clustered (refq.py), lifted to read at
# night, cut into its hood (with the skull in it), robe, near arm and far
# arm; the legs dropped (legs.rs draws them). The poses move and turn the
# parts; the robe's skirt swings by the frame.
import json, sys, math
S_ = '/private/tmp/claude-501/-Users-tobias-Projects/071d8f99-2676-46a7-a86e-8237a5f7a424/scratchpad/'
d = json.load(open(S_ + 'zw/wq.json'))
REF, RPAL = [list(r) for r in d['rows']], d['pal']
RW, RH = d['size']

W, H = 62, 56
OX, OY = 10, 12
FEET = (24.5, 56)
OUTLINE = (10, 8, 14)

PURPLE = set('DFGIJKMNPQ')
LEGS = set('OREHLC')

# Cleaning: its eyes, two burning pixels in the hood's dark.
for (x, y) in [(20, 6), (22, 6)]:
    REF[y][x] = 'e'


def label(x, y):
    c = REF[y][x]
    if c == '.':
        return None
    if y <= 11 or (x >= 18 and y <= 13 and x <= 25):
        return 'head'
    if x >= 18 and 14 <= y <= 23:
        return 'far' if (y <= 16 or x >= 29 or (y == 17 and x >= 23)) else 'near'
    if y >= 34:
        return None
    if y >= 27 and c not in PURPLE:
        return None
    if y >= 27 and x >= 17:
        return None
    return 'robe'


def lift(c, k=1.5):
    return tuple(min(255, int(v * k)) for v in c)


NOTES = {'A': 'the hood\'s dark', 'B': 'the robe, deepest', 'C': 'boots, deep', 'D': 'the robe, deep', 'E': 'wrappings, dark',
         'F': 'the robe, shade', 'G': 'the robe: violet, shade', 'H': 'the belt, shade', 'I': 'the robe, shade', 'J': 'the robe: violet',
         'K': 'the robe', 'L': 'the belt: brown', 'M': 'the robe: violet, lit', 'N': 'the robe, lit', 'O': 'bone-grey skin, bandages',
         'P': 'the robe, lit', 'Q': 'the robe, highlight', 'R': 'skin, lit'}
PAL = [(k, lift(RPAL[k]), NOTES[k]) for k in sorted(RPAL)]
PAL.append(('e', (150, 255, 100), 'its eyes, and the grave-light it gathers: green, burning'))
COL = {k: c for k, c, _ in PAL}


def part_of(name, keep=None):
    pts = [(x, y) for y in range(RH) for x in range(RW) if label(x, y) == name]
    x0, x1 = min(p[0] for p in pts), max(p[0] for p in pts)
    y0, y1 = min(p[1] for p in pts), max(p[1] for p in pts)
    rows = [''.join(REF[y][x] if label(x, y) == name else '.' for x in range(x0, x1 + 1)) for y in range(y0, y1 + 1)]
    return rows, (x0, y0)


parts, poses = {}, {}


def add_part(name, rows, pivot=(0, 0), points=None):
    w = max(len(r) for r in rows)
    parts[name] = (pivot, points or {}, [r.ljust(w, '.') for r in rows])


def rel(p, o):
    return (p[0] - o[0], p[1] - o[1])


HOOD, HOOD0 = part_of('head')
ROBE, ROBE0 = part_of('robe')
NEAR, NEAR0 = part_of('near')
FAR, FAR0 = part_of('far')
SH_N, SH_F = (18, 18), (18, 15)
HAND_N, HAND_F = (27, 20), (31, 18)
FLARE = [r.replace('e', 'e') for r in HOOD]
# its eyes flaring: a cross of green round each
FL = [list(r) for r in HOOD]
for (x, y) in [(20, 6), (22, 6)]:
    for (dx, dy) in [(0, -1), (0, 1), (1, 0), (-1, 0)]:
        i, j = x + dx - HOOD0[0], y + dy - HOOD0[1]
        if 0 <= j < len(FL) and 0 <= i < len(FL[j]) and FL[j][i] != '.':
            FL[j][i] = 'e'
FLARE = [''.join(r) for r in FL]
BLINK = [r.replace('e', 'A') for r in HOOD]
add_part('head', HOOD)
add_part('head_flare', FLARE)
add_part('blink', BLINK)


def skirt(rows, sway):
    """The robe, its skirt (below the belt) swung `sway` cells at the hem."""
    out = []
    n = len(rows)
    top = 26 - ROBE0[1]
    for j, r in enumerate(rows):
        k = max(0, j - top) / max(1, n - 1 - top)
        dx = int(round(sway * k))
        r = ('.' * dx + r) if dx > 0 else (r[-dx:] + '.' * -dx if dx < 0 else r)
        out.append(r)
    w = max(len(r) for r in out)
    return [r.ljust(w, '.') for r in out]


robes = {}
for sway in (-2, -1, 0, 1, 2):
    name = 'robe' if sway == 0 else f"robe_{'l' if sway < 0 else 'r'}{abs(sway)}"
    add_part(name, skirt(ROBE, sway))
    robes[sway] = name
add_part('arm_near', NEAR, rel(SH_N, NEAR0), {'hand': rel(HAND_N, NEAR0)})
add_part('arm_far', FAR, rel(SH_F, FAR0), {'hand': rel(HAND_F, FAR0)})
add_part('orb', ["..eee..", ".eeeee.", "eeeeeee", "eeeeeee", "eeeeeee", ".eeeee.", "..eee.."], (3, 3))
ARM_ANGLE = -math.degrees(math.atan2(HAND_N[1] - SH_N[1], HAND_N[0] - SH_N[0]))


def at(p, o=(0, 0)):
    return (p[0] + OX + o[0], p[1] + OY + o[1])


def pose(name, body=(0, 0), head=(0, 0), near=0.0, far=0.0, head_part='head', sway=0, ndx=(0, 0), fdx=(0, 0), orb=None):
    b = body
    out = [
        dict(part='arm_far', at=at(SH_F, (b[0] + fdx[0], b[1] + fdx[1])), shade=0.8, turn=far, tag='back_arm'),
        dict(part=robes[sway], at=at(ROBE0, (b[0] + (min(0, sway) if sway < 0 else 0), b[1]))),
        dict(part=head_part, at=at(HOOD0, (b[0] + head[0], b[1] + head[1]))),
        dict(part='arm_near', at=at(SH_N, (b[0] + ndx[0], b[1] + ndx[1])), outline=True, turn=near, tag='front_arm'),
    ]
    if orb:
        out.append(dict(part='orb', at=at(orb, b)))
    poses[name] = out


# Standing: upright, both clawed hands held out ahead of it, reaching.
pose('stand')
pose('breathe', body=(0, -1), near=2, far=2)
pose('blink', head_part='blink')
# Its shamble, in step with its legs (frame k: k/8 through its gait, the
# near foot down at 0): it bobs as each foot passes under it, its hands
# dipping and rising out ahead of it, each against its own side's foot;
# its robe's skirt swinging after its legs.
BOB = [0, -1, -1, 0, 0, -1, -1, 0]
SWAY = [1, 1, 0, -1, -1, -1, 0, 1]
NR = [-4, -2, 2, 4, 2, 0, -2, -4]
FA = [3, 1, -2, -4, -2, 0, 2, 3]
for k in range(8):
    pose(f'run_{k}', body=(0, BOB[k]), head=((1 if k in (2, 6) else 0), 0), near=NR[k], far=FA[k], sway=SWAY[k])
# Casting (the tell): both hands up ahead of its face, clear of it, the
# grave-light gathering between them, its eyes flaring; then thrust out at
# you.
pose('gather', body=(0, -1), head=(-1, 0), near=32, far=40, head_part='head_flare', orb=(29, 6))
pose('gather_b', body=(0, -1), head=(-1, 0), near=36, far=36, head_part='head_flare', orb=(29, 5))
pose('throw', body=(1, 0), head=(1, 0), near=4, far=2, ndx=(2, 0), fdx=(2, 0), head_part='head_flare')
# Calling up the dead: arms flung up and wide, eyes flaring; then swept
# down at the ground.
pose('raise', body=(0, -1), head=(0, -1), near=40, far=95, head_part='head_flare', sway=-1)
pose('beckon', body=(0, 1), head=(1, 1), near=-55, far=-45, head_part='head_flare', sway=1)
# Drawn in (a shove's tell): hands to its chest.
pose('draw', body=(-1, 0), near=-15, far=-10, ndx=(-3, 0), fdx=(-3, 0))
# In the air: arms up; falling: arms out.
pose('rise', near=55, far=65)
pose('drop', near=-25, far=20, sway=-1)


def ron():
    out = []
    w = out.append
    w("""// A zombie warlock: one of the dead that kept its craft. Drawn from the
// user's reference (zombie-side-view-04.png) itself: cut out, mirrored to
// face right, shrunk to 44 cells high, its colours clustered and lifted
// to read at night, cut into its hood (peaked back to a point, a skull
// face deep in its shadow, green eyes), its robe (faded violet, darker
// folds, ragged tatters, a brown wrap at its waist) and its two sleeved
// arms reaching ahead, long bony claws; the poses move and turn those
// parts, its skirt swinging by the frame. Its legs aren't drawn here:
// they're its creature file's `legs` (legs.rs, `rig`: bandaged shins and
// wrapped boots from under its hem, striding), and its walk keeps in step
// with them (`in_step`: frame k is k/8 through its gait). Casting, its
// hands go up ahead of its face and the grave-light gathers between them,
// its eyes flaring (`gather`: the tell), then it thrusts them at you
// (`throw`); calling up the dead, its arms go up wide (`raise`), then
// sweep down at the ground (`beckon`). Facing right; the game mirrors it.
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
    w(f"""    // The near arm at angles (raking): the arm as drawn ({ARM_ANGLE:.0f} degrees),
    // turned; shown instead of the pose's own near arm (`front_arm`) while
    // it aims.
    fans: {{
        "front_arm": [""")
    for a in (-90, -60, -30, 0, 30, 60, 90):
        w(f'            (angle: {a}, part: "arm_near", from: {ARM_ANGLE:.0f}),')
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


if __name__ == '__main__':
    open(sys.argv[1], 'w').write(ron())
