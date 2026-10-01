"""Hand-drawn HD parts, put over the upscaled placeholders (by part name)."""
import re, sys
SHARED = {
 "arm": ["tLt","tLt","Ttt","ss.","ss.","Ss.","Ss.","sss","sS."],
 "arm_fwd": ["tLt...","tLLt..",".ttt..","..sss.","...ss.","...Ss.","...sss","....sS"],
 "arm_back": ["...tLt","..tLLt","..ttt.",".sss..",".ss...","sS....","sss...","sS...."],
 "legs": [".pppppppp..",".qqqp.ppp..",".qqq..ppp..",".qqq..ppp..",".qqq..ppp..",".qqq..ppp..",".qqq..ppp..",".CCC..UUU..",".CCC..BBB..",".CCC..BBBB.",".CCCC.BBBBB"],
 "run1": ["...pppppp.....","..qqqqpppp....","..qqq..ppp....",".qqq....ppp...",".qqq.....ppp..","qqq.......ppp.","qqq.......ppp.","CCC.......UUU.","CCC.......BBB.","CC........BBBB","C.........BBBB"],
 "run2": ["...pppppp.....","...qqqppp.....","...qqq.ppp....","....qqq.ppp...",".....qqq.ppp..",".....CCC.ppp..",".....CCC.ppp..",".........UUU..",".........BBB..",".........BBBB.",".........BBBB."],
 "run3": ["...pppppp.....","...qqqpppp....","...qqq.ppp....","....qq.ppp....","....qq.ppp....","...CCC.ppp....","...CC..ppp....",".......UUU....",".......BBB....",".......BBBB...",".......BBBB..."],
 "run4": ["...pppppp.....","..ppppqqqq....","..ppp..qqq....",".ppp....qqq...",".ppp.....qqq..","ppp.......qqq.","ppp.......qqq.","UUU.......CCC.","BBB.......CCC.","BB........CCCC","B.........CCCC"],
}
# Boots: each column's lowest boot pixel is the sole.
for k in ["legs","run1","run2","run3","run4"]:
    rows=[list(r) for r in SHARED[k]]
    for x in range(len(rows[0])):
        for y in range(len(rows)-1,-1,-1):
            if rows[y][x] in "BC": rows[y][x]="o"; break
            if rows[y][x] != ".": break
    SHARED[k]=["".join(r) for r in rows]
PLAYER = {
 "head": ["...hhhhhh...",".hhjjjhhhh..","hhjjhhhhhhh.","hhhhhhhhhhhh","HHhhhhhsssss","HHhhSssHHss.","HHhSsssswks.","HHhSSssswkss",".HhSsssssssS","..HSSsssrr..","...SSSss...."],
 "blink": ["...hhhhhh...",".hhjjjhhhh..","hhjjhhhhhhh.","hhhhhhhhhhhh","HHhhhhhsssss","HHhhSssHHss.","HHhSssssssss","HHhSSsssSSss",".HhSsssssssS","..HSSsssrr..","...SSSss...."],
 "torso": ["..TTtsstt..",".TTTtsttLt.","TTTTtttLLt.","TTTttttLtt.","TTTtttttLt.","TTTttttttt.","bbbbbbbyyb.","TTTtttttLt.","TTTTTttttT."],
}
ORC = {
 "head": ["....hhH.......","....hHH.......","...sslllss....","..Sssllssss...",".SSssssssssss.",".SSsssKKKKKKs.","SSSsssssskkss.","sSSsssssssslss",".SSssssssssss.",".SSSsssssDDDD.","..SSsssssswsD.","...SSssssww..."],
 "blink": ["....hhH.......","....hHH.......","...sslllss....","..Sssllssss...",".SSssssssssss.",".SSsssKKKKKKs.","SSSsssssKKKss.","sSSsssssssslss",".SSssssssssss.",".SSSsssssDDDD.","..SSsssssswsD.","...SSssssww..."],
 "torso": [".TTttLtt...","TTTtbtLtt..","TTTttbtLt..","TTTtttbLt..","TTTttttbt..","TTTtttttb..","bbbbbbbyyy.","TTTtttLtt..","TTTTttttT.."],
}
EXTRA = {"player": "        'j': (126, 86, 56),     // hair, lit\n        'L': (104, 182, 124),   // tunic, lit\n        'o': (40, 28, 24),      // soles\n        'w': (238, 236, 240),   // eye white\n        'r': (176, 104, 96),    // mouth\n        'U': (98, 70, 52),      // boot cuff\n",
         "orc": "        'l': (140, 178, 94),    // skin, lit\n        'L': (156, 116, 76),    // leather, lit\n        'o': (32, 24, 20),      // soles\n        'U': (66, 52, 42),      // boot cuff\n"}
which = sys.argv[1]; path = sys.argv[2]
parts = {**SHARED, **(PLAYER if which == "player" else ORC)}
s = open(path).read()
s = s.replace("    parts: {", "", 0)
s = re.sub(r"(    palette: \{\n)", lambda m: m.group(1) + EXTRA[which], s, count=1)
for name, rows in parts.items():
    pat = re.compile(r'(\n(\s*)"' + re.escape(name) + r'": \([^\n]*rows: \[\n)(.*?)(\n\s*\]\),)', re.S)
    m = pat.search(s); assert m, name
    ind = m.group(2) + "    "
    s = s[:m.start(3)] + "\n".join(f'{ind}"{r}",' for r in rows) + s[m.end(3):]
s = s.replace("// The player: an", "// HD spike (PLATYPUS_HD=1): drawn at 1.5x (head, torso, arms, legs and\n// the run by hand; the rest upscaled, placeholders).\n// The player: an", 1)
s = s.replace("// An orc, drawn", "// HD spike (PLATYPUS_HD=1): drawn at 1.5x (head, torso, arms, legs and\n// the run by hand; the rest upscaled, placeholders).\n// An orc, drawn", 1)
open(path, "w").write(s)
