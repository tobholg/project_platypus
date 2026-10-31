# A reference cut out of its background and shrunk to `rows` high, its
# colours clustered to k: saved as a JSON grid (letters + palette) and a
# big gridded picture to segment by eye.
import sys, json, numpy as np
from PIL import Image, ImageDraw
from collections import deque
src, out, rows, k = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
im = np.asarray(Image.open(src).convert('RGB')).astype(float)
h, w, _ = im.shape
lum = im.max(axis=2)
bg = np.zeros((h, w), bool)
q = deque([(0, x) for x in range(w)] + [(h - 1, x) for x in range(w)] + [(y, 0) for y in range(h)] + [(y, w - 1) for y in range(h)])
while q:
    y, x = q.popleft()
    if bg[y, x] or lum[y, x] > 22: continue
    bg[y, x] = True
    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        yy, xx = y + dy, x + dx
        if 0 <= yy < h and 0 <= xx < w and not bg[yy, xx]:
            q.append((yy, xx))
fg = ~bg
ys, xs = np.where(fg)
y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
crop = im[y0:y1, x0:x1]; m = fg[y0:y1, x0:x1].astype(float)
ch, cw = crop.shape[:2]
cols = int(round(cw * rows / ch))
def shrink(a):
    return np.asarray(Image.fromarray(a.astype(np.float32)).resize((cols, rows), Image.BOX))
cov = shrink(m)
rgb = np.stack([shrink(crop[..., i] * m) for i in range(3)], -1) / np.maximum(cov, 1e-6)[..., None]
alpha = cov > 0.42
pix = rgb[alpha]
rng = np.random.default_rng(3)
c = pix[rng.choice(len(pix), k, replace=False)]
for _ in range(60):
    d = ((pix[:, None] - c[None]) ** 2).sum(-1); lab = d.argmin(1)
    for i in range(k):
        if (lab == i).any(): c[i] = pix[lab == i].mean(0)
order = np.argsort(c.sum(1)); c = c[order]
d = ((rgb[..., None, :] - c[None, None]) ** 2).sum(-1); lab = d.argmin(-1)
L = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'
grid = [''.join(L[lab[y, x]] if alpha[y, x] else '.' for x in range(cols)) for y in range(rows)]
json.dump({'rows': grid, 'pal': {L[i]: [int(v) for v in c[i]] for i in range(k)}, 'size': [cols, rows]}, open(out + '.json', 'w'), indent=0)
z = 18
img = Image.new('RGB', (cols * z + 30, rows * z + 30), (230, 230, 236))
dr = ImageDraw.Draw(img)
for y in range(rows):
    for x in range(cols):
        if alpha[y, x]:
            dr.rectangle([30 + x * z, 30 + y * z, 30 + x * z + z - 1, 30 + y * z + z - 1], fill=tuple(int(v) for v in c[lab[y, x]]))
for x in range(cols + 1):
    dr.line([(30 + x * z, 30), (30 + x * z, 30 + rows * z)], fill=(60, 60, 70) if x % 5 == 0 else (190, 190, 200))
    if x % 5 == 0: dr.text((32 + x * z, 4), str(x), fill=(0, 0, 0))
for y in range(rows + 1):
    dr.line([(30, 30 + y * z), (30 + cols * z, 30 + y * z)], fill=(60, 60, 70) if y % 5 == 0 else (190, 190, 200))
    if y % 5 == 0: dr.text((2, 32 + y * z), str(y), fill=(0, 0, 0))
img.save(out + '_grid.png')
print(cols, rows)
for i in range(k):
    print(L[i], [int(v) for v in c[i]])
