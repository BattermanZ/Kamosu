// The Sheet (#75, ADR 0023): one recipe set for paper.
//
// This file is layout and nothing else. Every word on the page arrives worded
// in `data.json` by `src/sheet.rs`, in the recipe's own Language, so there is
// no English here to translate and no arithmetic to drift from the screen's.
//
// The typography is "D · the cookbook card, with fine rules", chosen by
// Aurélien on 19 September 2026 against four treatments printed as real PDFs;
// the comparison and the reasoning are recorded on #75. What it settled:
//
//   · Zen Old Mincho throughout, 400 for text and 600 for titles and numerals —
//     paper is read, not tapped, so the interface face has no job here;
//   · a centred head (title, one meta line, a short rule) over the ingredients
//     in a column beside the method, so a recipe usually stays on one page;
//   · a hairline under each Ingredient Line, because lines that wrap were
//     otherwise hard to tell apart;
//   · beni red for numerals, labels and rules. Indigo was tried and rejected:
//     on paper it cannot be told from black.

#let d = json("/data.json")
#let words = d.words

#let beni = rgb("#b8474b")
#let grey = luma(95)
#let hairline = luma(200)
#let mincho = "Zen Old Mincho"

#set document(title: d.recipe.title)
#set page(
  paper: d.paper,
  margin: (x: 17mm, top: 12mm, bottom: 15mm),
  footer: context {
    set text(size: 7.5pt, fill: grey)
    let here = counter(page).get().first()
    let total = counter(page).final().first()
    grid(
      columns: (1fr, auto, 1fr),
      align: (left, center, right),
      d.recipe.title, [#words.page #here #words.of #total], d.printed,
    )
  },
)
#set text(font: mincho, size: 11.5pt, fill: black, lang: d.language, fallback: true)
#set par(leading: 0.62em)

#let label-text(t) = text(size: 8pt, weight: 600, tracking: 0.25em, fill: beni, upper(t))
#let head(t) = block(sticky: true, below: 8pt, width: 100%, stroke: (bottom: 0.5pt + beni),
  inset: (bottom: 5pt), label-text(t))
#let ornament(w: 30mm) = align(center, box(width: w, line(length: 100%, stroke: 0.6pt + beni)))

// Where a Component's block begins, for the line that names it.
#let page-of(block) = context {
  let found = query(label("block-" + str(block)))
  if found.len() > 0 { str(found.first().location().page()) }
}

#let ingredients(lines) = {
  head(words.ingredients)
  for l in lines {
    if l.section {
      block(sticky: true, above: 10pt, below: 4pt, text(weight: 600, size: 10.5pt, l.text))
      continue
    }
    block(breakable: false, above: 0pt, below: 0pt, width: 100%, inset: (y: 4.5pt),
      stroke: (bottom: 0.4pt + hairline), par(leading: 0.5em, {
        text(size: 11pt, l.text)
        if l.block != none {
          linebreak()
          text(size: 8.5pt, fill: beni)[#l.names, #words.page_ref #page-of(l.block)]
        }
        if l.about != none {
          linebreak()
          text(size: 8.5pt, fill: grey, l.about)
        }
      }))
  }
}

#let method(steps) = {
  head(words.method)
  let n = 0
  for s in steps {
    if s.section {
      block(sticky: true, above: 12pt, below: 5pt, text(weight: 600, size: 11pt, s.text))
      continue
    }
    n += 1
    block(breakable: false, above: 0pt, below: 7pt,
      grid(columns: (20pt, 1fr),
        text(size: 11.5pt, weight: 600, fill: beni, str(n) + "."),
        text(size: 11.5pt, s.text)))
  }
}

#let body(r) = grid(columns: (35%, 1fr), column-gutter: 9mm,
  ingredients(r.ingredients), {
    method(r.steps)
    if r.note != none {
      v(4pt)
      pad(left: 20pt, text(size: 10pt, fill: grey, r.note))
    }
  })

#let title-block(title, meta, under) = align(center, {
  text(weight: 600, size: 26pt, title)
  v(-8pt)
  if meta != none { text(size: 10pt, meta) }
  if under != none {
    linebreak()
    text(size: 9pt, fill: beni, under)
  }
})

// ——— The recipe ———
#if d.recipe.photo {
  image("/photo.jpg", width: 100%, height: 32mm, fit: "cover")
  v(2pt)
}
#title-block(d.recipe.title, d.recipe.meta, d.recipe.scaled)
#v(2pt)
#ornament()
#v(8pt)
#body(d.recipe)

// ——— Its Components, parent first, each whole (ADR 0023) ———
#for (i, b) in d.blocks.enumerate() {
  v(10pt)
  if b.recipe == none {
    // A repeat Kamosu stopped at: the page says so in plain words, where the
    // block would have been, and sets no heading for a recipe it does not print.
    block(sticky: true, width: 100%, below: 10pt, {
      ornament(w: 100%)
      v(4pt)
      align(center, text(size: 10pt, fill: grey)[#b.lead #label("block-" + str(i))])
    })
    continue
  }
  block(sticky: true, width: 100%, below: 10pt)[
    #ornament(w: 100%)
    #v(4pt)
    #align(center)[
      #text(weight: 600, size: 26pt, b.title) #label("block-" + str(i))
      #v(-8pt)
      #text(size: 9.5pt, fill: grey, b.lead)
      #if b.meta != none [ \ #text(size: 10pt, b.meta) ]
    ]
  ]
  body(b.recipe)
}

// ——— Where this page came from ———
#v(1fr)
#block(breakable: false, width: 100%, {
  set text(size: 7.5pt, fill: grey)
  set align(center)
  ornament()
  v(1pt)
  d.provenance.join([ · ])
  linebreak()
  text(size: 5.5pt, d.fingerprint)
})
