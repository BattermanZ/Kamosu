/* PROTOTYPE switcher. Deliberately styled as a tool, not as part of any direction. */
const SCREENS = [
  {key:'home',        t:'Home — shelves that scroll sideways', d:'Carried from your #13 pick: shelves, a resume banner for a cook in progress, the Kitchen name holding settings, and no search anywhere near it.'},
  {key:'recipes',     t:'Recipes — where search lives',        d:'The other destination. ADR 0011 puts finding here so Home can answer “show me something”.'},
  {key:'recipe',      t:'A recipe led by its photograph',      d:'Ingredient Lines at full size, Kamosu’s Reading subordinate beneath. Two lines carry no Reading and are shown whole.'},
  {key:'cover',       t:'A recipe wearing a generated Cover',  d:'No photograph, a long title, 17 ingredients. The Cover comes from the Lineage id, so it survives a rename.'},
  {key:'cookAmounts', t:'Cooking — a step that adds an amount',d:'The Step is the largest type in the app. Above it, the amounts for this step and nothing else; the timer is read out of the sentence.'},
  {key:'cookEmpty',   t:'Cooking — a step that adds nothing',  d:'The panel’s empty state. It says so rather than vanishing, so the screen does not jump between steps.'},
  {key:'cookPlate',   t:'Cooking — an amount that is a Component', d:'“4 Chicken Katsu Cutlets” is a recipe of its own, appearing as an ordinary amount. A Section arrives mid-cook.'},
];

const KEYS = (typeof ORDER !== 'undefined' ? ORDER : Object.keys(THEMES)).filter(k=>THEMES[k]);
const params = new URLSearchParams(location.search);
let dKey = KEYS.includes(params.get('d')) ? params.get('d') : KEYS[0];
let lang = ['en','fr','es'].includes(params.get('lang')) ? params.get('lang') : 'en';
const esc = window.esc;

function sync(){ history.replaceState(null,'','?'+new URLSearchParams({d:dKey,lang})); render(); }

function render(){
  const D = THEMES[dKey], t = T[lang];
  SHAPE.setTheme(D);
  document.title = `Kamosu — ${D.name} (${lang.toUpperCase()}) — #36`;

  document.getElementById('ident').innerHTML = `
    <div class="ident-top">
      <div>
        <p class="ident-name">Direction ${KEYS.indexOf(dKey)+1} of ${KEYS.length} · ${esc(D.tagline)}</p>
        <h1 class="ident-title">${esc(D.name)}</h1>
        <p class="ident-thesis">${D.thesis}</p>
      </div>
      <div class="ident-marks">
        <div class="mark-slot"><span>Wordmark</span><div class="mark-box" style="--f-display:${D.vars['--f-display']}">${D.wordmark}</div></div>
        <div class="mark-slot"><span>App icon</span><div class="mark-box icon">${D.icon}</div></div>
      </div>
    </div>
    <div class="ident-specs">
      <div class="spec"><h4>Palette</h4><div class="swatches">${
        D.palette.map(c=>`<div class="sw"><i style="background:${c.v}"></i><b>${esc(c.n)}</b><em>${esc(c.v)}</em></div>`).join('')}</div></div>
      <div class="spec"><h4>Typefaces</h4><ul>${D.type.map(x=>`<li>${x}</li>`).join('')}</ul></div>
      <div class="spec"><h4>Type scale</h4><ul>${D.scale.map(x=>`<li>${x}</li>`).join('')}</ul></div>
      <div class="spec"><h4>Spacing &amp; shape</h4><ul>${D.spacing.map(x=>`<li>${x}</li>`).join('')}</ul></div>
    </div>`;

  const style = Object.entries(D.vars).map(([k,v])=>`${k}:${v}`).join(';');
  document.getElementById('stage').innerHTML = SCREENS.map((s,i)=>`
    <figure class="frame">
      <div class="phone"><div class="screen k" style="${style}">${SHAPE[s.key](t)}</div></div>
      <figcaption>
        <div class="cap-n">Screen ${i+1}</div>
        <div class="cap-t">${esc(s.t)}</div>
        <div class="cap-d">${s.d}</div>
      </figcaption>
    </figure>`).join('');

  document.getElementById('bar').innerHTML = `
    <button id="prev" title="Previous direction (←)">◀</button>
    <div class="cur"><b>${esc(D.name)}</b><small>${KEYS.indexOf(dKey)+1} / ${KEYS.length} · direction</small></div>
    <button id="next" title="Next direction (→)">▶</button>
    <div class="sep"></div>
    <div class="langs">${['fr','en','es'].map(l=>`<button data-l="${l}" aria-pressed="${l===lang}">${l}</button>`).join('')}</div>`;
  prev.onclick = ()=>cycle(-1);
  next.onclick = ()=>cycle(1);
  document.querySelectorAll('#bar .langs button').forEach(b=> b.onclick = ()=>{ lang=b.dataset.l; sync(); });
  window.scrollTo({top:0});
}

function cycle(n){ dKey = KEYS[(KEYS.indexOf(dKey)+n+KEYS.length)%KEYS.length]; sync(); }

addEventListener('keydown', e=>{
  if (/^(INPUT|TEXTAREA)$/.test(e.target.tagName) || e.target.isContentEditable) return;
  if (e.key==='ArrowLeft') cycle(-1);
  if (e.key==='ArrowRight') cycle(1);
  if (['1','2','3'].includes(e.key)) { lang = ['fr','en','es'][+e.key-1]; sync(); }
});

render();
