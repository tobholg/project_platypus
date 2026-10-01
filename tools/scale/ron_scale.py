"""The 1.5x rescale (PLAN "SC"): scale numbers in a RON file by field name.

Rules: `field` -> factor. A field's value may be a number or a tuple of
numbers (each scaled). Integers stay integers (rounded); floats keep up to
two decimals. Prints each change; `--dry` writes nothing.

    python3 tools/scale/ron_scale.py RULES FILE... [--dry]

RULES names a set below. A field matches only as a whole word followed by
`:` (so `step` doesn't match `step_height`)."""
import math, re, sys

L, A, S = 1.5, 2.25, 1.5  # length, area, speed (and acceleration)
RULES = {
    "creature": {
        # lengths
        "size": L, "step_height": L, "jump_height": L, "safe_height": L, "aggro": L, "reach": L, "keep": L,
        "jump_to_reach": L, "near": L, "far": L, "hover": L, "hovers": L, "leash": L, "pounce_range": L,
        "range": L, "radius": L, "flee_range": L, "lift": L, "hips": L, "spread": L, "thick": L,
        # speeds and accelerations (cells/s, cells/s²)
        "run_speed": S, "dash_speed": S, "ground_accel": S, "ground_decel": S, "turn_accel": S, "air_accel": S,
        "gravity": S, "max_fall": S, "wall_slide_speed": S, "wall_jump_push": S, "fly_speed": S, "fly_accel": S,
        "swim_speed": S, "swim_accel": S, "knock": S, "lunge": S,
        # damage per cell fallen
        "per_cell": 1 / L,
    },
}
# A villager's `wander: 60` (cells round its home) — not the hunter's `wander: (..)`.
NUMBER_ONLY = {"creature": {"wander": L}}

# Emitters (sparks, `look`s): speeds, gravity and crackle travel; counts,
# sizes (finer sparks: the point), lives, spreads (radians) and drag stay.
LOOK = [("speed", S, None), ("gravity", S, None), ("jitter", S, None)]

# Rule sets with a context: (field, factor, context) — the field is scaled
# only inside text matching the context regex (None: anywhere). A field of
# "#0" scales the first positional number inside the context (`Knock(260)`).
CONTEXT = {
    # (A move's `knock: 1.3` is a multiplier of its weapon's: kept, by `min`.)
    "weapons": [("knock", S, None, 5), ("lunge", S, None), ("thrust", L, None), ("dive", S, None), ("slam", L, None),
                ("pickup", L, None)] + LOOK,
    "runes": [
        ("speed", S, r"(?:Bolt|Orb|Stream|Call)\([^)]*\)"), ("rate", A, r"Stream\([^)]*\)"),
        ("range", L, None), ("radius", L, None), ("width", L, None), ("height", L, None),
        ("lift", A, r"Well\([^)]*\)"), ("pull", A, r"Well\([^)]*\)"), ("grip", S, r"Well\([^)]*\)"),
        ("power", S, r"Force\([^)]*\)"), ("pull", S, r"Force\([^)]*\)"),
        ("cells", A, None), ("#0", S, r"Knock\(\d+\)"),
        ("speed", S, r"look: \(.*"), ("gravity", S, r"look: \(.*"), ("jitter", S, r"look: \(.*"),
        ("speed", S, r"^\s*burst: \(.*"), ("gravity", S, r"^\s*burst: \(.*"), ("jitter", S, r"^\s*burst: \(.*"),
    ],
    "items": [("area", L, r"Mine\([^)]*\)"), ("holds", A, r"Vessel\([^)]*\)"),
              ("speed", S, r"sparks: \(.*"), ("gravity", S, r"sparks: \(.*"), ("jitter", S, r"sparks: \(.*")],
    "gear": [("thrust", S, r"rocket: \([^)]*\)"), ("speed", S, r"rocket: \([^)]*\)"),
             ("length", L, r"hook: \(.*"), ("reel", S, r"hook: \(.*"), ("speed", S, r"hook: \(.*")],
    "tools": [("radius", L, None), ("throw_speed", S, None)],
    "tempo": [('"run_speed"', S, None), ('"ground_accel"', S, None), ('"ground_decel"', S, None), ('"turn_accel"', S, None),
              ('"air_accel"', S, None), ('"gravity"', S, None), ('"max_fall"', S, None), ('"jump_height"', L, None),
              ('"dash_speed"', S, None), ('"wall_slide_speed"', S, None), ('"wall_jump_push"', S, None)],
    "lighting": [("range", L, None)] + LOOK,
    "crafting": [("reach", L, None)],
    "loot": [("deeper_than", L, None)],
    "progression": [("#0", L, r"Depth\(\d+\)")],
    "lairs": [("depth", L, None)],
    # Liquids' and gases' sideways reach a tick (cells/tick), and a gas's blast.
    "materials": [("dispersion", S, None), ("radius", L, r"explodes: \([^)]*\)")],
    "life": [("depth", L, None), ("above", L, None)],
}

NUM = r"-?\d+(?:\.\d+)?"

def fmt(old: str, v: float) -> str:
    if "." not in old:
        # Half away from zero (22.5 -> 23): Python's round() would go to even.
        return str(int(math.floor(abs(v) + 0.5)) * (1 if v >= 0 else -1))
    r = round(v, 2)
    return str(int(r)) + ".0" if r == int(r) else f"{r:g}"

def scale_value(text: str, k: float) -> str:
    return re.sub(NUM, lambda m: fmt(m.group(0), float(m.group(0)) * k), text)

def apply(rules: dict, number_only: dict, s: str, changes: list) -> str:
    def one(field, k, tuples):
        nonlocal s
        val = rf"(?:{NUM}|\(\s*{NUM}\s*,\s*{NUM}\s*\))" if tuples else NUM
        pat = re.compile(rf"(?<![\w])({re.escape(field)}:\s*)({val})(?![\w.])")
        def rep(m):
            new = scale_value(m.group(2), k)
            if new != m.group(2):
                changes.append(f"{field}: {m.group(2)} -> {new}")
            return m.group(1) + new
        s = pat.sub(rep, s)
    for f, k in rules.items():
        one(f, k, True)
    for f, k in number_only.items():
        one(f, k, False)
    return s

def apply_context(rules, s, changes):
    for rule in rules:
        field, k, ctx = rule[:3]
        least = rule[3] if len(rule) > 3 else 0
        if field == "#0":
            def rep_ctx(m):
                t = m.group(0)
                return re.sub(NUM, lambda n: _note(changes, ctx, n.group(0), fmt(n.group(0), float(n.group(0)) * k)), t, count=1)
        else:
            val = rf"(?:{NUM}|\(\s*{NUM}\s*,\s*{NUM}\s*\))"
            pat = re.compile(rf"(?<![\w\"])({re.escape(field)}:\s*)({val})(?![\w.])")
            def rep_ctx(m, pat=pat, field=field, k=k):
                def one(f):
                    if least and abs(float(re.search(NUM, f.group(2)).group(0))) < least:
                        return f.group(0)
                    return f.group(1) + _note(changes, field, f.group(2), scale_value(f.group(2), k))
                return pat.sub(one, m.group(0))
        if ctx is None:
            s = rep_ctx(re.match(r"(?s).*", s)) if field != "#0" else s
        else:
            s = re.sub(ctx, rep_ctx, s, flags=re.M)
    return s

def _note(changes, field, old, new):
    if new != old:
        changes.append(f"{field}: {old} -> {new}")
    return new

if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if a != "--dry"]
    dry = "--dry" in sys.argv
    name = args[0]
    for path in args[1:]:
        src = open(path).read()
        changes = []
        if name in CONTEXT:
            out = apply_context(CONTEXT[name], src, changes)
        else:
            out = apply(RULES[name], NUMBER_ONLY.get(name, {}), src, changes)
        print(f"{path}: {len(changes)} changes" + ("" if not changes else "\n  " + "; ".join(changes)))
        if not dry:
            open(path, "w").write(out)
