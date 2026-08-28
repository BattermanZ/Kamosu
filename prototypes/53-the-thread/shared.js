// PROTOTYPE — pieces every variant shares, so the only thing that differs
// between A, B and C is how the FORKING THREAD itself is laid out.

import { esc, stars, BRANCHES, childrenOf, ATTEMPTS } from './data.js';

export function header(count, branchCount) {
  return `<div class="top">
    <span class="wordmark">醸 Kamosu</span>
    <p class="eyebrow">Korean Fried Chicken — the Thread</p>
    <h1 class="title">The Thread</h1>
    <p class="lede">${count} Version${count === 1 ? '' : 's'} across ${branchCount} Branch${branchCount === 1 ? '' : 'es'}. Every save is a Version; nothing is ever rewritten. Tap any Version to read it, or cook from it.</p>
  </div>`;
}

export function versionSheetHtml(v) {
  const b = BRANCHES[v.branch];
  return `<div class="sheet-scrim" data-act="closeSheet"></div>
    <div class="sheet" role="dialog" aria-modal="true">
      <button class="sheet-close" data-act="closeSheet" aria-label="Close">✕</button>
      <h3>${v.name ? esc(v.name) : esc(b.who) + '’s save'}</h3>
      <p class="meta">${esc(b.who)} · ${v.when}${v.language === 'fr' ? ' · français' : ''}</p>
      <p class="body">${v.changed ? esc(v.changed) : 'Saved with nothing written down.'}</p>
      <button class="cta" data-act="closeSheet">Cook this Version</button>
    </div>`;
}

export function attemptChip(a) {
  return `<button class="att-chip" data-act="openAttempt" data-idx="${a.idx}" title="${esc(a.who)}, ${stars(a.rating)}">
    <span class="att-pot">🍲</span>${a.rating != null ? `<span class="att-stars">${stars(a.rating)}</span>` : ''}
  </button>`;
}

export function attemptSheetHtml(idx) {
  const a = ATTEMPTS[idx];
  return `<div class="sheet-scrim" data-act="closeSheet"></div>
    <div class="sheet" role="dialog" aria-modal="true">
      <button class="sheet-close" data-act="closeSheet" aria-label="Close">✕</button>
      <h3>${esc(a.who)} cooked this</h3>
      <p class="meta">${a.when} · ${stars(a.rating)}</p>
      ${a.note ? `<p class="body">${esc(a.note)}</p>` : `<p class="body">No notes written down.</p>`}
    </div>`;
}

// ---- generic tree walking, shared by every variant ----

// Follows a single-child chain from startId until a fork (>1 children) or a
// leaf (0 children). Every variant uses this instead of hardcoding depth.
export function walkSegment(startId, versionsArr, versionsById) {
  const chain = [];
  let cur = startId;
  while (cur) {
    chain.push(versionsById[cur]);
    const kids = childrenOf(versionsArr, cur);
    if (kids.length !== 1) return { chain, forkId: kids.length > 1 ? cur : null };
    cur = kids[0].id;
  }
  return { chain, forkId: null };
}

export function forkMarker(kids, branches) {
  const names = kids.map((k) => esc(branches[k.branch].who));
  const who = names.length === 2 ? `${names[0]} and ${names[1]}` : names.join(', ');
  return `<div class="forkmark">
    <p class="forkmark-t">⑂ ${who} part here</p>
    <p class="forkmark-s">Kamosu worked this out by walking both chains back until they met — this is the last Version they share.</p>
  </div>`;
}
