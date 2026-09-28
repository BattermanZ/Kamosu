# Third-party notices

Kamosu is licensed under the GNU Affero General Public License v3.0 only (see
[LICENSE](LICENSE)). This file records third-party material that is either
**bundled in this repository** or **downloaded at runtime by an Operator**.
Only the first is redistributed by Kamosu.

Rust crates and npm packages compiled into the binary keep their own licences,
which their manifests declare. `just audit` checks those dependencies against
the advisory databases.

---

## Bundled in this repository

### Zen Old Mincho

Copyright 2021 The Zen Old Mincho Project Authors
(<https://github.com/googlefonts/zen-oldmincho>). Licensed under the SIL Open
Font License, Version 1.1. The full licence is in
[`assets/fonts/OFL-ZenOldMincho.txt`](assets/fonts/OFL-ZenOldMincho.txt).

### Zen Kaku Gothic New

Copyright 2022 The Zen Kaku Gothic Project Authors
(<https://github.com/googlefonts/zen-kakugothic>). Licensed under the SIL Open
Font License, Version 1.1. The full licence is in
[`assets/fonts/OFL-ZenKakuGothicNew.txt`](assets/fonts/OFL-ZenKakuGothicNew.txt).

**Modification notice.** Both families ship as Latin and Latin-ext subsets, in
woff2 for the browser and as decompressed TrueType for the server, which sets
text on Share Cards and Sheets. No glyph is altered. Neither family declares a
Reserved Font Name, so the subsets keep their names.
[`assets/fonts/README.md`](assets/fonts/README.md) says how they are made.

---

## Downloaded at runtime

**This is not distributed with Kamosu.** No model weights ship in the
repository, the container image or a Backup (ADR 0029). Meaning Search is off
until an Operator turns it on, and turning it on is where they accept the terms
below. The model is then fetched from Hugging Face onto their own machine.

### EmbeddingGemma 300M

Fetched from `onnx-community/embeddinggemma-300m-ONNX` at a pinned revision.
Governed by the **Gemma Terms of Use**, which is *not* an open-source licence
and carries a prohibited-use policy.

- Terms: <https://ai.google.dev/gemma/terms>
- Prohibited use policy: <https://ai.google.dev/gemma/prohibited_use_policy>

Anyone running Kamosu with Meaning Search on is bound by those terms. The
authoritative repository, revision, terms version and URLs are the constants in
`src/meaning.rs`; if they disagree with this file, the code wins.
