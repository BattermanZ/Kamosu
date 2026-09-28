# The fonts under the binary

Kamosu self-hosts its two typefaces: an instance may never reach the internet,
so no external font request exists anywhere in the app. Latin and Latin-ext
subsets only, with their OFL licences alongside them.

## The `.woff2` files — what the browser loads

Zen Old Mincho (the display face) and Zen Kaku Gothic New (the interface face),
subset to Latin and Latin-ext, served at `/assets/fonts/` and named by the
`@font-face` rules in `ui/src/app.css`.

## The `.ttf` files — what the server draws with

The `.ttf` files are **the same subsets as the woff2 files**, decompressed. The
server sets text in two places, and neither reads woff2, which is a compressed
transport wrapper around exactly these tables:

- **The Share Card** (#65). The Share Link page composes the picture a
  messaging app shows (the recipe's photograph or its Cover, with the title set
  on it), and the server draws it with a glyph rasteriser that reads TrueType.
  It uses `zen-old-mincho-600-*` only, because the title is the only text on a
  card.
- **The Sheet** (#75, ADR 0023). A recipe set for paper by Typst, embedded in
  the binary, in Zen Old Mincho throughout, so it takes `zen-old-mincho-400-*`
  for text as well as `600` for titles and numerals. Typst finds a glyph in the
  Latin-ext half when the Latin half lacks it, so the pair is handed over as two
  files rather than merged into one.

So these are not second fonts. They are the fonts already shipping, in the form
the server can read, and they must stay in step with the woff2: the page, the
card and the Sheet have to set a title in the same face.

Regenerate them from the woff2 files with `fontTools`. woff2 → ttf is lossless,
so nothing is chosen here and nothing can drift in appearance:

```sh
python3 -m venv /tmp/fontenv && /tmp/fontenv/bin/pip install fonttools brotli
/tmp/fontenv/bin/python - <<'EOF'
from fontTools.ttLib import TTFont
for weight in ['400', '600']:
    for subset in ['latin', 'latin-ext']:
        name = f'zen-old-mincho-{weight}-{subset}'
        font = TTFont(f'assets/fonts/{name}.woff2')
        font.flavor = None
        font.save(f'assets/fonts/{name}.ttf')
EOF
```

Only the display face is converted. Nothing the server sets uses the interface
face, and everything else the server renders is HTML, where a browser loads the
woff2.
