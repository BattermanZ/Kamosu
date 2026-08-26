/* PROTOTYPE data. Real content, lifted from Aurélien's 86-recipe Crouton export.
   Ingredient Lines are written as a person writes them (ADR 0002: the line is the truth);
   `reading` is Kamosu's subordinate Reading, and is deliberately null on some lines. */

const T = {
  en: {
    tabHome:'Home', tabRecipes:'Recipes', tabShopping:'Shopping', tabCooked:'Cooked',
    kitchen:'Chez Nous', search:'Search recipes',
    shelfMost:'Cooked most', shelfQuick:'Quick tonight', shelfNever:'Never cooked', shelfRecent:'Recently opened',
    ingredients:'Ingredients', method:'Method', notes:'Notes',
    serves:n=>`Serves ${n}`, prepCook:(p,c)=>`${p} min prep · ${c} min cook`,
    cookedTimes:n=>`Cooked ${n} times`, neverCooked:'Never cooked',
    from:'From', startCooking:'Start cooking', addToShopping:'Add to shopping list',
    stepOf:(a,b)=>`Step ${a} of ${b}`, forThisStep:'For this step',
    nothingNew:'Nothing new to add — just the pot',
    timer:m=>`Set a ${m}-minute timer`, timerRunning:m=>`${m} left`,
    next:'Next', back:'Back', finish:'Finish cooking', mins:n=>`${n} min`,
    component:'A recipe of its own', servesL:'SERVINGS', prepL:'PREP', cookL:'COOK',
    browseAll:n=>`Browse all ${n} recipes →`, stillCooking:'Still cooking', pickUp:'Pick up →',
    blurbMost:'The ones that earned their place', blurbQuick:'Under 35 minutes, start to plate',
    blurbNever:'Saved and still waiting', blurbRecent:'Where you left off',
    sortRecent:'Recently cooked', sortAZ:'A–Z', sortAdded:'Added',
    recipesCount:n=>`${n} recipes`, pause:'Pause', awake:'Screen stays awake',
    nextStep:'Next step', scale:'Scale', servingsN:n=>`${n} servings`,
    cookedH:'Cooked', timesLast:(n,w)=>`${n} times · last ${w}`,
    settingsHint:'Maison Batterman ⌄', minPrep:'min prep', minCook:'min cook', servingsL:'servings',
  },
  fr: {
    tabHome:'Accueil', tabRecipes:'Recettes', tabShopping:'Courses', tabCooked:'Cuisiné',
    kitchen:'Chez Nous', search:'Rechercher des recettes',
    shelfMost:'Les plus cuisinées', shelfQuick:'Rapide ce soir', shelfNever:'Jamais cuisinées', shelfRecent:'Ouvertes récemment',
    ingredients:'Ingrédients', method:'Préparation', notes:'Notes',
    serves:n=>`Pour ${n} personnes`, prepCook:(p,c)=>`${p} min de préparation · ${c} min de cuisson`,
    cookedTimes:n=>`Cuisinée ${n} fois`, neverCooked:'Jamais cuisinée',
    from:'Depuis', startCooking:'Commencer à cuisiner', addToShopping:'Ajouter à la liste de courses',
    stepOf:(a,b)=>`Étape ${a} sur ${b}`, forThisStep:'Pour cette étape',
    nothingNew:'Rien de nouveau à ajouter — juste la casserole',
    timer:m=>`Lancer un minuteur de ${m} minutes`, timerRunning:m=>`${m} restantes`,
    next:'Suivant', back:'Retour', finish:'Terminer la cuisson', mins:n=>`${n} min`,
    component:'Une recette à part entière', servesL:'PERSONNES', prepL:'PRÉPARATION', cookL:'CUISSON',
    browseAll:n=>`Voir les ${n} recettes →`, stillCooking:'Cuisson en cours', pickUp:'Reprendre →',
    blurbMost:'Celles qui ont fait leurs preuves', blurbQuick:'Moins de 35 minutes, de bout en bout',
    blurbNever:'Enregistrées et toujours en attente', blurbRecent:'Là où vous en étiez',
    sortRecent:'Cuisinées récemment', sortAZ:'A–Z', sortAdded:'Ajoutées',
    recipesCount:n=>`${n} recettes`, pause:'Pause', awake:'L’écran reste allumé',
    nextStep:'Étape suivante', scale:'Quantités', servingsN:n=>`${n} personnes`,
    cookedH:'Cuisiné', timesLast:(n,w)=>`${n} fois · dernière ${w}`,
    settingsHint:'Maison Batterman ⌄', minPrep:'min de préparation', minCook:'min de cuisson', servingsL:'personnes',
  },
  es: {
    tabHome:'Inicio', tabRecipes:'Recetas', tabShopping:'Compras', tabCooked:'Cocinado',
    kitchen:'Chez Nous', search:'Buscar recetas',
    shelfMost:'Las más cocinadas', shelfQuick:'Rápido esta noche', shelfNever:'Nunca cocinadas', shelfRecent:'Abiertas recientemente',
    ingredients:'Ingredientes', method:'Preparación', notes:'Notas',
    serves:n=>`Para ${n} personas`, prepCook:(p,c)=>`${p} min de preparación · ${c} min de cocción`,
    cookedTimes:n=>`Cocinada ${n} veces`, neverCooked:'Nunca cocinada',
    from:'Desde', startCooking:'Empezar a cocinar', addToShopping:'Añadir a la lista de la compra',
    stepOf:(a,b)=>`Paso ${a} de ${b}`, forThisStep:'Para este paso',
    nothingNew:'Nada nuevo que añadir — solo la olla',
    timer:m=>`Poner un temporizador de ${m} minutos`, timerRunning:m=>`quedan ${m}`,
    next:'Siguiente', back:'Atrás', finish:'Terminar de cocinar', mins:n=>`${n} min`,
    component:'Una receta en sí misma', servesL:'RACIONES', prepL:'PREPARACIÓN', cookL:'COCCIÓN',
    browseAll:n=>`Ver las ${n} recetas →`, stillCooking:'Cocinando ahora', pickUp:'Retomar →',
    blurbMost:'Las que se han ganado su sitio', blurbQuick:'Menos de 35 minutos, de principio a fin',
    blurbNever:'Guardadas y aún esperando', blurbRecent:'Donde lo dejaste',
    sortRecent:'Cocinadas hace poco', sortAZ:'A–Z', sortAdded:'Añadidas',
    recipesCount:n=>`${n} recetas`, pause:'Pausa', awake:'La pantalla sigue encendida',
    nextStep:'Paso siguiente', scale:'Cantidades', servingsN:n=>`${n} raciones`,
    cookedH:'Cocinado', timesLast:(n,w)=>`${n} veces · última ${w}`,
    settingsHint:'Maison Batterman ⌄', minPrep:'min de preparación', minCook:'min de cocción', servingsL:'raciones',
  },
};

const IMG = 'img/';

const KATSU = {
  slug:'katsu', lineage:'katsu-curry-8f2a',
  title:'Katsu Curry (Japanese Curry with Chicken Cutlet)',
  img: IMG+'katsu-curry-japanese-curry-with-chicken-.jpg',
  serves:4, prep:5, cook:20, source:'japan.recipetineats.com', cooked:6, lang:'en',
  ingredients:[
    {line:'400g / 0.9lb onion (sliced into 1cm / ⅜” wide pieces)', reading:{amt:'400 g', food:'onion'}},
    {line:'250g / 0.6lb potato (cut into 1.5cm / ⅝” cubes)', reading:{amt:'250 g', food:'potato'}},
    {line:'100g / 3.5oz carrot (sliced to 7mm / ¼” thick pieces, note 1)', reading:{amt:'100 g', food:'carrot'}},
    {line:'1 tbsp oil', reading:{amt:'1 tbsp', food:'oil'}},
    {line:'230g / 0.5lb House Vermont Curry (Mild, note 2)', reading:{amt:'230 g', food:'House Vermont Curry'}},
    {line:'800ml / 1.7pt water', reading:{amt:'800 ml', food:'water'}},
    {line:'4 cups cooked rice (hot)', reading:{amt:'4 cups', food:'cooked rice'}},
    {line:'4 Chicken Katsu Cutlets (cut into 2.5cm / 1” wide strips, note 3)', reading:{amt:'4', food:'Chicken Katsu Cutlets', component:true}},
    {line:'fukujinzuke, to serve', reading:null},
  ],
  steps:[
    {t:'Add oil to a pot and heat over medium high heat.'},
    {t:'Add onion and sauté for a few minutes or until the onion becomes translucent and edges start getting slightly burnt.'},
    {t:'Add potatoes and carrots into the pot and stir for a couple of minutes or until the surface of the vegetables starts getting cooked.'},
    {t:'Add water and turn the heat up to bring it to a boil. Then reduce the heat to medium low and simmer for about 7 minutes or until the vegetables are nearly cooked through (note 4).'},
    {t:'Break the curry roux cake into small blocks along the lines and add them into the pot. Stir gently to blend the curry roux.'},
    {t:'Reduce the heat to low, place a lid on and cook for about 10 minutes or until the curry roux is completely dissolved. Stir occasionally as the curry tends to stick to the bottom of the pot.'},
    {t:'Check the consistency of the sauce. It should be like béchamel sauce. If it’s too thick, adjust with some water. If too thin, cook further without the lid.'},
    {t:'Turn the heat off.'},
    {section:'Serving'},
    {t:'Place a cup of hot cooked rice onto one side of a plate. Place the chicken cutlet pieces next to the rice, leaning them on the rice so that there will be a space to pour the curry.'},
    {t:'Pour curry next to the chicken cutlet, put fukujinzuke on the side and serve immediately.'},
  ],
  note:'Followed recipe exactly, just added one extra cube of curry. Add some spices to the chicken next time?',
};

const COQ = {
  slug:'coq', lineage:'coq-au-vin-31d7',
  title:'Braised Chicken in Red Wine (Coq au Vin)',
  img:null, serves:4, prep:30, cook:130, source:'YouTube', cooked:0, lang:'en',
  ingredients:[
    {line:'3 lb skin on chicken drumsticks', reading:{amt:'3 lb', food:'chicken drumsticks'}},
    {line:'1 bottle dry red wine (750mL)', reading:{amt:'750 ml', food:'red wine'}},
    {line:'150g thick cut bacon', reading:{amt:'150 g', food:'bacon'}},
    {line:'225g pearl onions or shallots', reading:null},
    {line:'225g carrots', reading:{amt:'225 g', food:'carrot'}},
    {line:'1 lb crimini mushrooms (baby bellas)', reading:{amt:'1 lb', food:'crimini mushrooms'}},
    {line:'olive oil', reading:null},
    {line:'salt', reading:null},
    {line:'25g tomato paste', reading:{amt:'25 g', food:'tomato paste'}},
    {line:'4 cloves minced garlic', reading:{amt:'4', food:'garlic clove'}},
    {line:'75g cognac', reading:{amt:'75 g', food:'cognac'}},
    {line:'40g ap flour', reading:{amt:'40 g', food:'flour'}},
    {line:'300g full bodied chicken stock', reading:{amt:'300 g', food:'chicken stock'}},
    {line:'2 sprigs thyme', reading:{amt:'2', food:'thyme'}},
    {line:'2 bay leaves', reading:{amt:'2', food:'bay leaf'}},
    {line:'200g egg noodles', reading:{amt:'200 g', food:'egg noodles'}},
    {line:'100g cold butter', reading:{amt:'100 g', food:'butter'}},
  ],
  steps:[
    {t:'Add drumsticks into ziplock bag and add 750mL of wine. Marinate in the fridge for 20-30 min.'},
    {t:'Remove chicken from wine and lay drumsticks out on a parchment lined sheet tray. Save the wine for later.'},
    {t:'Dry chicken on all sides well.'},
    {t:'Bake at 450F/230C for 20-30 min.'},
    {t:'To prep the rest of the ingredients, slice the bacon into lardons (rectangles).'},
    {t:'Preheat dutch oven over medium heat. Add about 2T of olive oil followed by bacon. Stir and render bacon for 10-12 minutes, stirring often, until browned.'},
  ],
  note:'The recipe is best served hot.',
};

/* Shelf cards. `img:null` means the recipe wears a generated Cover. */
const SHELVES = [
  {key:'shelfMost', blurb:'blurbMost', cards:[
    {title:'Katsu Curry (Japanese Curry with Chicken Cutlet)', img:IMG+'katsu-curry-japanese-curry-with-chicken-.jpg', lineage:'katsu-curry-8f2a', mins:25, cooked:6},
    {title:'Beef Bourguignon', img:IMG+'beef-bourguignon.jpg', lineage:'beef-bourg-04c1', mins:420, cooked:5},
    {title:'Cheese Ramen', img:IMG+'cheese-ramen.jpg', lineage:'cheese-ramen-9b30', mins:10, cooked:4},
    {title:'Braised Chicken in Red Wine (Coq au Vin)', img:null, lineage:'coq-au-vin-31d7', mins:160, cooked:3},
  ]},
  {key:'shelfQuick', blurb:'blurbQuick', cards:[
    {title:'Cheese Ramen', img:IMG+'cheese-ramen.jpg', lineage:'cheese-ramen-9b30', mins:10, cooked:4},
    {title:'Camarones al Ajillo', img:IMG+'camarones-al-ajillo.jpg', lineage:'camarones-7e55', mins:15, cooked:2},
    {title:'Gochujang Pasta', img:null, lineage:'gochujang-pasta-a204', mins:null, cooked:0},
    {title:'Sukiyaki Udon', img:IMG+'sukiyaki-udon.jpg', lineage:'sukiyaki-udon-6f8b', mins:20, cooked:1},
  ]},
  {key:'shelfNever', blurb:'blurbNever', cards:[
    {title:'Bollo limpio', img:null, lineage:'bollo-limpio-c93e', mins:null, cooked:0},
    {title:'Dan Dan Noodles', img:IMG+'dan-dan-noodles.jpg', lineage:'dan-dan-2a70', mins:30, cooked:0},
    {title:'Purée de Pommes de Terre', img:IMG+'puree-de-pommes-de-terre.jpg', lineage:'puree-pdt-d5a9', mins:55, cooked:0},
    {title:'Îles Flottantes', img:IMG+'iles-flottantes.jpg', lineage:'iles-flottantes-11bc', mins:40, cooked:0},
  ]},
  {key:'shelfRecent', blurb:'blurbRecent', cards:[
    {title:'Katsu Curry (Japanese Curry with Chicken Cutlet)', img:IMG+'katsu-curry-japanese-curry-with-chicken-.jpg', lineage:'katsu-curry-8f2a', mins:25, cooked:6},
    {title:'Miso Salmon', img:IMG+'miso-salmon-recipe-by-tasty.jpg', lineage:'miso-salmon-77aa', mins:30, cooked:2},
    {title:'Braised Chicken in Red Wine (Coq au Vin)', img:null, lineage:'coq-au-vin-31d7', mins:160, cooked:3},
    {title:'Bollo limpio', img:null, lineage:'bollo-limpio-c93e', mins:null, cooked:0},
  ]},
];

/* The three cooking states ADR 0011 names. */
const COOK = {
  amounts: {recipe:KATSU, step:3, n:11, amounts:[{amt:'800 ml', food:'water'}], timer:7},
  empty:   {recipe:KATSU, step:7, n:11, amounts:[], timer:null},
  plate:   {recipe:KATSU, step:9, n:11, section:'Serving', timer:null,
            amounts:[{amt:'4 cups', food:'cooked rice'}, {amt:'4', food:'Chicken Katsu Cutlets', component:true}]},
};

/* Helpers, defined here because data.js loads before the direction files. */
window.esc = s => String(s).replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
window.hash = s => { let h = 2166136261; for (let i=0;i<s.length;i++){ h ^= s.charCodeAt(i); h = Math.imul(h, 16777619);} return (h>>>0); };

/* The whole library, for the Recipes screen — where search lives (ADR 0011). */
const LIBRARY = [].concat(...SHELVES.map(s=>s.cards))
  .filter((c,i,a)=>a.findIndex(x=>x.lineage===c.lineage)===i)
  .concat([
    {title:'Îles Flottantes', img:IMG+'iles-flottantes.jpg', lineage:'iles-flottantes-11bc', mins:40, cooked:0, tag:'FR'},
    {title:'Camarones al Ajillo', img:IMG+'camarones-al-ajillo.jpg', lineage:'camarones-7e55', mins:15, cooked:2, tag:'ES'},
  ])
  .filter((c,i,a)=>a.findIndex(x=>x.lineage===c.lineage)===i);

/* Extra facts the recipe page shows, so the identity is judged against a full page. */
KATSU.attempts = 6;
KATSU.lastCooked = '12 Aug';
KATSU.branches = 2;
KATSU.tag = null;
COQ.attempts = 3;
COQ.lastCooked = '2 Jul';
