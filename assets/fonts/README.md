# The fonts under the binary

Kamosu self-hosts its two typefaces: an instance may never reach the internet,
so no external font request exists anywhere in the app. Latin and Latin-ext
subsets only, with their OFL licences alongside them.

## The `.woff2` files — what the browser loads

Zen Old Mincho (the display face) and Zen Kaku Gothic New (the interface face),
subset to Latin and Latin-ext, served at `/assets/fonts/` and named by the
`@font-face` rules in `ui/src/app.css`.

## The `.ttf` files — what the server draws with

`zen-old-mincho-600-latin.ttf` and `zen-old-mincho-600-latin-ext.ttf` are the
**same two subsets**, decompressed. They exist because #65's Share Link page
composes the picture a messaging app shows — the recipe's photograph, or its
Cover, with the title set on it — and that is drawn by the server rather than by
a browser. A glyph rasteriser reads TrueType; nothing reads woff2, which is a
compressed transport wrapper around exactly these tables.

So this is not a second font. It is the font already shipping, in the form the
server can read, and the pair must stay in step: the page and the page's own
card have to set a title in the same face.

Regenerate them from the woff2 files with `fontTools` — woff2 → ttf is lossless,
so nothing is chosen here and nothing can drift in appearance:

```sh
python3 -m venv /tmp/fontenv && /tmp/fontenv/bin/pip install fonttools brotli
/tmp/fontenv/bin/python - <<'PY'
from fontTools.ttLib import TTFont
for name in ['zen-old-mincho-600-latin', 'zen-old-mincho-600-latin-ext']:
    font = TTFont(f'assets/fonts/{name}.woff2')
    font.flavor = None
    font.save(f'assets/fonts/{name}.ttf')
PY
```

Only the display face at weight 600 is converted, because the title on a card is
the only text the server rasterises. Everything else the server renders is HTML,
and a browser loads the woff2.
