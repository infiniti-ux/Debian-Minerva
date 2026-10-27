#!/usr/bin/env python3
"""Regenerate the Aluminium OS reference gallery and contact sheet.

Usage:  python3 references/aluminium-os/render.py

Reads manifest.json + images/ and writes index.html + contact-sheet.jpg.
Only dependency: Pillow.
"""
from __future__ import annotations

import html
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageOps

ROOT = Path(__file__).resolve().parent
ITEMS = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))
GROUPS = list(dict.fromkeys(i["group"] for i in ITEMS))


# ---------------------------------------------------------------- contact sheet
def contact_sheet() -> None:
    cols, cell_w, img_h, label_h, pad = 3, 480, 300, 88, 18
    cell_h = img_h + label_h
    rows = (len(ITEMS) + cols - 1) // cols
    sheet = Image.new(
        "RGB",
        (pad + cols * (cell_w + pad), pad + rows * (cell_h + pad) + 66),
        (15, 17, 23),
    )
    d = ImageDraw.Draw(sheet)
    font_title = ImageFont.truetype(
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 30
    )
    font_sub = ImageFont.truetype(
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 17
    )
    font_small = ImageFont.truetype(
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 15
    )
    font_bold = ImageFont.truetype(
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 18
    )
    d.text((pad, 12), "Aluminium OS — leaked desktop previews", font=font_title, fill="#e8ecf5")
    d.text(
        (pad, 50),
        f"{len(ITEMS)} screenshots  ·  Jan 2026 bug report (ALOS / Android 16) + May 2026 Mystic Leaks (Android 17)",
        font=font_sub,
        fill="#8b93a7",
    )

    for n, item in enumerate(ITEMS):
        cx = pad + (n % cols) * (cell_w + pad)
        cy = pad + 66 + (n // cols) * (cell_h + pad)
        d.rectangle([cx, cy, cx + cell_w, cy + cell_h], fill=(28, 31, 41), outline=(52, 57, 72))
        im = Image.open(ROOT / item["file"]).convert("RGB")
        im = ImageOps.contain(im, (cell_w - 12, img_h - 12), Image.LANCZOS)
        sheet.paste(im, (cx + (cell_w - im.width) // 2, cy + 6 + (img_h - 12 - im.height) // 2))
        d.text((cx + 12, cy + img_h + 4), f"{n+1:02d}  {item['title']}", font=font_bold, fill="#e8ecf5")
        note = item["note"]
        if len(note) > 68:
            note = note[:65] + "..."
        d.text((cx + 12, cy + img_h + 30), note, font=font_small, fill="#9aa3b8")
        d.text((cx + 12, cy + img_h + 52), f"{item['width']}×{item['height']} px", font=font_small, fill="#707a90")

    sheet.save(ROOT / "contact-sheet.jpg", "JPEG", quality=88, optimize=True, progressive=True)
    print("wrote contact-sheet.jpg", sheet.size)


# ---------------------------------------------------------------------- gallery
def gallery() -> None:
    cards = []
    for n, item in enumerate(ITEMS):
        group_slug = "jan" if item["group"].startswith("Leak Jan") else "may"
        cards.append(
            f"""      <figure class="card" data-group="{group_slug}">
        <a class="thumb" href="{html.escape(item['file'])}" data-index="{n}">
          <img loading="lazy" src="{html.escape(item['file'])}" alt="{html.escape(item['title'])}">
        </a>
        <figcaption>
          <span class="num">{n+1:02d}</span>
          <h2>{html.escape(item['title'])}</h2>
          <p>{html.escape(item['note'])}</p>
          <div class="meta"><span>{item['width']}×{item['height']} px</span>
            <a href="{html.escape(item['source_url'])}" target="_blank" rel="noopener">sumber ↗</a></div>
        </figcaption>
      </figure>"""
        )

    sources = "\n".join(
        f'        <li><a href="{html.escape(u)}" target="_blank" rel="noopener">{html.escape(label)}</a></li>'
        for label, u in [
            ("9to5Google — Android desktop leak (Jan 2026)", "https://9to5google.com/2026/01/27/android-desktop-leak/"),
            ("9to5Google — desktop OS leak (Mei 2026)", "https://9to5google.com/2026/05/12/android-desktop-os-leak/"),
            ("Android Authority — massive leak (Mei 2026)", "https://www.androidauthority.com/google-aluminium-os-leak-3665979/"),
            ("The Verge — 16-minute leaked video", "https://www.theverge.com/tech/928598/google-aluminium-os-mystic-leaks-video"),
            ("Beebom — kumpulan screenshot", "https://gadgets.beebom.com/news/googles-aluminium-os-just-leaked-in-detail-ahead-of-google-io-2026"),
            ("Android Headlines — hands-on leak", "https://www.androidheadlines.com/2026/05/aluminum-os-just-leaked-in-a-16-minute-video-and-it-looks-a-lot-like-stock-android.html"),
            ("XDA Developers — analisis UI", "https://www.xda-developers.com/google-may-finally-have-the-right-laptop-os-but-it-picked-the-messiest-possible-path-to-get-there/"),
            ("Video asli Mystic Leaks (YouTube)", "https://www.youtube.com/watch?v=dXmFIfv_tIA"),
        ]
    )

    doc = f"""<!doctype html>
<html lang="id">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Aluminium OS — leaked desktop previews</title>
<style>
  :root {{ color-scheme: dark; }}
  * {{ box-sizing: border-box; }}
  body {{ margin:0; background:#0f1117; color:#e8ecf5;
         font:15px/1.55 ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,sans-serif; }}
  header {{ padding:40px 28px 24px; max-width:1400px; margin:auto; }}
  h1 {{ margin:0 0 8px; font-size:30px; letter-spacing:-.02em; }}
  header p {{ margin:0; color:#9aa3b8; max-width:820px; }}
  .bar {{ max-width:1400px; margin:0 auto; padding:0 28px 18px; display:flex; gap:10px; flex-wrap:wrap; }}
  button {{ background:#1c1f29; color:#c9d1e4; border:1px solid #343948; border-radius:999px;
            padding:7px 16px; cursor:pointer; font:inherit; font-size:13px; }}
  button:hover {{ border-color:#7aa2ff; }}
  button.active {{ background:#3b6fe0; border-color:#3b6fe0; color:#fff; }}
  main {{ max-width:1400px; margin:0 auto; padding:0 28px 60px;
          display:grid; grid-template-columns:repeat(auto-fill,minmax(330px,1fr)); gap:18px; }}
  .card {{ margin:0; background:#1c1f29; border:1px solid #343948; border-radius:14px; overflow:hidden; }}
  .card[hidden] {{ display:none; }}
  .thumb {{ display:block; aspect-ratio:16/10; background:#0b0d12; overflow:hidden; }}
  .thumb img {{ width:100%; height:100%; object-fit:contain; display:block; transition:transform .2s; }}
  .thumb:hover img {{ transform:scale(1.03); }}
  figcaption {{ padding:14px 16px 16px; }}
  .num {{ font-size:11px; color:#7aa2ff; font-weight:700; letter-spacing:.1em; }}
  h2 {{ font-size:16px; margin:4px 0 6px; }}
  figcaption p {{ margin:0 0 10px; font-size:13px; color:#9aa3b8; }}
  .meta {{ display:flex; justify-content:space-between; font-size:12px; color:#707a90; }}
  .meta a {{ color:#9ab6ff; text-decoration:none; }}
  footer {{ border-top:1px solid #262b38; padding:26px 28px 60px; max-width:1400px; margin:auto; }}
  footer h3 {{ font-size:14px; text-transform:uppercase; letter-spacing:.1em; color:#8b93a7; }}
  footer ul {{ columns:2; padding-left:18px; }}
  footer li {{ margin-bottom:6px; break-inside:avoid; }}
  footer a {{ color:#9ab6ff; }}
  #lightbox {{ position:fixed; inset:0; background:rgba(6,8,13,.94); display:none;
               align-items:center; justify-content:center; flex-direction:column; z-index:9; }}
  #lightbox.open {{ display:flex; }}
  #lightbox img {{ max-width:94vw; max-height:82vh; object-fit:contain; }}
  #lightbox .cap {{ color:#c9d1e4; font-size:13px; margin-top:12px; text-align:center; padding:0 20px; }}
  #lightbox .hint {{ color:#6f7a90; font-size:12px; margin-top:4px; }}
  #close {{ position:absolute; top:18px; right:22px; font-size:30px; color:#fff; cursor:pointer; background:none; border:0; }}
</style>
</head>
<body>
<header>
  <h1>Aluminium OS — leaked desktop previews</h1>
  <p>{len(ITEMS)} screenshot hasil leak: {html.escape(GROUPS[0])} dan {html.escape(GROUPS[1])}.
     Klik gambar untuk memperbesar. Semua file ada di folder <code>images/</code>.</p>
</header>
<div class="bar">
  <button class="active" data-filter="all">Semua ({len(ITEMS)})</button>
  <button data-filter="jan">Jan 2026 · ALOS / Android 16 ({sum(1 for i in ITEMS if i['group'].startswith('Leak Jan'))})</button>
  <button data-filter="may">Mei 2026 · Mystic Leaks / Android 17 ({sum(1 for i in ITEMS if i['group'].startswith('Leak Mei'))})</button>
</div>
<main>
{chr(10).join(cards)}
</main>
<footer>
  <h3>Sumber</h3>
  <ul>
{sources}
  </ul>
  <p style="color:#707a90;font-size:12px">Catatan: ini build development awal yang berjalan di VM; tampilan final bisa berubah.
  Virtual desktops, Recents view, task manager, dan gesture touchpad hanya terlihat di video leak, bukan di screenshot diam.</p>
</footer>
<div id="lightbox">
  <button id="close" aria-label="close">×</button>
  <img id="lbimg" src="" alt="">
  <div class="cap" id="lbcap"></div>
  <div class="hint">← → untuk navigasi · Esc untuk tutup</div>
</div>
<script>
const items = {json.dumps([{'file': i['file'], 'title': i['title'], 'note': i['note']} for i in ITEMS], ensure_ascii=False)};
const lb = document.getElementById('lightbox');
let cur = 0;
function show(n) {{
  cur = (n + items.length) % items.length;
  document.getElementById('lbimg').src = items[cur].file;
  document.getElementById('lbcap').textContent = (cur+1).toString().padStart(2,'0') + ' — ' + items[cur].title + ' · ' + items[cur].note;
  lb.classList.add('open');
}}
document.querySelectorAll('.thumb').forEach(a => a.addEventListener('click', e => {{
  e.preventDefault(); show(Number(a.dataset.index));
}}));
document.getElementById('close').onclick = () => lb.classList.remove('open');
lb.addEventListener('click', e => {{ if (e.target === lb) lb.classList.remove('open'); }});
document.addEventListener('keydown', e => {{
  if (!lb.classList.contains('open')) return;
  if (e.key === 'Escape') lb.classList.remove('open');
  if (e.key === 'ArrowRight') show(cur + 1);
  if (e.key === 'ArrowLeft') show(cur - 1);
}});
document.querySelectorAll('.bar button').forEach(btn => btn.addEventListener('click', () => {{
  document.querySelectorAll('.bar button').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  const f = btn.dataset.filter;
  document.querySelectorAll('.card').forEach(c => c.hidden = f !== 'all' && c.dataset.group !== f);
}}));
</script>
</body>
</html>
"""
    (ROOT / "index.html").write_text(doc, encoding="utf-8")
    print("wrote index.html")


if __name__ == "__main__":
    contact_sheet()
    gallery()
