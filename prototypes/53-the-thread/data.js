// PROTOTYPE — throwaway. Issue #53: how a forking Thread reads on a phone.
//
// Issue #18 already settled the general shape (a line that forks, Attempts
// hanging off it) and shipped it as direction C. What it never tested:
//   - more than two Branches
//   - a Branch forking off another Branch, not the trunk (a Copy of a Copy,
//     or a Translation of a Copy)
//   - real scale: legible with three Versions AND with thirty
// This fixture exists to stress exactly those three things.
//
// v1/v2 and Marc's m1/m2 are REAL, lifted from #18 (in turn lifted from
// Aurélien's own Crouton export). Everything after is invented but plausible,
// built to give each variant a genuine run of quiet saves to collapse or not.
//
// Vocabulary is CONTEXT.md's: Lineage, Version, Branch, Branch Point, Attempt.

export const ME = 'Aurélien';
export const MY_KITCHEN = 'Maison Batterman';
export const MARC = 'Marc';
export const MARC_KITCHEN = 'Chez Marc';
export const CAMILLE = 'Camille';
export const CAMILLE_KITCHEN = 'Chez Camille';

const V = (id, branch, parent, when, { name = null, changed = null, language = 'en' } = {}) => ({
  id,
  branch,
  parent,
  when,
  name,
  changed,
  language,
});

export const VERSIONS_ARR = [
  // ---- shared trunk ----
  V('v1', 'mine', null, '3 Mar 2026', { changed: 'Imported from mykoreankitchen.com' }),
  V('v2', 'mine', 'v1', '14 Mar 2026', { changed: 'Split it into sections so I stop scrolling past the sauce' }),

  // ---- mine, after the Branch Point (v2) ----
  V('v3', 'mine', 'v2', '9 Aug 2026', { name: 'Hotter second fry', changed: 'Second fry at 190 instead of 175 — the coating was going soft by the time it got to the table' }),
  V('v4', 'mine', 'v3', '11 Aug 2026'),
  V('v5', 'mine', 'v4', '12 Aug 2026'),
  V('v6', 'mine', 'v5', '15 Aug 2026', { changed: 'Added a pinch of MSG to the marinade, on Camille’s advice' }),
  V('v7', 'mine', 'v6', '16 Aug 2026'),
  V('v7b', 'mine', 'v7', '17 Aug 2026'),
  V('v8', 'mine', 'v7b', '18 Aug 2026'),
  V('v9', 'mine', 'v8', '19 Aug 2026'),
  V('v10', 'mine', 'v9', '20 Aug 2026', { name: 'Barbecue batch', changed: 'Doubled the batch for the barbecue — yields 8 instead of 4' }),
  V('v11', 'mine', 'v10', '21 Aug 2026'),
  V('v12', 'mine', 'v11', '22 Aug 2026'),
  V('v13', 'mine', 'v12', '23 Aug 2026', { changed: 'Went back to 175 for the second fry — 190 was catching before the middle pieces caught up' }),
  V('v14', 'mine', 'v13', '24 Aug 2026'),
  V('v15', 'mine', 'v14', '25 Aug 2026'),
  V('v16', 'mine', 'v15', '26 Aug 2026', { name: 'August version', changed: 'Wrote the marinade time down properly — I never used to time it' }),

  // ---- Marc's branch, forking from the trunk at v2 ----
  V('m1', 'marc', 'v2', '2 Jun 2026', { changed: 'Air fryer — no deep frying in a flat' }),
  V('m2', 'marc', 'm1', '28 Jul 2026', { changed: 'Cut the sugar right down and put chilli flakes in the sauce' }),
  V('m3', 'marc', 'm2', '3 Aug 2026'),
  V('m4', 'marc', 'm3', '10 Aug 2026', { name: 'Kid-friendly', changed: 'Dialled the chilli back down for the kids — up to Camille to make it spicy again if she wants' }),
  V('m5', 'marc', 'm4', '17 Aug 2026'),
  V('m6', 'marc', 'm5', '24 Aug 2026', { changed: 'Swapped the plain flour for cornflour — crisper in the air fryer' }),

  // ---- Camille's branch: a Translation forking off MARC's m4, not the trunk ----
  V('f1', 'camille', 'm4', '12 Aug 2026', { language: 'fr', changed: 'Translated Marc’s version into French for my mother' }),
  V('f2', 'camille', 'f1', '19 Aug 2026', { language: 'fr' }),
  V('f3', 'camille', 'f2', '22 Aug 2026', { name: 'Version de maman', changed: 'Ajouté un trait de nuoc mam à la place de la sauce soja — la touche de maman', language: 'fr' }),
  V('f4', 'camille', 'f3', '25 Aug 2026', { language: 'fr' }),
  V('f5', 'camille', 'f4', '26 Aug 2026', { changed: 'Poids en grammes au lieu des tasses', language: 'fr' }),
];

export const VERSIONS = Object.fromEntries(VERSIONS_ARR.map((v) => [v.id, v]));

export const BRANCHES = {
  mine: { id: 'mine', who: ME, kitchen: MY_KITCHEN, head: 'v16', letter: 'A', color: '--ai', label: 'Yours' },
  marc: { id: 'marc', who: MARC, kitchen: MARC_KITCHEN, head: 'm6', letter: 'M', color: '--beni', label: "Marc's" },
  camille: { id: 'camille', who: CAMILLE, kitchen: CAMILLE_KITCHEN, head: 'f5', letter: 'C', color: '--matcha', label: "Camille's (français)" },
};

// ---- Attempts ----
// An Attempt belongs to the Lineage, not a Branch (ADR 0005), and is pinned to
// the Version that was on screen when the cooking happened.
export const ATTEMPTS = [
  { who: ME, when: '21 Mar 2026', rating: null, version: 'v1', note: '' },
  { who: CAMILLE, when: '18 Apr 2026', rating: 4, version: 'v2', note: '' },
  { who: ME, when: '2 May 2026', rating: 3, version: 'v2', note: 'Soggy by the time everyone sat down.' },
  { who: MARC, when: '5 Jun 2026', rating: 4, version: 'm1', note: 'First air fryer go, a bit dry in the batch’s corners.' },
  { who: MARC, when: '30 Jul 2026', rating: 5, version: 'm2', note: 'Kids ate it without complaining about the heat, so tuned right.' },
  { who: ME, when: '9 Aug 2026', rating: 5, version: 'v3', note: 'Hotter second fry. Stayed crisp all the way through dinner.' },
  { who: CAMILLE, when: '13 Aug 2026', rating: 5, version: 'f1', note: 'Maman approved, said it tasted like her sister’s.' },
  { who: ME, when: '16 Aug 2026', rating: 4, version: 'v6', note: 'MSG version — nobody noticed but me, and I noticed.' },
  { who: ME, when: '20 Aug 2026', rating: 5, version: 'v10', note: 'Fed twelve people off one batch, nothing left.' },
];

ATTEMPTS.forEach((a, idx) => (a.idx = idx));

export const ATTEMPTS_BY_VERSION = ATTEMPTS.reduce((m, a) => {
  (m[a.version] ??= []).push(a);
  return m;
}, {});

// ---- generic tree helpers (every variant shares these; none of them may hardcode branch counts) ----

export function childrenOf(versionsArr, id) {
  return versionsArr.filter((v) => v.parent === id);
}

// Oldest-first chain for one branch head, walking parent pointers back to the root.
export function chainOf(versionsById, headId) {
  const chain = [];
  let cur = headId;
  while (cur) {
    chain.unshift(versionsById[cur]);
    cur = versionsById[cur].parent;
  }
  return chain;
}

// The last Version two Branches share, found by walking both parent chains —
// exactly what src/operations.rs will do for real. Exercised here so the
// prototype's fork markers are computed, never declared.
export function branchPoint(versionsById, headA, headB) {
  const chainA = new Set();
  for (let c = headA; c; c = versionsById[c].parent) chainA.add(c);
  for (let c = headB; c; c = versionsById[c].parent) if (chainA.has(c)) return c;
  throw new Error('damaged bundle: chains never meet');
}

export const stars = (n) => (n == null ? '—' : '★'.repeat(n) + '☆'.repeat(5 - n));

export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

// ---- the two datasets the switcher flips between ----
// SMALL: exactly the literal "legible with three Versions" case from the
// ticket — the shared trunk plus one continuation, no fork yet.
// LARGE: everything above — 27 Versions, three Branches, one Branch (Camille's)
// forking off another Branch (Marc's) rather than the trunk.

export function datasetFor(size) {
  if (size === 'small') {
    const ids = new Set(['v1', 'v2', 'v3']);
    const versionsArr = VERSIONS_ARR.filter((v) => ids.has(v.id));
    const branches = { mine: { ...BRANCHES.mine, head: 'v3' } };
    return { versionsArr, versionsById: Object.fromEntries(versionsArr.map((v) => [v.id, v])), branches };
  }
  return { versionsArr: VERSIONS_ARR, versionsById: VERSIONS, branches: BRANCHES };
}
