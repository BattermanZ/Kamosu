/* Renders the chosen direction across all seven screens in all three interface
   languages. There is no switcher and nothing to choose: the choice was made. */
const SCREENS = [
  ['home','Home — shelves that scroll sideways'],
  ['recipes','Recipes — where search lives'],
  ['recipe','A recipe led by its photograph'],
  ['cover','A recipe wearing a generated Cover'],
  ['cookAmounts','Cooking — a step that adds an amount'],
  ['cookEmpty','Cooking — a step that adds nothing'],
  ['cookPlate','Cooking — an amount that is a Component'],
];
const LANGS = [['en','English'],['fr','Français'],['es','Español']];

SHAPE.setTheme(CHOSEN);
const style = Object.entries(CHOSEN.vars).map(([k,v])=>`${k}:${v}`).join(';');

document.getElementById('note').innerHTML = `
  <p class="eyebrow">Kamosu · issue #36 · decided 26 August 2026</p>
  <h1>The visual identity: <b>Noren · tight</b></h1>
  <p class="lede">Chosen from six directions on 26 August 2026. This page is a
  <strong>record of a moment</strong>: it is what the chosen palette, typefaces, type
  scale and spacing looked like on the day they were chosen, applied to real recipes
  from the Crouton export. <strong>It is never updated.</strong> It is not a target to
  build against and it will not match the app — the screens here were borrowed from an
  earlier prototype to have something real to judge against, and the real screens are
  decided in #37 and the screen tickets. What is always current lives in #80: a tokens
  page rendered from the real stylesheet, which cannot drift because it is the code.</p>`;

document.getElementById('stage').innerHTML = LANGS.map(([l,label])=>`
  <section class="lang">
    <h2>${label}</h2>
    <div class="row">${SCREENS.map(([k,t])=>`
      <figure>
        <div class="phone"><div class="screen k" style="${style}">${SHAPE[k](T[l])}</div></div>
        <figcaption>${t}</figcaption>
      </figure>`).join('')}</div>
  </section>`).join('');
