/* The chosen direction — Noren · tight — as a flat token block.
   The five other directions considered on 2026-08-26 were discarded; they survive
   only on the throwaway branch prototype/36-visual-identity. */

const CHOSEN = {
  name:'Noren · tight',
  covers:[['#1d2b4c','#f4efe3'],['#7d8b5c','#f4efe3'],['#b8474b','#f4efe3'],
          ['#3d5480','#f4efe3'],['#2f3a2c','#ece5d5'],['#5c4a3a','#f4efe3']],
  tileMark:`<svg width="30" height="30" viewBox="0 0 30 30"><rect width="30" height="30" rx="2" fill="#1d2b4c"/><text x="15" y="22" text-anchor="middle" font-family="'Zen Old Mincho',serif" font-size="19" fill="#f4efe3">醸</text></svg>`,
  vars:{
    /* colour */
    '--ground':'#f4efe3','--ground-2':'#ece5d5','--card':'#fffdf8','--ink':'#1a1a1c','--ink-2':'#5c5a55',
    '--rule':'#ded5c2','--rule-w':'1px',
    '--accent':'#1d2b4c','--on-accent':'#f4efe3','--support':'#b8474b','--support-2':'#7d8b5c',
    /* the cooking screen is the one place the ground changes */
    '--cook-ground':'#131c33','--cook-ink':'#f4efe3','--cook-ink-2':'#9aa8c4','--cook-rule':'#2c3a5c',
    '--cook-accent':'#f4efe3','--cook-on-accent':'#131c33','--cook-panel':'#1d2b4c',
    /* type */
    '--f-display':"'Zen Old Mincho',serif",'--f-ui':"'Zen Kaku Gothic New',system-ui,sans-serif",'--w-display':'600',
    '--s-step':'33px','--lh-step':'1.34','--ls-step':'-.015em',
    '--s-title':'27px','--lh-title':'1.16','--ls-title':'-.012em',
    '--s-line':'17px','--s-read':'12.5px','--s-body':'16px','--s-label':'10.5px','--ls-label':'.15em',
    /* shape */
    '--r-sm':'2px','--r-md':'2px','--r-lg':'2px','--r-pill':'2px','--tile-w':'168px','--hero-h':'282px',
  },
};
