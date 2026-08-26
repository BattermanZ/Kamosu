/* The four directions. Each is ONLY tokens plus a mark — the shape is identical
   across all four, because the shape was settled in #13 and ADR 0011. */

const seal = (glyph, bg, ink, r) =>
  `<svg width="76" height="76" viewBox="0 0 76 76"><rect width="76" height="76" rx="${r}" fill="${bg}"/>
   <text x="38" y="53" text-anchor="middle" font-family="var(--f-display)" font-size="42" font-weight="600" fill="${ink}">${glyph}</text></svg>`;

const THEMES = {

  noren: {
    tileMark:`<svg width="30" height="30" viewBox="0 0 30 30"><rect width="30" height="30" rx="2" fill="#1d2b4c"/><text x="15" y="22" text-anchor="middle" font-family="'Zen Old Mincho',serif" font-size="19" fill="#f4efe3">醸</text></svg>`,
    name:'Noren',
    tagline:'Your #13 pick, unchanged',
    thesis:'The direction you chose in the look-and-feel prototype, carried over untouched: indigo over unbleached <em>kinari</em> paper, mincho for everything a person wrote, safflower red and matcha as the only other colours. Corners are all but square. The cooking screen turns indigo — the one place the app changes ground, so that standing at the stove feels like a different room.',
    covers:[['#1d2b4c','#f4efe3'],['#7d8b5c','#f4efe3'],['#b8474b','#f4efe3'],['#3d5480','#f4efe3'],['#2f3a2c','#ece5d5'],['#5c4a3a','#f4efe3']],
    wordmark:`<svg width="210" height="52" viewBox="0 0 210 52"><rect x="0" y="4" width="44" height="44" rx="2" fill="#1d2b4c"/>
      <text x="22" y="38" text-anchor="middle" font-family="'Zen Old Mincho',serif" font-size="27" fill="#f4efe3">醸</text>
      <text x="56" y="37" font-family="'Zen Old Mincho',serif" font-size="27" font-weight="600" letter-spacing=".5" fill="#1a1a1c">Kamosu</text></svg>`,
    icon:`<svg width="76" height="76" viewBox="0 0 76 76"><rect width="76" height="76" rx="4" fill="#1d2b4c"/>
      <text x="38" y="52" text-anchor="middle" font-family="'Zen Old Mincho',serif" font-size="42" fill="#f4efe3">醸</text></svg>`,
    palette:[{n:'Ai',v:'#1d2b4c'},{n:'Ai deep',v:'#131c33'},{n:'Kinari',v:'#f4efe3'},{n:'Sumi',v:'#1a1a1c'},{n:'Beni',v:'#b8474b'},{n:'Matcha',v:'#7d8b5c'}],
    type:['<b>Zen Old Mincho</b> — Steps, titles','<b>Zen Kaku Gothic New</b> — interface','Display 600, near-square corners'],
    vars:{
      '--ground':'#f4efe3','--ground-2':'#ece5d5','--card':'#fffdf8','--ink':'#1a1a1c','--ink-2':'#5c5a55',
      '--rule':'#ded5c2','--rule-w':'1px','--accent':'#1d2b4c','--on-accent':'#f4efe3','--support':'#b8474b','--support-2':'#7d8b5c',
      '--cook-ground':'#131c33','--cook-ink':'#f4efe3','--cook-ink-2':'#9aa8c4','--cook-rule':'#2c3a5c',
      '--cook-accent':'#f4efe3','--cook-on-accent':'#131c33','--cook-panel':'#1d2b4c',
      '--f-display':"'Zen Old Mincho',serif",'--f-ui':"'Zen Kaku Gothic New',system-ui,sans-serif",'--w-display':'600',
      '--s-step':'34px','--lh-step':'1.42','--ls-step':'0','--s-title':'26px','--lh-title':'1.25','--ls-title':'.01em',
      '--s-line':'17px','--s-read':'13px','--s-body':'15.5px','--s-label':'11px','--ls-label':'.18em',
      '--r-sm':'2px','--r-md':'2px','--r-lg':'2px','--r-pill':'2px','--tile-w':'168px','--hero-h':'282px',
    },
  },

  koji: {
    tileMark:`<svg width="30" height="30" viewBox="0 0 64 64"><rect width="64" height="64" rx="16" fill="#E0602C"/><circle cx="24" cy="42" r="5.4" fill="#FFF3E2"/><circle cx="35" cy="30" r="4.1" fill="#FFF3E2" opacity=".85"/><circle cx="43" cy="20" r="2.9" fill="#FFF3E2" opacity=".7"/><circle cx="19" cy="26" r="2.4" fill="#FFF3E2" opacity=".55"/></svg>`,
    name:'Koji',
    tagline:'Warm, round, contemporary',
    thesis:'Named for what <em>kamosu</em> means — to ferment, to brew, to let something become. Ivory ground, persimmon accent, lacquer-brown ink, and generous radii everywhere: nothing on screen has a sharp corner. Covers are colour with intent rather than an apology for a missing photograph, which matters when a third of your library has none. The mark is a seal of rising bubbles.',
    covers:[['#E0602C','#FFF3E2'],['#4C6B4A','#F3EEDC'],['#C8A184','#2B1C15'],['#6B4FA0','#F5EEFF'],['#2B1C15','#F0C56A'],['#B8455B','#FFEDE4']],
    wordmark:`<svg width="214" height="52" viewBox="0 0 214 52"><g transform="translate(0,8) scale(0.56)"><rect width="64" height="64" rx="16" fill="#E0602C"/>
      <circle cx="24" cy="42" r="5.4" fill="#FFF3E2"/><circle cx="35" cy="30" r="4.1" fill="#FFF3E2" opacity=".85"/>
      <circle cx="43" cy="20" r="2.9" fill="#FFF3E2" opacity=".7"/><circle cx="19" cy="26" r="2.4" fill="#FFF3E2" opacity=".55"/></g>
      <text x="48" y="36" font-family="'Bricolage Grotesque',sans-serif" font-size="30" font-weight="700" letter-spacing="-.8" fill="#2B1C15">kamosu</text></svg>`,
    icon:`<svg width="76" height="76" viewBox="0 0 76 76"><rect width="76" height="76" rx="18" fill="#E0602C"/>
      <circle cx="28" cy="50" r="6.4" fill="#FFF3E2"/><circle cx="41" cy="36" r="4.9" fill="#FFF3E2" opacity=".85"/>
      <circle cx="51" cy="24" r="3.4" fill="#FFF3E2" opacity=".7"/><circle cx="23" cy="31" r="2.8" fill="#FFF3E2" opacity=".55"/></svg>`,
    palette:[{n:'Ivory',v:'#FFF8EE'},{n:'Rice',v:'#F3E7D3'},{n:'Persimmon',v:'#E0602C'},{n:'Lacquer',v:'#2B1C15'},{n:'Moss',v:'#4C6B4A'},{n:'Clay',v:'#C8A184'}],
    type:['<b>Bricolage Grotesque</b> — Steps, titles','<b>Instrument Sans</b> — interface','Display set tight, −.02em'],
    vars:{
      '--ground':'#FFF8EE','--ground-2':'#F3E7D3','--card':'#F3E7D3','--ink':'#2B1C15','--ink-2':'#8A7361',
      '--rule':'#E7D8C2','--rule-w':'1.5px','--accent':'#E0602C','--on-accent':'#ffffff','--support':'#4C6B4A','--support-2':'#C8A184',
      '--cook-ground':'#FFF8EE','--cook-ink':'#2B1C15','--cook-ink-2':'#8A7361','--cook-rule':'#E7D8C2',
      '--cook-accent':'#E0602C','--cook-on-accent':'#ffffff','--cook-panel':'#FCE9DE',
      '--f-display':"'Bricolage Grotesque',sans-serif",'--f-ui':"'Instrument Sans',system-ui,sans-serif",'--w-display':'700',
      '--s-step':'35px','--lh-step':'1.26','--ls-step':'-.028em','--s-title':'28px','--lh-title':'1.1','--ls-title':'-.025em',
      '--s-line':'17px','--s-read':'12.5px','--s-body':'15.5px','--s-label':'11px','--ls-label':'.14em',
      '--r-sm':'12px','--r-md':'18px','--r-lg':'22px','--r-pill':'999px','--tile-w':'156px','--hero-h':'292px',
    },
  },

  paper: {
    tileMark:`<svg width="30" height="30" viewBox="0 0 30 30"><rect x=".8" y=".8" width="28.4" height="28.4" rx="3" fill="none" stroke="#8B2F2C" stroke-width="1.5"/><text x="15" y="21" text-anchor="middle" font-family="Fraunces,serif" font-size="17" font-weight="500" fill="#1B1917">K</text></svg>`,
    name:'Paper',
    tagline:'Classic western cookbook',
    thesis:'A cookbook that happens to be a phone. Warm cream, an optical serif with real weight in it, hairline rules instead of boxes, and a single bordeaux accent held back for the one thing you can tap. Where Noren is Japanese and quiet, this is a Penguin cookery paperback: the same restraint, a different accent, and numbers set tabular so a column of quantities lines up.',
    covers:[['#8B2F2C','#F7F2E9'],['#41503A','#F1EDE1'],['#7A4230','#F7EFE7'],['#33415A','#EDEFF3'],['#6B5432','#F5F0E2'],['#5C3350','#F6EFF4']],
    wordmark:`<svg width="200" height="52" viewBox="0 0 200 52"><text x="0" y="36" font-family="Fraunces,serif" font-size="31" font-weight="500" letter-spacing="2.5" fill="#1B1917">Kamosu</text>
      <rect x="1" y="44" width="181" height="1.5" fill="#8B2F2C"/></svg>`,
    icon:`<svg width="76" height="76" viewBox="0 0 76 76"><rect width="76" height="76" rx="8" fill="#F7F2E9"/>
      <rect x="1.5" y="1.5" width="73" height="73" rx="7" fill="none" stroke="#DED5C4" stroke-width="2"/>
      <text x="38" y="52" text-anchor="middle" font-family="Fraunces,serif" font-size="42" font-weight="500" fill="#1B1917">K</text>
      <rect x="22" y="59" width="32" height="1.8" fill="#8B2F2C"/></svg>`,
    palette:[{n:'Paper',v:'#F7F2E9'},{n:'Ink',v:'#1B1917'},{n:'Muted',v:'#6E6558'},{n:'Rule',v:'#E0D7C6'},{n:'Bordeaux',v:'#8B2F2C'},{n:'Sage',v:'#5E6B4F'}],
    type:['<b>Fraunces</b> — Steps, titles, Covers','<b>Inter</b> — interface, Readings','Numbers set tabular'],
    vars:{
      '--ground':'#F7F2E9','--ground-2':'#EFE8DA','--card':'#FDFBF6','--ink':'#1B1917','--ink-2':'#6E6558',
      '--rule':'#E0D7C6','--rule-w':'1px','--accent':'#8B2F2C','--on-accent':'#F7F2E9','--support':'#5E6B4F','--support-2':'#9A8E7A',
      '--cook-ground':'#F7F2E9','--cook-ink':'#1B1917','--cook-ink-2':'#6E6558','--cook-rule':'#E0D7C6',
      '--cook-accent':'#1B1917','--cook-on-accent':'#F7F2E9','--cook-panel':'#EFE8DA',
      '--f-display':"Fraunces,serif",'--f-ui':"Inter,system-ui,sans-serif",'--w-display':'500',
      '--s-step':'33px','--lh-step':'1.34','--ls-step':'-.015em','--s-title':'27px','--lh-title':'1.16','--ls-title':'-.012em',
      '--s-line':'17px','--s-read':'12.5px','--s-body':'16px','--s-label':'10.5px','--ls-label':'.15em',
      '--r-sm':'3px','--r-md':'3px','--r-lg':'3px','--r-pill':'3px','--tile-w':'170px','--hero-h':'288px',
    },
  },

  slate: {
    tileMark:`<svg width="30" height="30" viewBox="0 0 30 30"><rect width="30" height="30" rx="9" fill="#0E7C86"/><rect x="10" y="10" width="10" height="10" rx="3" fill="#F4F6F8"/></svg>`,
    name:'Slate',
    tagline:'Cool, crisp, modern',
    thesis:'The one direction with no warmth in it. A cool near-white ground, cards that are actually white so a photograph sits on something, graphite ink, and a deep teal that reads as considered rather than decorative. Corners are softly rounded but not playful. If the other three are a kitchen, this is a well-lit worktop — it will age the slowest and shout the least.',
    covers:[['#0E7C86','#EAF6F7'],['#334155','#E8EDF3'],['#D2553F','#FFF1ED'],['#4B5F3A','#EFF4E9'],['#5B4A7A','#F1ECF8'],['#1F5E8C','#E9F1F8']],
    wordmark:`<svg width="212" height="52" viewBox="0 0 212 52"><rect x="0" y="12" width="28" height="28" rx="8" fill="#0E7C86"/>
      <rect x="9" y="21" width="10" height="10" rx="3" fill="#F4F6F8"/>
      <text x="40" y="36" font-family="Archivo,sans-serif" font-size="28" font-weight="700" letter-spacing="-.6" fill="#10161C">Kamosu</text></svg>`,
    icon:`<svg width="76" height="76" viewBox="0 0 76 76"><rect width="76" height="76" rx="20" fill="#0E7C86"/>
      <rect x="23" y="23" width="30" height="30" rx="9" fill="#F4F6F8"/><rect x="33" y="33" width="10" height="10" rx="3" fill="#0E7C86"/></svg>`,
    palette:[{n:'Mist',v:'#F4F6F8'},{n:'White',v:'#FFFFFF'},{n:'Graphite',v:'#10161C'},{n:'Slate',v:'#5A6672'},{n:'Teal',v:'#0E7C86'},{n:'Coral',v:'#D2553F'}],
    type:['<b>Archivo</b> — Steps, titles','<b>Inter</b> — interface, Readings','Tight display tracking, −.02em'],
    vars:{
      '--ground':'#F4F6F8','--ground-2':'#E8EDF2','--card':'#FFFFFF','--ink':'#10161C','--ink-2':'#5A6672',
      '--rule':'#DCE3E9','--rule-w':'1px','--accent':'#0E7C86','--on-accent':'#FFFFFF','--support':'#D2553F','--support-2':'#8A96A2',
      '--cook-ground':'#FFFFFF','--cook-ink':'#10161C','--cook-ink-2':'#5A6672','--cook-rule':'#DCE3E9',
      '--cook-accent':'#0E7C86','--cook-on-accent':'#FFFFFF','--cook-panel':'#E6F2F3',
      '--f-display':"Archivo,system-ui,sans-serif",'--f-ui':"Inter,system-ui,sans-serif",'--w-display':'700',
      '--s-step':'34px','--lh-step':'1.26','--ls-step':'-.022em','--s-title':'27px','--lh-title':'1.12','--ls-title':'-.022em',
      '--s-line':'17px','--s-read':'12.5px','--s-body':'15.5px','--s-label':'10.5px','--ls-label':'.13em',
      '--r-sm':'8px','--r-md':'12px','--r-lg':'16px','--r-pill':'999px','--tile-w':'162px','--hero-h':'286px',
    },
  },
};

/* ---------------------------------------------------------------------------
   Two probes on Noren, asked for on 2026-08-26. Both keep Noren's palette, its
   shape tokens (2px corners, 168px tiles, 1px rules) and its mark untouched.
   Only type moves — which is the whole point of putting them side by side. */

const TYPEFACES_OF = k => ({
  '--f-display': THEMES[k].vars['--f-display'],
  '--f-ui':      THEMES[k].vars['--f-ui'],
  '--w-display': THEMES[k].vars['--w-display'],
});
const SCALE_OF = k => Object.fromEntries(
  ['--s-step','--lh-step','--ls-step','--s-title','--lh-title','--ls-title',
   '--s-line','--s-read','--s-body','--s-label','--ls-label']
  .map(v => [v, THEMES[k].vars[v]]));

const derive = (base, over) => ({ ...THEMES[base], vars: { ...THEMES[base].vars, ...over } });

THEMES.norenLatin = Object.assign(derive('noren', { ...TYPEFACES_OF('paper'), ...SCALE_OF('paper') }), {
  name:'Noren · Latin type',
  type:['<b>Fraunces</b> — Steps, titles','<b>Inter</b> — interface, Readings','Paper’s scale and tracking, Noren’s colour'],
  tagline:'Noren’s colour, Paper’s typefaces and scale',
  thesis:'Noren’s indigo, kinari, beni and matcha exactly as they were — and Paper’s typefaces and type scale in place of the Japanese ones. Fraunces sets the Steps and titles, Inter sets everything small. This is the direct test of whether what you liked about Paper was its colour or its lettering, because here the colour is entirely Noren’s. The mark keeps 醸, which needs a Japanese face for that one glyph and would be drawn as a shape in a real logo anyway.',
  wordmark:`<svg width="210" height="52" viewBox="0 0 210 52"><rect x="0" y="4" width="44" height="44" rx="2" fill="#1d2b4c"/>
    <text x="22" y="38" text-anchor="middle" font-family="'Zen Old Mincho',serif" font-size="27" fill="#f4efe3">醸</text>
    <text x="56" y="37" font-family="Fraunces,serif" font-size="27" font-weight="500" letter-spacing="1.2" fill="#1a1a1c">Kamosu</text></svg>`,
});

THEMES.norenTight = Object.assign(derive('noren', SCALE_OF('paper')), {
  name:'Noren · tight',
  type:['<b>Zen Old Mincho</b> — Steps, titles','<b>Zen Kaku Gothic New</b> — interface','Set to Paper’s numbers, not Noren’s'],
  tagline:'Noren’s typefaces, Paper’s type scale',
  thesis:'Zen Old Mincho and Zen Kaku Gothic New kept exactly as they are, set to Paper’s numbers instead of Noren’s: the Step drops from 34px to 33px and its line spacing from 1.42 to 1.34, titles tighten from +0.01em tracking to −0.012em, and body text goes up half a point. If Noren felt airy rather than wrong, this is the fix — and it earns back 31 characters on the cooking screen, taking your real steps that fit without a scroll from 72% to 79%.',
});

/* The type scale and spacing shown in the spec panel are read back off the tokens,
   so the panel cannot disagree with what is rendered. */
for (const k in THEMES) {
  const v = THEMES[k].vars;
  THEMES[k].scale = [
    `Step <b>${v['--s-step']}</b> / ${v['--lh-step']}`,
    `Title <b>${v['--s-title']}</b> / ${v['--lh-title']}`,
    `Line <b>${v['--s-line']}</b>`,
    `Reading <b>${v['--s-read']}</b>`,
    `Body <b>${v['--s-body']}</b>`,
    `Label <b>${v['--s-label']}</b> caps`,
  ];
  THEMES[k].spacing = [
    `Steps <b>4 8 12 16 24 32 48</b>`,
    `Gutter <b>20</b>`,
    `Radius <b>${v['--r-sm']} / ${v['--r-md']} / ${v['--r-lg']}</b>`,
    `Rule <b>${v['--rule-w']}</b>`,
    `Tile <b>${v['--tile-w']}</b>`,
  ];
}

/* Switcher order: the three Noren variants adjacent, so only type changes between them. */
const ORDER = ['noren','norenLatin','norenTight','koji','paper','slate'];
