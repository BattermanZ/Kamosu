<!--
	The recipe screen, and — when a second Branch of the same Lineage is on this
	instance — the Divergence.

	ADR 0014, which refuses the obvious thing: holding two Branches shows TWO
	WHOLE RECIPES WITH A SWITCH BETWEEN THEM, never a difference. There is no
	comparison screen here, no side-by-side and no object called "the
	difference". You stand inside one recipe or the other — whole, in order,
	cookable — and the threshold at the top moves you between them.

	A line the other Branch has and yours has not is a GHOST: struck through, in
	the position it holds over there, saying whose it is. A removal seen from
	their side and an addition seen from yours are the same object, so one
	mechanism serves both and the page reads the same whichever recipe you are in.

	Which line is which is read against the Branch Point by the `divergence`
	Operation (ADR 0019). Nothing here pairs anything, and no line carries an id.

	THE PAGE'S OWN LAYOUT was chosen by Aurélien on 29 August 2026 against three
	full mockups drawn on real recipes, and the reasoning is on #81 rather than
	repeated here. What it settled:

	  · the hero is the photograph at 320px with the title standing ON it, over
	    an indigo wash — near-solid at the hem, so the title is readable over a
	    photograph whose colours nobody chose;
	  · with no photograph the Cover leads and carries the title the same way,
	    bare, because its dye was chosen and needs no wash (#46). Only the 10.5px
	    Source line cannot clear the contrast bar there, so on a Cover it is set
	    on paper beneath the hero instead;
	  · the meta is one full-bleed ruled strip of three cells;
	  · an Ingredient Line is a hairline-ruled row led by a small indigo square,
	    its Reading subordinate beneath it;
	  · a Step is a number in a narrow column, in indigo, and the step at body
	    size;
	  · a Section, in either list, is the quiet uppercase heading over a rule.

	The Reading is also CORRECTED here, in place, on the line it belongs to (#32
	item 193) — see `Correcting.svelte`, which owns why that makes no Version.

	No mark distinguishes a line Kamosu read from one it did not (ADR 0002): the
	Reading is simply there or it is not. A badge that fires sometimes teaches
	people it fires always.

	THE SUBORDINATE LINE IS ONE SLOT (#49, ADR 0016). Scaling and conversion are
	one act and share it with the Reading echo, so a row never carries two small
	lines under its written one. What the slot holds, in order: the converted
	amount where this reader needs one — `about 250 g` under `2 cups flour` for
	a metric cook — and otherwise the echo of what Kamosu read. A reader already
	in her own measures gets the echo, because the conversion has nothing to say
	and a blank slot would tell her less than the echo does. Both are quiet, both
	are visibly Kamosu's rather than the cook's, and neither is ever a badge.

	SINCE #71 THE ECHO IS SILENT WHERE IT WOULD ONLY REPEAT THE LINE, which is
	the rule the conversion already obeyed. The echo was chosen while a Reading
	existed only because somebody had typed one — it was the only way to see
	what Kamosu held. Now Kamosu reads every line it can, so an unfiltered echo
	would set `Za’tar` under `Za’tar` on most rows, and would appear on exactly
	the lines Kamosu managed to read: the badge ADR 0002 refuses, arrived at
	sideways. The echo that survives is the one that earns its place — a Reading
	somebody CORRECTED, saying something the written line does not.

	A Step's slot holds its conversions — the oven in the other system, on the
	conventional ladder, and each amount it writes — drawn straight after what
	each converts, never written into the sentence (#150).

	A COMPONENT UNFOLDS IN PLACE, ITS STEPS AT THE FOOT (#50, ADR 0008). An
	Ingredient whose Reading names a Lineage rather than a Food is a Component —
	the dough inside a pizza — and it is an ordinary Ingredient Line in every
	respect but two: the square in front of it is matcha rather than indigo, and
	it opens.

	What it opens into was chosen by Aurélien on 3 September 2026 against two
	treatments drawn on real recipes, recorded on #50. He was offered A · the
	nest, which put the inner recipe's Steps inside the row with its
	Ingredients, and chose B · the annexe:

	  · its INGREDIENT LINES unfold here, indented under the row behind a matcha
	    rule, already scaled by how much of that recipe this line asks for — so
	    the list stays a list you can shop from;
	  · its STEPS are set at the FOOT of the page, after this recipe's Method,
	    under a heading of their own. Never spliced into the method: composition
	    says WHAT and never WHEN, and Kamosu does not know the dough is made the
	    day before.

	What decided it was the library rather than taste. Every dough in the real
	86-recipe export runs to twelve or fifteen Steps, so the nest put fifteen
	steps between `Dough for 2 pizzas` and `250 g mozzarella` in the case the
	whole feature is written about.

	NONE OF THE ARITHMETIC OR THE WORDING IS HERE. How much of the inner recipe
	is wanted, its scaled amounts, and the one line beneath a Component's
	written line all arrive worded from the Core — which is why the Share Link
	page and an agent at the MCP door say exactly what this screen says.
-->
<script lang="ts" module>
	/**
	 * The Branch a Copy just landed on, so the page it opens can say so. It
	 * lives on the module rather than on the component because the Copy
	 * navigates: the component that knew is destroyed on the way (#83).
	 *
	 * It carries `named` for the same reason it carries the Branch (#87). A
	 * Copy is the save where EVERY pointer has to be attached afresh — the new
	 * Branch's first Version has no Readings to carry any of them forward — so
	 * it is the save where a failure to attach one matters most, and it was the
	 * one save that could not say so: the page that knew was already gone.
	 *
	 * It carries the save's Language offer for exactly the same reason (#106).
	 * The Core answers one on a copied save like any other, and writing French
	 * onto a recipe somebody else's Kitchen wrote is *where an offer is most
	 * likely* — so a Copy dropping it on the floor would leave the one case
	 * that needed it most as the one case that never got it.
	 */
	let copiedInto = $state<
		{ branchId: string; named: boolean; languageOffer: string | null; varied?: string } | undefined
	>(undefined);
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { cookingDay } from '$lib/cooking-day';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { StillRunning, waitForJob } from '$lib/api/job';
	import type {
		DivergenceOutput,
		GetRecipeOutput,
		GetThreadOutput,
		MakeSheetOutput,
	} from '$lib/api/catalogue';
	import Hero from './Hero.svelte';
	import { ratingLabel } from '$lib/rating';
	import VersionStrip from './VersionStrip.svelte';
	import { branchPlainName } from '$lib/cookbook';
	import MarkedRow from './MarkedRow.svelte';
	import StepWords from '$lib/StepWords.svelte';
	import Correcting from './Correcting.svelte';
	import Writing from './Writing.svelte';
	import Promotion from './Promotion.svelte';
	import Tags from './Tags.svelte';
	import RelatedRecipes from './RelatedRecipes.svelte';
	import Language from './Language.svelte';
	import LanguageSheet from './LanguageSheet.svelte';
	import RenameSheet from './RenameSheet.svelte';
	import LanguageOffer from './LanguageOffer.svelte';
	import { asBranchLanguage, type WrittenLanguage } from '$lib/language';
	import { tappableLink } from '$lib/source';
	import Confirm from '$lib/Confirm.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import AttemptPhoto from '$lib/offline/AttemptPhoto.svelte';
	import { onlyOnThisPhone } from '$lib/offline/outbox';
	import PhotoToRecipe, { type Offered } from '$lib/PhotoToRecipe.svelte';
	import StepPhoto from '$lib/StepPhoto.svelte';
	import { Online, refreshed, thisDevice, type Device } from '$lib/offline/device.svelte';
	import { useSheets } from '$lib/api/sheet';
	import HowMuch from '$lib/HowMuch.svelte';
	import { said, same, toSearch, type Wanted } from '$lib/how-much';
	import { useLibrary } from '$lib/offline/library.svelte';
	import { keptAt } from '$lib/offline/reads';
	import { figureOf, timeText, unitWord } from '$lib/duration';
	import { takePaste, type PastedDraft } from '$lib/pasted.svelte';
	import { standing } from '$lib/offline/standing.svelte';
	import {
		prose,
		draftVersion,
		reading,
		fieldText,
		nutritionText,
		rowKey,
		type Side,
		type Taken,
	} from './divergence';

	interface Props {
		branchId: string;
		/**
		 * Whether this is the app installed on an iPhone or iPad, where a Sheet
		 * goes to the share sheet rather than a tab (#149). A test hands in its own.
		 */
		device?: Pick<Device, 'installed' | 'apple'>;
	}

	let { branchId, device = thisDevice() }: Props = $props();

	const kamosu = useKamosu();
	const sheets = useSheets();

	let recipe = $state<GetRecipeOutput | undefined>(undefined);
	let divergence = $state<DivergenceOutput | undefined>(undefined);
	/**
	 * Every version of this recipe the reader may see, for the switch (#131):
	 * the Branches of the Lineage that share a history with this one, which is
	 * every Copy and variation of it and never a Translation (see below).
	 */
	let versions = $state<GetThreadOutput['branches']>([]);
	/** Whether the page is the reader's own recipe, the one marks compare against. */
	let onYours = $state(false);
	let failed = $state(false);
	/**
	 * The cookings of this dish, which the Thread already answers. Kept because
	 * one of them may hold an As Cooked nobody has decided about yet (#58) —
	 * the recipe screen is where Promotion is offered.
	 */
	let attempts = $state<GetThreadOutput['attempts']>([]);

	/**
	 * Your own pictures of this dish, from every time you cooked it (#110,
	 * option B) — newest cooking first, as the diary has them. Read from
	 * `list_attempts`, which the Core scopes to the caller, and never from the
	 * Thread's `attempts`, which carries the household's: a picture somebody
	 * else took is theirs to promote, not yours. One still only on this phone
	 * waits until it has been sent.
	 */
	let myPictures = $state<Offered[]>([]);
	let promotingPhoto = $state(false);
	/**
	 * Every Branch of this Lineage the reader can reach, which the Thread
	 * already answers and which this screen used to read for one fact and
	 * throw away. It is what tells a recipe it exists in another Language
	 * (#106, ADR 0006): a Translation is an ordinary Branch, so there is
	 * nothing else to ask — no Operation lists *this recipe's translations*,
	 * because there is no such object to list.
	 */
	let lineageBranches = $state<GetThreadOutput['branches']>([]);
	/**
	 * Every Version on every Branch of this Lineage, which the Thread answers
	 * anyway. The button into it says how many are this Branch's own and when
	 * the last was saved (#133), so a cook can tell before tapping whether
	 * anything is behind it.
	 */
	let lineageVersions = $state<GetThreadOutput['versions']>([]);
	/**
	 * The line under that button: the Versions on this Branch's own chain,
	 * counted and dated. A Variation's chain includes the Versions it started
	 * from, since those are its past too, and never another Branch's later
	 * ones. Filtered by the id on screen rather than reset on leaving, so the
	 * last recipe's count can never label this one's button.
	 */
	const threadLine = $derived.by(() => {
		const own = lineageVersions.filter((each) => each.branch_id === branchId);
		if (own.length === 0) return null;
		const last = own.reduce(
			(latest, each) => (each.created_at > latest ? each.created_at : latest),
			own[0]!.created_at,
		);
		const when = new Date(last).toLocaleDateString();
		if (own.length === 1) return m.recipe_thread_saved_once({ when });
		if (own.length === 2) return m.recipe_thread_saved_twice({ when });
		return m.recipe_thread_saved_times({ count: own.length, when });
	});
	/** Bumped after a Promotion, to read the recipe back with its new Version. */
	let reread = $state(0);

	/**
	 * Which side of the Divergence the page draws. `mine` is always the
	 * reader's own recipe (#131), so on anybody else's version the page is
	 * `theirs`: the rows are the same rows, read from the other side, and the
	 * page is still the recipe in the URL.
	 */
	let side = $state<Side>('mine');
	/** Whether the divergence is marked at all. Off is simply the recipe. */
	let marks = $state(true);
	let open = $state(new Set<string>());
	let taken = $state(new Map<string, Taken>());
	let saving = $state(false);
	let changeNote = $state('');
	let saved = $state<'no' | 'yes' | 'failed'>('no');

	/** Deleting this recipe (#120): the confirmation is open. */
	let confirmingDelete = $state(false);
	/** …and the act itself is running, so the sheet cannot be fired twice. */
	let deleting = $state(false);
	/** A refusal that came back, shown in the words it came in. */
	let deleteFailed = $state<string | undefined>(undefined);
	/**
	 * Whether a Share Link is live, asked only when the confirmation opens.
	 *
	 * The confirmation mentions the link **only when there is one** — a warning
	 * about a link nobody minted is noise, and noise in a sheet like this is
	 * how the sentence that matters stops being read. The recipe page does not
	 * otherwise need to know, so this is one call at the moment it becomes a
	 * fact worth having rather than a field on every read.
	 *
	 * Three states and not two, because the ask can fail. Neither silence nor
	 * an invented warning is honest then: one hides a link that really is
	 * about to stop working, the other frightens somebody about a link they
	 * never minted. `unknown` says which of the two it is and lets the reader
	 * decide, which is the only thing a screen that does not know can do.
	 */
	let shareIsLive = $state<'no' | 'yes' | 'unknown'>('no');
	/**
	 * Whether this recipe is on the reader's own Shopping List (#73).
	 *
	 * Read here rather than folded into `get_recipe`, for the reason
	 * `note_recipe_opened` is its own Operation: a Shopping List is one
	 * **Person's**, and a recipe is a Kitchen's, so a recipe reading its
	 * reader's private list on the side would tie the two together. Nothing
	 * waits on it; if it never answers, the button simply offers to add, and
	 * adding twice makes no second entry.
	 */
	let onTheList = $state(false);

	// ---- how much, for the errands (#109) ---------------------------------
	//
	// A VIEW THAT CARRIES FORWARD, Aurélien's choice of 23 September 2026. The
	// amounts are asked of the Core at the Yield named here, and nothing is
	// stored for having named it: Add to shopping list puts the recipe on the
	// list at it, Cook this opens the cooking's own question already set to it,
	// and a Sheet prints it. A recipe already on the list opens at the list's
	// amount, and changing it here changes the list. Leave with nothing added
	// and it is forgotten.

	/**
	 * How much this page is read at, where somebody named it — the reader, or
	 * the Shopping List the recipe is on. Undefined is the page as the Core
	 * sends it unasked: as written, or at the Yield an open cooking is at.
	 */
	let named = $state<Wanted | undefined>(undefined);
	/** The picker is open under the ingredients' heading. */
	let choosingHowMuch = $state(false);
	/** The last change of how much could not be read — no network, usually. */
	let howMuchFailed = $state(false);
	let shopping = $state(false);
	/** Where asking for a Sheet has got to (#75). */
	let printing = $state<'idle' | 'setting' | 'stillSetting' | 'ready' | 'failed'>('idle');
	/**
	 * The Sheet fetched and waiting for the tap that shares it (#149), in the
	 * installed app on an Apple device. Set only while `printing` is `ready`.
	 */
	let prepared: File | undefined;
	/** The Sheet being set goes to the share sheet rather than a tab (#149). */
	let sharing = $state(false);
	/** Being set, however long it takes: one Sheet at a time, and never a second tab. */
	const settingSheet = $derived(printing === 'setting' || printing === 'stillSetting');
	/**
	 * Whether the page is being written on rather than read (#83). It is the
	 * same page either way, which is the whole of the direction Aurélien
	 * chose: no second route, no compose screen.
	 */
	let writing = $state(false);
	/**
	 * A paste the + on Recipes read and made this recipe from (#175). The
	 * recipe holds only its title; the lines wait here, on the writing screen,
	 * for Save, the way a paste made on that screen does (#83).
	 */
	let pastedDraft = $state<PastedDraft | undefined>(undefined);
	$effect(() => {
		if (!recipe || !content || writing) return;
		const draft = takePaste(branchId);
		if (!draft) return;
		pastedDraft = draft;
		writing = true;
	});
	/**
	 * What the last save did, said HERE rather than on the writing screen:
	 * saving closes that screen, so anything it drew would be destroyed before
	 * it could be read (#83).
	 */
	let wrote = $state<
		| { collapsed: boolean; copied: boolean; named: boolean; language_offer: string | null }
		| undefined
	>(undefined);

	/**
	 * The Language sheet is open (#106) — where a recipe's Language is said by
	 * hand and where a Translation is started. Both mint a Version, which is
	 * why they are here among the acts and not up beside the Tags row.
	 */
	let sayingLanguage = $state(false);
	/**
	 * The sheet naming this version is open (#134): a Branch's name, which its
	 * chip in the strip and the line under the title both show.
	 */
	let renaming = $state(false);
	/**
	 * Whether to offer naming this version at all. Only a Co-author may, so
	 * only where the reader writes it. An unnamed version is offered a name only
	 * once the strip shows, since a recipe on its own has nothing to tell apart
	 * (the default stated with #134's options); a named one can always be
	 * renamed or cleared.
	 */
	const offersRename = $derived(
		recipe !== undefined && recipe.writes && (recipe.name !== null || versions.length > 1),
	);
	/** What that line says, and what the sheet it opens is called. */
	const renameLabel = $derived(
		recipe?.name ? m.recipe_version_rename() : m.recipe_version_name_it(),
	);
	/**
	 * Translating into this Language: the writing screen opens on this
	 * recipe's own words, to be replaced. Undefined is the ordinary edit.
	 */
	let translatingInto = $state<WrittenLanguage | undefined>(undefined);

	/**
	 * Every OTHER Branch of this Lineage, which is the whole of what the
	 * Language line reads. Computed here rather than inside `Language.svelte`
	 * so that the component takes plain facts and can be rendered in a test
	 * without a Thread.
	 */
	const otherBranches = $derived(
		lineageBranches
			.filter((each) => each.branch_id !== branchId)
			.map((each) => ({ branch_id: each.branch_id, language: each.language })),
	);
	/**
	 * The Branches that translate THIS one. A Translation names the Version it
	 * renders and `get_thread` answers that pointer per Branch, so this is a
	 * fact rather than an inference from Languages: two Branches in different
	 * Languages are not necessarily a Translation and its source — one may be
	 * a Divergence somebody relabelled.
	 */
	const translationsOfThis = $derived(
		lineageBranches.filter(
			(each) => each.branch_id !== branchId && each.translation?.source_branch_id === branchId,
		),
	);
	/**
	 * Whether Unknown is barred here, **in the Core's own terms**: this recipe
	 * translates something, or something translates it —
	 * `set_recipe_language` refuses exactly those two. Not "a sibling is in
	 * another Language", which would wrongly bar a recipe whose only sibling
	 * is a Divergence, and is not the rule being enforced.
	 *
	 * Where the two could still disagree — a Translation held by a Kitchen
	 * this reader does not cook in is absent from the Thread — this errs
	 * toward OFFERING, and the Core's refusal is then shown in its own words.
	 * Wrongly offering costs a sentence; wrongly barring hides a choice behind
	 * a reason that is not true.
	 */
	const inALanguageFamily = $derived(Boolean(recipe?.translation) || translationsOfThis.length > 0);
	/**
	 * The Languages not worth offering to translate into: this recipe's own,
	 * and those of the Translations this family already holds. A Divergence is
	 * deliberately NOT counted — somebody else's copy of these words happening
	 * to be in French is no reason to refuse to write a French translation.
	 */
	const languagesTaken = $derived([
		...(recipe ? [recipe.language] : []),
		...translationsOfThis.map((each) => each.language),
		...otherBranches
			.filter((each) => each.branch_id === recipe?.translation?.source_branch_id)
			.map((each) => each.language),
	]);

	/**
	 * The Language this save's text reads as, narrowed to one this build can
	 * act on. `language_offer` is declared as a plain string, so a newer
	 * server could answer a Language this build has no word for — not a thing
	 * to offer, since accepting it could not be carried out.
	 *
	 * Read from the Copy as well as the ordinary save: a copied save answers
	 * an offer like any other, and the page it lands on is the one that has to
	 * put it (#106).
	 */
	/**
	 * **Go to the Branch a write landed on, if it is not this one** — true when
	 * it navigated, so a caller can say what to do otherwise.
	 *
	 * Four different writes on this page can land somewhere else: a save that
	 * Copied, a Translation, and either half of #106 acting on a recipe another
	 * Kitchen writes. They are four Operations but one rule — staying here
	 * would leave the cook reading a recipe that is no longer the one they
	 * just changed — so it is written once.
	 */
	function follow(landedOn: string): boolean {
		if (landedOn === branchId) return false;
		void goto(`/recipes/${landedOn}`);
		return true;
	}

	/**
	 * **What a save that edited this recipe leaves behind** — the writing
	 * screen's, and a cooking's picture promoted onto it (#110), which is an
	 * ordinary edit making a Version and so ends the same two ways.
	 */
	function afterSave(landed: {
		branch_id: string;
		collapsed: boolean;
		copied: boolean;
		named: boolean;
		language_offer: string | null;
		varied?: string;
	}) {
		// A save moves the head Version, and both of these are keyed by a
		// line's index into the list that just changed underneath them.
		correcting = null;
		fixed = new Map();
		// A Copy put the Version on a NEW Branch. Staying here would leave
		// the cook reading the recipe they deliberately did not change, so
		// the page follows the one they now hold.
		if (landed.copied && landed.branch_id !== branchId) {
			// Said on the page it lands on rather than this one, which is
			// about to be left. `copied` is remembered across the
			// navigation because a Copy is the one save whose outcome is
			// not obvious from what is on screen afterwards: the recipe
			// looks the same, and only the Kitchen it now sits in changed.
			// The Language offer rides along for the same reason (#106).
			copiedInto = {
				branchId: landed.branch_id,
				named: landed.named,
				languageOffer: landed.language_offer,
				varied: landed.varied,
			};
			follow(landed.branch_id);
			return;
		}
		wrote = landed;
		reread += 1;
	}

	/**
	 * The dish, as a string, so the read below runs again when the page moves
	 * to another dish and not every time this one is read again — at another
	 * amount, say, or after a save.
	 */
	const lineage = $derived(recipe?.lineage_id);
	$effect(() => {
		if (!lineage) return;
		let current = true;
		kamosu
			.listAttempts({})
			.then((diary) => {
				if (!current) return;
				myPictures = diary.attempts
					.filter((attempt) => attempt.lineage_id === lineage)
					.flatMap((attempt) =>
						attempt.photographs
							.filter((photograph) => !onlyOnThisPhone(photograph))
							.map((photograph) => ({
								photograph,
								attempt: attempt.id,
								taken: cookingDay(attempt.created_at),
							})),
					);
			})
			.catch((error: unknown) => {
				// The row is an offer, not the recipe: a diary that cannot be
				// read leaves it out rather than failing the page.
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	const offeredLanguage = $derived.by(() => {
		const offered =
			copiedInto?.branchId === branchId
				? copiedInto.languageOffer
				: (wrote?.language_offer ?? null);
		return offered ? asBranchLanguage(offered) : null;
	});

	/**
	 * Print a Sheet (#75, ADR 0023): the recipe as it stands on this screen,
	 * set for paper by the server as a Job. The PDF opens in a tab of its own
	 * and printing is the browser's own Print — Kamosu has no dialog of its
	 * own, and no second place where scaling could disagree with this screen.
	 *
	 * The tab is opened at the tap and filled once the Job ends: a tab opened
	 * later, from a promise, is one a browser is entitled to block.
	 *
	 * A Sheet that outlasts the ordinary wait has not failed (#117). The screen
	 * says it is still being set and goes on waiting while the cook stays on
	 * this recipe, so the tab it promised is the tab the Sheet arrives in.
	 * Leaving — for another screen, or for another recipe on this one — stops
	 * the reading: a Sheet not ready by then is not waited for, and its empty
	 * tab closes. The Job itself carries on regardless (ADR 0032). One that was
	 * ready as the cook left still fills its tab, but never moves the page they
	 * have gone to.
	 *
	 * In the app installed on an iPhone or iPad no tab opens (#149): the Sheet
	 * is fetched here once the Job ends, and this same button then shares it
	 * (`shareSheet`; option A, Aurélien, 25 September 2026).
	 */
	async function printSheet() {
		const forVisit = visit;
		const leftBehind = () => closed || visit !== forVisit;
		const toShare = sharesSheets();
		sharing = toShare;
		printing = 'setting';
		const tab = toShare ? null : window.open('', '_blank');
		try {
			// At the amount on screen: the page is what a Sheet prints (ADR 0023).
			// Not while a Divergence is shown, where the amounts on screen are
			// `divergence`'s and the scaler is not offered (`pageScaledTo`).
			const asked = await kamosu.makeSheet(
				named === undefined || divergence
					? { branch_id: branchId }
					: { branch_id: branchId, wanted_yield: named },
			);
			const job = await waitForJob(kamosu, asked.job_id, { stopped: leftBehind }).catch(
				(error: unknown) => {
					if (!(error instanceof StillRunning)) throw error;
					printing = 'stillSetting';
					return waitForJob(kamosu, asked.job_id, {
						giveUpAfter: Infinity,
						stopped: leftBehind,
					});
				},
			);
			if (job.status !== 'completed') {
				// Left before the Sheet was ready: nothing is left to fill.
				tab?.close();
				return;
			}
			const at = (job.result as MakeSheetOutput).fetch_at;
			if (toShare) {
				const file = await sheets(at);
				// Left while it was fetched: nothing is kept for a page not shown.
				if (leftBehind()) return;
				prepared = file;
				printing = 'ready';
				return;
			}
			if (tab) tab.location.href = at;
			else if (!leftBehind()) window.location.assign(at);
			if (!leftBehind()) printing = 'idle';
		} catch {
			tab?.close();
			if (!leftBehind()) printing = 'failed';
		}
	}

	/**
	 * The installed app on an iPhone or iPad (#149). iOS keeps every address
	 * inside the manifest's scope in the app's own window, which has no Share,
	 * Save or Print, so a Sheet opened there can be looked at and nothing else.
	 * Asked with a stand-in PDF before the real one exists, because the choice
	 * of path is made at the first tap.
	 */
	function sharesSheets(): boolean {
		if (!device.installed || !device.apple || typeof navigator.share !== 'function') return false;
		// Not empty, so the answer is about a PDF rather than about nothing.
		const probe = new File(['%PDF-'], 'Sheet.pdf', { type: 'application/pdf' });
		return navigator.canShare?.({ files: [probe] }) === true;
	}

	/**
	 * Hand the prepared Sheet to the share sheet: Save to Files, Print, AirDrop.
	 * Called straight from the tap with nothing awaited first, since iOS only
	 * opens the share sheet for a tap it can still see (#149). Closing the
	 * share sheet is not a failure, and keeps the Sheet for another try; once
	 * it has gone somewhere, the next tap sets a fresh one.
	 */
	function shareSheet() {
		const file = prepared;
		if (!file) return;
		const forVisit = visit;
		const moved = () => closed || visit !== forVisit || prepared !== file;
		navigator.share({ files: [file] }).then(
			() => {
				if (moved()) return;
				prepared = undefined;
				printing = 'idle';
			},
			(error: unknown) => {
				if (error instanceof DOMException && error.name === 'AbortError') return;
				if (moved()) return;
				prepared = undefined;
				printing = 'failed';
			},
		);
	}

	/** Set once this screen closes, so a Sheet still being waited on stops being read. */
	let closed = false;
	$effect(() => () => {
		closed = true;
	});

	/**
	 * Read the recipe at another amount (#109). The Core scales every line it
	 * can and says what it scaled to; nothing here works an amount out, and
	 * nothing is stored. With no network the read fails and the page stays as
	 * it was, saying so — a scale is the Core's to work out.
	 *
	 * Where the recipe is on the Shopping List, the list follows: it is the
	 * errand this scaler is for.
	 */
	async function chooseHowMuch(wanted: Wanted, alsoTheList = true) {
		const asked = branchId;
		try {
			const read = await kamosu.getRecipe({ branch_id: asked, wanted_yield: wanted });
			if (asked !== branchId) return;
			recipe = read;
			named = wanted;
			// The read carries every correction made here since the last one,
			// scaled — the overlay's lines were worked out at the old amount.
			fixed = new Map();
			howMuchFailed = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			howMuchFailed = true;
			return;
		}
		if (alsoTheList && onTheList)
			void kamosu
				.setShoppingYield({ branch_id: asked, shopping_yield: wanted })
				.catch((error: unknown) => {
					if (!(error instanceof OperationError)) throw error;
				});
	}

	/**
	 * Open the confirmation, and ask whether a Share Link is live while it
	 * opens rather than before — the sheet is drawn immediately either way,
	 * and the line appears if the answer arrives saying there is one.
	 */
	async function askToDelete() {
		deleteFailed = undefined;
		shareIsLive = 'no';
		confirmingDelete = true;
		try {
			const link = await kamosu.getShareLink({ branch_id: branchId });
			shareIsLive = link.shared ? 'yes' : 'no';
		} catch {
			shareIsLive = 'unknown';
		}
	}

	/**
	 * Take this recipe off the shelf (#120). One Branch: the translation beside
	 * it, and any other Kitchen's copy, are untouched, and so is every cooking
	 * ever made from it.
	 *
	 * Back to the shelf afterwards, because there is nothing left to stand on.
	 * `replaceState` so the back button does not walk into a recipe that is
	 * gone — the page it would land on answers *no such Branch*, which is a
	 * true sentence and a baffling one to be shown for pressing back.
	 */
	async function deleteRecipe() {
		deleting = true;
		deleteFailed = undefined;
		try {
			await kamosu.deleteRecipe({ branch_id: branchId });
			await goto('/recipes', { replaceState: true });
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			deleteFailed = error.message;
		} finally {
			deleting = false;
		}
	}

	// ---- on the phone (#76) ---------------------------------------------

	const online = new Online();
	const library = useLibrary();
	/**
	 * When the phone kept the copy being shown, for a recipe the Person's
	 * Kitchens do not hold. Their own recipes are the library and always on
	 * the phone; anything else is only the copy from the day it was opened,
	 * and offline the page says so.
	 */
	const keptOn = $derived(standing.branchId === branchId ? standing.keptAt : undefined);
	const onlyKept = $derived(
		!online.current && library.onlyOpened(branchId) && keptOn !== undefined,
	);

	$effect(() => {
		const id = branchId;
		standing.branchId = id;
		void keptAt('get_recipe', { branch_id: id }).then((at) => {
			if (standing.branchId === id) standing.keptAt = at;
		});
		return () => {
			if (standing.branchId === id) {
				standing.branchId = undefined;
				standing.keptAt = undefined;
			}
		};
	});

	// The phone answered first and the server has since said something else:
	// read again, from the copy that is now level.
	let seenRefreshes: number | undefined;
	$effect(() => {
		const seen =
			(refreshed.get('get_recipe') ?? 0) +
			(refreshed.get('get_thread') ?? 0) +
			(refreshed.get('divergence') ?? 0);
		if (seenRefreshes !== undefined && seen > seenRefreshes) untrack(() => (reread += 1));
		seenRefreshes = seen;
	});

	/** Which Branch Home has been told was opened: once per visit, not per read. */
	let notedOpening: string | undefined;

	/**
	 * **What the last save did belongs to the recipe it was done on.** This
	 * screen is reused across `/recipes/A` → `/recipes/B` rather than remade,
	 * so without this the banner follows you to a recipe you never wrote on —
	 * which for the stale *saved* line is noise, and for #87's *a line could
	 * not be marked* is an alert about a defect that is not there.
	 */
	let saidFor: string | undefined;
	/**
	 * Counts the recipes this screen has shown, so work begun on one can tell
	 * it has been left — even for the same recipe opened again (#117).
	 */
	let visit = 0;
	$effect(() => {
		if (saidFor !== branchId) {
			saidFor = branchId;
			visit += 1;
			untrack(() => {
				wrote = undefined;
				named = undefined;
				// The comparison was the last version's. Walking along the strip
				// to your own must not carry its marks, or anything taken from
				// it, onto a page that has nothing to compare (#131).
				divergence = undefined;
				side = 'mine';
				marks = true;
				open = new Set();
				taken = new Map();
				changeNote = '';
				saved = 'no';
				choosingHowMuch = false;
				howMuchFailed = false;
				// A Sheet being set was the last recipe's; `printSheet` stops
				// waiting on it once it sees the recipe has changed (#117). A
				// Sheet ready to share was that recipe's too (#149).
				printing = 'idle';
				prepared = undefined;
			});
		}
	});

	$effect(() => {
		// Read again when a Promotion has just put a Version on this Branch, so
		// the recipe below the band becomes the recipe that was cooked (#58).
		void reread;
		let current = true;
		void (async () => {
			try {
				// At the amount already named here, so a Promotion or a Component
				// re-read does not quietly put the page back to the recipe as
				// written underneath somebody doing the errands (#109).
				const at = untrack(() => named);
				const read = await kamosu.getRecipe(
					at === undefined ? { branch_id: branchId } : { branch_id: branchId, wanted_yield: at },
				);
				if (!current) return;
				recipe = read;

				// Home's *recently opened* shelf, and the whole of what feeds it
				// (#64, ADR 0027). It is a separate Operation rather than
				// something `get_recipe` does on the side, because `get_recipe`
				// must stay a read: a read-only Access Key may read every recipe,
				// and making the reading itself a write would lock it out of the
				// library.
				//
				// Nothing waits on it and nothing is shown if it fails. It is one
				// timestamp that never travels and costs nothing if lost, so it
				// may never be the reason a cook cannot read a recipe — a
				// read-only Key is refused here every time, and reads on.
				if (notedOpening !== branchId) {
					notedOpening = branchId;
					void kamosu.noteRecipeOpened({ branch_id: branchId }).catch(() => {});
					// What it would put on a Shopping List, read once so the list
					// can be worked out on the phone with no network (#77). The
					// library fill reads it too; this covers a recipe written since.
					void kamosu.shoppingBasis({ branch_id: branchId }).catch(() => {});
				}

				void kamosu
					.getShoppingList({})
					.then((list) => {
						if (!current) return;
						const entry = list.chosen.find((each) => each.branch_id === branchId);
						onTheList = entry !== undefined;
						// On the list, so the page opens at the list's amount (#109) —
						// as written included, where an open cooking would otherwise
						// have scaled the page to its own.
						const onScreen = read.versions.at(-1)?.scaled_to ?? null;
						if (
							entry &&
							untrack(() => named) === undefined &&
							!same(entry.shopping_yield, onScreen)
						)
							void chooseHowMuch(entry.shopping_yield, false);
					})
					.catch(() => {});

				// Any Branch of the Lineage answers the same Thread, so this is how
				// the screen learns the other versions of the recipe exist at all.
				const thread = await kamosu.getThread({ branch_id: branchId });
				if (!current) return;
				attempts = thread.attempts;
				lineageBranches = thread.branches;
				lineageVersions = thread.versions;
				// **A Translation is not a Divergence, and cannot be paired with
				// one.** A Divergence is two Branches that parted from a shared
				// Version; a Translation's chain STARTS FRESH, which is exactly
				// what separates it from a Copy (ADR 0006, `start_translation`).
				// So a Translation and the recipe it renders share no Version at
				// all, and asking for a Divergence between them is answered —
				// correctly — with "their chains never converge".
				//
				// **Whether two Branches share a chain is answerable here**, and
				// is not worth a request that would be refused. The Thread
				// carries every Branch's every occurrence, so two Branches part
				// from a shared Version exactly when they have a Version id in
				// common — which two Translations of one recipe never do, and
				// a Branch and its Copy always do.
				const versionsOf = new Map<string, Set<string>>();
				for (const occurrence of thread.versions) {
					const seen = versionsOf.get(occurrence.branch_id) ?? new Set<string>();
					seen.add(occurrence.version_id);
					versionsOf.set(occurrence.branch_id, seen);
				}
				const onPage = versionsOf.get(branchId) ?? new Set<string>();
				versions = thread.branches.filter(
					(each) =>
						each.branch_id === branchId ||
						[...(versionsOf.get(each.branch_id) ?? [])].some((id) => onPage.has(id)),
				);
				// **Marks compare against your own** (#131, screen choice 1): the
				// unnamed version in your own Cookbook that you wrote. On it there
				// is nothing to mark; on any other version the rows are read with
				// yours as `mine`, and the page draws `theirs`. With none of your
				// own there is nothing to compare against at all.
				const yours = versions.find((each) => each.mine && !each.arrived && each.name === null);
				onYours = yours?.branch_id === branchId;
				if (!yours || onYours) return;

				// Read on its own, so a refusal costs the Divergence and never
				// the recipe. Whatever the Core declines to pair, the cook is
				// still holding a recipe they can read and cook from.
				divergence = await kamosu
					.divergence({ branch_id: yours.branch_id, other_branch_id: branchId })
					.catch((error: unknown) => {
						if (!(error instanceof OperationError)) throw error;
						return undefined;
					});
				if (current && divergence) side = 'theirs';
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			}
		})();
		return () => {
			current = false;
		};
	});

	// ---- where you are standing ------------------------------------------

	const here = $derived(side === 'mine' ? divergence?.mine : divergence?.theirs);
	/**
	 * Whose the other version is — `divergence.theirs`, named plainly for the
	 * marks' own sentences ("not Hélène’s"). Reading their recipe does not
	 * make you them, so a Ghost is described in terms of them from either side.
	 */
	const otherKitchen = $derived(divergence ? branchPlainName(divergence.theirs) : '');

	const content = $derived(here?.content ?? recipe?.versions.at(-1)?.content);
	const readings = $derived(here?.readings ?? recipe?.versions.at(-1)?.readings ?? []);
	/**
	 * The one subordinate line the Core worked out for THIS reader — scaling and
	 * conversion in a single slot (#49). Computed there rather than here on
	 * purpose: an agent at the MCP door gets the same answer this screen shows,
	 * and the arithmetic lives in one place beneath both Doors.
	 */
	const measured = $derived(
		here?.measured ?? recipe?.versions.at(-1)?.measured ?? { ingredients: [], steps: [] },
	);
	/**
	 * The Components of the recipe you are standing in, unfolded by the Core
	 * (#50, ADR 0008) — flat, depth first, each carrying the `path` of line
	 * indexes that reaches it. Both sides of a Divergence carry their own, so a
	 * dough unfolds whichever recipe you are standing in.
	 */
	const components = $derived(here?.components ?? recipe?.versions.at(-1)?.components ?? []);
	/**
	 * What this page's amounts are scaled to, in the Core's own word for it, or
	 * null where they are as written (#109). Not while a Divergence is shown:
	 * both sides come from `divergence`, which scales to an open cooking and
	 * takes no named amount, so the scaler is not offered there at all.
	 * `?? null`: a recipe the phone kept before #109 has no `scaled_to`.
	 */
	const pageScaledTo = $derived(divergence ? null : (recipe?.versions.at(-1)?.scaled_to ?? null));

	/** One Component, or nothing: the entry sitting at `index` of the list at `at`. */
	function componentAt(at: number[], index: number) {
		return components.find(
			(component) =>
				component.path.length === at.length + 1 &&
				at.every((step, depth) => component.path[depth] === step) &&
				component.path[at.length] === index,
		);
	}

	/**
	 * **Every Component's Steps, in the order the page meets them** — the foot
	 * of the page under treatment B. A Component with no Steps, one this
	 * instance does not hold, and one that stopped at a repeat all contribute
	 * nothing: there is no method to set.
	 */
	/**
	 * Which Components are open. **Closed by default** (ADR 0008): the row says
	 * which recipe it names and how much of it, and the recipe itself is a tap
	 * away. Keyed by path, so a Component inside a Component opens on its own.
	 *
	 * Crossing to the other Kitchen's recipe closes everything, for the reason
	 * `open` and `correcting` are cleared there: the two Branches have their own
	 * lists, so a path that means the dough here means another line over there.
	 */
	let unfoldedComponents = $state(new Set<string>());
	const pathKey = (path: number[]) => path.join('.');
	function toggleComponent(path: number[]) {
		const next = new Set(unfoldedComponents);
		const key = pathKey(path);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		unfoldedComponents = next;
	}
	const isOpen = (path: number[]) => unfoldedComponents.has(pathKey(path));

	/**
	 * **Every open Component's Steps, in the order the page meets them** — the
	 * foot of the page under treatment B. Folding a Component away takes its
	 * method with it, so the foot of the page holds exactly what the list above
	 * says is open. A Component with no Steps, one this instance does not hold,
	 * and one that stopped at a repeat all contribute nothing.
	 */
	const annexes = $derived(
		components.filter((component) => component.content?.steps.length && isOpen(component.path)),
	);

	// ---- correcting a Reading --------------------------------------------

	/** One slot of `readings`: what Kamosu understood of a line, or nothing. */
	type Slot = GetRecipeOutput['versions'][number]['readings'][number];
	/** One Component of the recipe being read, unfolded by the Core (ADR 0008). */
	type Component = GetRecipeOutput['versions'][number]['components'][number];
	/**
	 * A line corrected here: the Reading as it now stands, and the one
	 * subordinate line it now produces. They travel together because
	 * `set_reading` answers with both — the conversion is the Core's, and this
	 * screen only ever displays it.
	 */
	type Fixed = { reading: Slot; measured: string | null };

	/** Which Ingredient Line has the corrector open, by index into the list. */
	let correcting = $state<number | null>(null);
	/**
	 * Readings corrected here, laid over what was fetched. `set_reading` makes
	 * no Version, so there is nothing to refetch and nothing that would show up
	 * in the Thread — the line simply reads differently from now on.
	 */
	let fixed = $state(new Map<number, Fixed>());

	/**
	 * The Reading on one line, with anything corrected here laid over it.
	 *
	 * The overlay is consulted only while standing in your own recipe. It is
	 * keyed by line index, and the two Branches have their own lists — so on
	 * the other side index 2 is a different ingredient entirely, and reading
	 * through the overlay there would put your correction on their line.
	 */
	const readingAt = (index: number): Slot =>
		fixed.has(index) ? (fixed.get(index)?.reading ?? null) : (readings[index] ?? null);

	/**
	 * **The one line beneath an Ingredient Line**, and the whole of the rule:
	 * the converted amount where this reader needs one, the echo of what Kamosu
	 * read where she does not, and nothing at all where there is neither. Never
	 * both (#49, ADR 0016).
	 *
	 * **The echo is silent where it would only repeat the line above it**, which
	 * is the same rule the conversion already obeys. It was written when a
	 * Reading existed only because somebody had typed one, so an echo was the
	 * only way to see what Kamosu held; since #71 Kamosu reads every line it
	 * can, and an echo that parrots the line is two things it must not be — a
	 * repetition ADR 0016 says must be absent, and a mark on exactly the lines
	 * Kamosu managed to read, which is the badge ADR 0002 refuses. What is
	 * left is the echo that earns its place: the Reading somebody **corrected**,
	 * which says something the line does not.
	 *
	 * Like `readingAt`, the overlay is consulted only in your own recipe: the
	 * two Branches have their own lists, so index 2 on the other side is a
	 * different ingredient entirely.
	 */
	function beneathLine(index: number): string {
		if (index < 0) return '';
		const converted = fixed.has(index)
			? (fixed.get(index)?.measured ?? null)
			: (measured.ingredients[index] ?? null);
		if (converted) return converted;
		const echo = reading(readingAt(index));
		const written = content?.ingredients?.[index]?.text ?? '';
		return echo && saysMoreThan(echo, written) ? echo : '';
	}

	/**
	 * Whether an echo is worth showing under the line it was read from: it is,
	 * only where some word of it is not already up there. Compared as bare
	 * letters and digits, because the echo drops the punctuation and the
	 * articles the line keeps — `2 gousses ail` is entirely inside
	 * `2 gousses d’ail` and says nothing new, while a corrected `250 g farine
	 * de blé` under `200 g de farine` says two things.
	 */
	function saysMoreThan(echo: string, line: string): boolean {
		const bare = (text: string) => text.toLowerCase().replace(/[^\p{L}\p{N} ]/gu, '');
		const written = bare(line);
		return bare(echo)
			.split(/\s+/)
			.filter(Boolean)
			.some((word) => !written.includes(word));
	}

	/**
	 * A Reading may be corrected on any version you may see, somebody else's
	 * included: correcting what Kamosu read of a line mints no Version, and
	 * #131 put it with cooking and the shopping list, on the *may see* side
	 * of ADR 0041 (answer 7 on the issue). The Core checks the same thing.
	 *
	 * This is the whole of the rule. A marked row reaches the corrector too,
	 * through a second target inside its unfolded panel rather than through its
	 * own tap — see `MarkedRow`'s `onFixReading`.
	 */
	const correctable = true;

	function toggleCorrector(index: number) {
		correcting = correcting === index ? null : index;
	}

	function corrected(index: number, reading: Slot, measuredLine: string | null) {
		const next = new Map(fixed);
		// On a page read at a named amount the answer's line was worded at the
		// cooking's amount instead, so none is laid over until the page is read
		// again at its own — rather than a figure for the wrong amount (#109).
		next.set(index, { reading, measured: named === undefined ? measuredLine : null });
		fixed = next;
		correcting = null;

		// **A correction on a line that is, or was, a Component re-reads the
		// recipe** (#87). Everything else a correction changes is in `fixed`,
		// which is laid over the Readings — but a Component is not a Reading on
		// this page, it is the Core's unfolding of one, and every part of that
		// is worked out on the server from the Reading that just changed: which
		// recipe it names, how much of it is wanted, and the inner recipe's own
		// lines already scaled by that share (ADR 0008).
		//
		// So the pointer is not the only thing worth refetching for. Correcting
		// `500 g` to `250 g` on a dough leaves the pointer alone and halves the
		// share, and without this the row would read 250 g above an unfolding
		// still scaled at 500 — the line contradicting its own panel.
		//
		// An ordinary line's correction still refetches nothing: there is
		// nothing on screen for it that `fixed` does not already hold.
		const wasComponent = componentAt([], index) !== undefined;
		const isNowComponent = (reading?.lineage_id ?? null) !== null;
		if (wasComponent || isNowComponent) {
			reread += 1;
		} else if (named !== undefined) {
			// `set_reading` words the corrected line at the amount a cooking is
			// at, and knows nothing of this page's: read it again at this one.
			void chooseHowMuch(named, false);
		}
	}

	const unshared = $derived(
		divergence
			? [...divergence.ingredients, ...divergence.steps].filter((row) => row.state !== 'same')
					.length
			: 0,
	);

	/** Marked rows are only ever drawn when there IS a divergence and it is shown. */
	const marking = $derived(Boolean(divergence) && marks);

	/**
	 * What the other side has for a single value, when the two do not agree.
	 * The marking covers the whole recipe, not only the two lists (ADR 0019):
	 * a Title renamed or a Yield halved is a difference a cook needs to see.
	 */
	/**
	 * A marked value in words. The two times are minutes, so they read with
	 * their unit, `1 h 30`, as they do in the strip (#175); everything else is
	 * `fieldText`'s.
	 */
	function markText(name: keyof DivergenceOutput['fields'], value: unknown): string {
		const time = name === 'prep_time_minutes' || name === 'cook_time_minutes';
		return time && typeof value === 'number' ? timeText(value) : fieldText(value);
	}

	function markOf(name: keyof DivergenceOutput['fields']): string | null {
		if (!marking || !divergence) return null;
		const field = divergence.fields[name];
		if (field.same) return null;
		// A Note is a block of prose; "{kitchen} has {value}" reads as nonsense
		// against one, so it is introduced as the note it is.
		if (side === 'theirs') {
			// Their value is the page, so the mark says what YOURS has (#131):
			// naming them beside your value would say something false about
			// their recipe.
			const value = markText(name, field.mine);
			return name === 'note'
				? m.divergence_field_note_yours({ value })
				: m.divergence_field_yours({ value });
		}
		const value = markText(name, field.theirs);
		return name === 'note'
			? m.divergence_field_note({ kitchen: otherKitchen, value })
			: m.divergence_field_differs({ kitchen: otherKitchen, value });
	}

	// Putting the marking away rebuilds the list from a different set of rows —
	// so an open panel would reopen on whatever line happens to land at that
	// index. It closes everything first.
	function toggleMarks() {
		marks = !marks;
		open = new Set();
		correcting = null;
		unfoldedComponents = new Set();
	}

	function toggle(key: string) {
		const next = new Set(open);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		open = next;
	}

	function carry(key: string, what: Taken | 'undo') {
		const next = new Map(taken);
		if (what === 'undo') next.delete(key);
		else next.set(key, what);
		taken = next;
		saved = 'no';
	}

	function startSaving() {
		if (!divergence) return;
		changeNote = prose(divergence, taken);
		saving = true;
	}

	async function save() {
		if (!divergence) return;
		try {
			await kamosu.saveRecipeVersion(draftVersion(divergence, taken, changeNote));
			taken = new Map();
			saving = false;
			saved = 'yes';
			// A new Version can move a line's index, and both of these are keyed
			// by index. Neither survives the save.
			correcting = null;
			fixed = new Map();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			saved = 'failed';
		}
	}

	/** A Step's number, counted over the rows so a Ghost step takes none — it is
	 *  not a step of the recipe you are standing in. */
	function numbering() {
		let n = 0;
		return (ghost: boolean) => (ghost ? null : ++n);
	}
</script>

<!--
	The Source line, wherever it is set (#152, Aurélien's choice A of
	25 September 2026). Where the Source has a web link, the line itself is the
	link: the same place, size and colour, with an underline and a ↗ so it does
	not lean on colour alone, which on the wash it could not. It opens outside
	Kamosu, and an installed app on iOS shows it in a Safari view with Done.
	Without a link, or with one that is not a web address, it is the plain line
	it always was.
-->
{#snippet sourceLine(source: NonNullable<GetRecipeOutput['versions'][number]['content']['source']>)}
	{@const href = tappableLink(source.link)}
	{#if href}
		<a {href} target="_blank" rel="noopener noreferrer" class="underline underline-offset-2">
			{m.recipe_from_source({ source: source.text })}<span aria-hidden="true">&nbsp;↗</span><span
				class="sr-only">, {m.recipe_source_opens()}</span
			>
		</a>
	{:else}
		{m.recipe_from_source({ source: source.text })}
	{/if}
{/snippet}

<!--
	What stands on the hero on THIS screen: the Source, where a photograph is
	carrying it, and the title. On a Cover the Source cannot clear the
	contrast bar at 10.5px, so it is set on paper beneath the hero instead
	(#81) — which is why this snippet asks whether there is a photograph and
	the one on the writing screen does not.
-->
{#snippet titleOnHero()}
	{#if content?.main_photo && content.source}
		<p class="text-label text-on-accent uppercase">
			{@render sourceLine(content.source)}
		</p>
	{/if}
	<h1 class="mt-1 font-display text-title font-semibold text-on-accent">
		{content?.title}
	</h1>
{/snippet}

<!--
	A time in the strip, with its unit in the figure: `15 min`, `1 h 30`, `9 h`
	(#175, Aurélien's reading option 1). It printed the stored minutes bare
	before, over "min prep", so a 9-hour prove read 540. The label beneath is
	now only Prep or Cook, since the figure says its own unit.
-->
{#snippet timeFigure(minutes: number)}
	<b class="block font-display text-panel-figure font-semibold">
		{#each figureOf(minutes) as part, index (index)}
			<span class={index > 0 ? 'ms-1' : ''}
				>{part.value}{#if part.unit}<small class="ms-1 text-read font-normal"
						>{unitWord(part.unit)}</small
					>{/if}</span
			>
		{/each}
	</b>
{/snippet}

<!--
	Writing (#83). The page becomes writable in place, which is why this is a
	swap on the same route and not a screen of its own: Aurélien chose that
	over a separate compose screen, and a `/recipes/<id>/edit` route would be
	the rejected option wearing the chosen one's name. `Writing.svelte` holds
	why the shape is what it is.
-->
{#if writing && recipe && content}
	<Writing
		{branchId}
		lineageId={recipe.lineage_id}
		whose={recipe}
		content={pastedDraft
			? {
					...content,
					title: pastedDraft.title ?? content.title,
					note: pastedDraft.note ?? content.note,
					ingredients: pastedDraft.ingredients,
					steps: pastedDraft.steps.map((step) => ({ ...step, photo: null })),
				}
			: content}
		components={recipe.versions.at(-1)?.components ?? []}
		{translatingInto}
		onCancel={() => {
			writing = false;
			pastedDraft = undefined;
			translatingInto = undefined;
		}}
		onSaved={(landed) => {
			writing = false;
			pastedDraft = undefined;
			const wasTranslating = translatingInto !== undefined;
			translatingInto = undefined;
			// A Translation is a Branch of its own, and the cook has just
			// written it — so the page goes there, the way it follows a Copy.
			// Staying here would leave them reading the recipe they translated
			// with no sign the translation exists. It answers no offer: it
			// declared the Language it is written in.
			if (wasTranslating && landed.branch_id !== branchId) {
				// Leaving: nothing keyed by this recipe's lines may follow.
				correcting = null;
				fixed = new Map();
				follow(landed.branch_id);
				return;
			}
			afterSave(landed);
		}}
	/>
{:else}
	<div
		class="mx-auto max-w-2xl pb-tabbar"
		data-side={side}
		data-whose={recipe && !recipe.writes ? 'theirs' : 'mine'}
	>
		{#if failed}
			<p class="px-gutter py-6 text-body text-support" role="alert">{m.recipe_failed()}</p>
		{:else if !content}
			<p class="px-gutter py-6 text-body text-ink-2">{m.loading()}</p>
		{:else}
			{#if versions.length > 1}
				<VersionStrip
					{versions}
					current={branchId}
					compared={divergence ? { unshared, with: otherKitchen } : undefined}
					{onYours}
					{marks}
					{toggleMarks}
				/>
			{/if}

			<!--
			What leads the recipe: its Main Photo, or — for the roughly one
			recipe in three that has none — its Cover (#46). A Cover is not a
			placeholder for a missing picture; it is what a recipe without one
			wears. That it is one of these two things is #46's; that the title
			stands on whichever it is, is #81's.
		-->
			{#if recipe}
				<Hero
					lineageId={recipe.lineage_id}
					title={content.title}
					photo={content.main_photo}
					over={titleOnHero}
				/>
			{/if}

			<!--
			On a Cover the hero carries the title alone: kinari over the pasta
			shape measures 3.9:1, which the title clears at large-text's 3:1 and
			10.5px text does not (the title is 25px since #88, still large text
			at weight 600, so this is unchanged). So the Source is set here instead, on paper.
		-->
			<!--
			A named version's name, under its title (#131, screen choice 3): a
			variation, or a Branch named when two Cookbooks joined. On paper
			rather than on the hero, for the contrast reason above.
		-->
			{#if recipe?.name}
				<p class="px-gutter pt-3 font-display text-list-title font-semibold text-ink">
					{recipe.name}
				</p>
			{/if}
			{#if content.source && !content.main_photo}
				<p class="px-gutter pt-3 text-label text-ink-2 uppercase">
					{@render sourceLine(content.source)}
				</p>
			{/if}
			{#if markOf('source')}
				<p class="px-gutter pt-2 text-read text-accent">{markOf('source')}</p>
			{/if}
			{#if markOf('title')}
				<p class="px-gutter pt-2 text-read text-accent">{markOf('title')}</p>
			{/if}

			<!--
			What this recipe says about its Language, and nothing at all on the
			ordinary recipe that has nothing to say (#106, ADR 0006). Aurélien's
			choice of 22 September 2026; `Language.svelte` holds the reasoning
			and the rule that an empty state here would undo it.

			NOT DRAWN WHEN YOU HAVE CROSSED TO THE OTHER BRANCH, for the reason
			the Tags row is not. `recipe.language` and `recipe.translation` are
			THIS Branch's, and a Divergence does not carry theirs — so standing
			in their recipe under a line describing yours would say something
			false about which recipe you are reading.
		-->
			{#if recipe}
				<Language
					language={recipe.language}
					translation={recipe.translation}
					others={otherBranches}
				/>
			{/if}

			<!-- The meta: one full-bleed strip, three cells, hairlines between. -->
			{#if content.prep_time_minutes !== null || content.cook_time_minutes !== null || content.yield}
				<div class="mt-4 flex border-y border-rule">
					{#if content.prep_time_minutes !== null}
						<div class="flex-1 px-2 py-3 text-center">
							{@render timeFigure(content.prep_time_minutes)}
							<span class="mt-1 block text-label text-ink-2 uppercase">{m.recipe_prep()}</span>
						</div>
					{/if}
					{#if content.cook_time_minutes !== null}
						<div class="flex-1 border-l border-rule px-2 py-3 text-center first:border-l-0">
							{@render timeFigure(content.cook_time_minutes)}
							<span class="mt-1 block text-label text-ink-2 uppercase">{m.recipe_cook()}</span>
						</div>
					{/if}
					{#if content.yield}
						<div class="flex-1 border-l border-rule px-2 py-3 text-center first:border-l-0">
							<b class="block font-display text-panel-figure font-semibold">
								{content.yield.amount}
							</b>
							<span class="mt-1 block text-label text-ink-2 uppercase">{content.yield.noun}</span>
						</div>
					{/if}
				</div>
			{/if}
			{#each ['prep_time_minutes', 'cook_time_minutes', 'yield'] as const as name (name)}
				{#if markOf(name)}
					<p class="mt-1 px-gutter text-read text-accent">{markOf(name)}</p>
				{/if}
			{/each}

			<!--
			What this Kitchen says about the dish, above the dish itself (#104).
			The row is drawn on every recipe, tagged or not — Aurélien's choice,
			and `Tags.svelte` holds why.

			NOT DRAWN WHEN YOU HAVE CROSSED TO THE OTHER BRANCH. A Divergence
			does not mark tags (ADR 0019) and the `divergence` Operation
			therefore carries none, so there is nothing of theirs to show and
			showing yours beside their recipe would say something false about
			whose filing it is. Standing in your own Branch — which is every
			recipe that has no second Branch at all — the row is simply there.
		-->
			{#if recipe}
				<Tags branchId={recipe.branch_id} writes={recipe.writes} tags={recipe.tags} />
			{/if}
			<!--
			`nutrition` is deliberately absent from that loop. Its mark goes with
			the figure, at the foot of the Ingredients, because that is where
			Aurélien put the figure (#84) and a mark separated from the thing it
			marks is a sentence about nothing.
		-->

			{#if onlyKept && keptOn}
				<p class="mt-3 px-gutter text-read text-ink-2">
					{m.offline_kept_from({ date: keptOn.toLocaleDateString() })}
				</p>
			{/if}

			<!--
			One Ingredient Line, marked or not — the same row either way, because
			a list a cook shops from must keep one rhythm whether or not a second
			Branch happens to exist. `at` is the line's index into the written
			list, or -1 for a row with no line of its own to correct.
		-->
			{#snippet ingredientLine(text: string, at: number)}
				{@const readable = correctable && at >= 0}
				{@const component = componentAt([], at)}
				<li class="flex gap-3 border-b border-rule py-3">
					<!--
					Matcha rather than indigo where the line names a recipe (#50). The
					token was reserved for exactly this. It has a job: every line on
					this page is already tappable, to correct its Reading, so
					tappability alone cannot say there is a recipe behind this one.
				-->
					<span
						class="ingredient-marker shrink-0 {component ? 'bg-support-2' : 'bg-accent'}"
						aria-hidden="true"
					></span>
					<div class="min-w-0 flex-1">
						{#if readable}
							<button
								type="button"
								class="block w-full text-left"
								aria-expanded={correcting === at}
								onclick={() => toggleCorrector(at)}
							>
								{@render written(text, at, Boolean(component))}
							</button>
						{:else}
							{@render written(text, at, Boolean(component))}
						{/if}
						{#if component}
							{@render componentLine(component)}
						{/if}
						{#if readable && correcting === at}
							<Correcting
								{branchId}
								lineIndex={at}
								line={text}
								reading={readingAt(at)}
								componentTitle={component?.title}
								onDone={(next, converted) => corrected(at, next, converted)}
								onCancel={() => (correcting = null)}
							/>
						{/if}
						{#if component && isOpen(component.path)}
							{@render unfolded(component)}
						{/if}
					</div>
				</li>
			{/snippet}

			<!--
			The written Line, and beneath it the one subordinate line — smaller,
			quieter, and simply absent where there is nothing to say. Nothing here
			says whether it is a conversion or an echo, and nothing says whether
			Kamosu read the line at all (ADR 0002).

			A COMPONENT'S SLOT IS FILLED BY `componentLine` INSTEAD, outside this
			snippet — it is a target, and this one is rendered inside the button
			that opens the corrector. A button inside a button is invalid HTML and
			gives one row two overlapping targets, which on a phone is a coin toss.
		-->
			{#snippet written(text: string, at: number, isComponent: boolean)}
				<span class="block text-line">{text}</span>
				{#if !isComponent && pageScaledTo && at >= 0 && !measured.ingredients[at]}
					<!-- Asked for another amount, and this line did not move (#109). -->
					<span class="block text-read text-ink-2">{m.how_much_not_scaled()}</span>
				{:else if !isComponent && beneathLine(at)}
					<span class="block text-read text-ink-2">{beneathLine(at)}</span>
				{/if}
			{/snippet}

			<!--
			A COMPONENT'S OWN LINE: which recipe it names and how much of it, or the
			one sentence saying why there is no unfolding. It sits in the same slot
			every Ingredient Line has for its conversion (#49, ADR 0016) — a
			Component says something DIFFERENT there, not something extra beside it
			— and it is worded by the Core, so this screen, the Share Link page and
			an agent at the MCP door all say it alike.

			IT IS ALSO THE WAY IN, where there is something to open. The written
			line above keeps its own tap, which every line on this page has, so the
			recipe's NAME is what opens the recipe — the more obvious of the two
			anyway. A Component with nothing behind it is not a target: a missing
			recipe and a stopped repeat are sentences, not doors.
		-->
			{#snippet componentLine(component: Component)}
				{#if component.content}
					<button
						type="button"
						class="flex w-full items-start gap-1 text-left text-read text-support-2"
						aria-expanded={isOpen(component.path)}
						onclick={() => toggleComponent(component.path)}
					>
						<span class="min-w-0 flex-1">{component.said}</span>
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							class="mt-1 h-3 w-3 shrink-0"
							style={isOpen(component.path) ? 'transform: rotate(90deg)' : ''}
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="m9 5 7 7-7 7" />
						</svg>
					</button>
				{:else}
					<span class="block text-read text-support-2">{component.said}</span>
				{/if}
			{/snippet}

			<!--
			A Step's words, with what the Core added for this reader: its oven in
			the other system, on the conventional ladder, and each amount converted
			and scaled, each drawn straight after what it converts (#150, ADR 0016).
			Additions BESIDE the sentence, never written into it — a Step's truth is
			its text — and absent from most steps, which need none.
		-->
			{#snippet stepWithAdditions(text: string, at: number)}
				<p class="text-body">
					<StepWords
						{text}
						conversions={(at >= 0 ? measured.steps[at] : null) ?? []}
						readingClass="text-read text-ink-2"
					/>
				</p>
			{/snippet}

			<!--
			A Step's photograph, where it has one (#110): a small square at the end
			of the row, which opens the picture across the screen. Aurélien's
			choice (R2) over a full-width picture beneath the words, which pulled
			the Method apart; `StepPhoto` holds the rest.
		-->
			{#snippet stepPhoto(photo: string | null | undefined, n: number | null)}
				{#if photo && n !== null}
					<StepPhoto
						photograph={photo}
						number={n}
						shapeClass="h-[var(--photo-thumb)] w-[var(--photo-thumb)]"
					/>
				{/if}
			{/snippet}

			<!--
			A COMPONENT UNFOLDED: the inner recipe's own Ingredient Lines, indented
			under the row that names them behind a matcha rule, on the recessed
			ground — visibly another recipe's inside without being a card.

			Its Steps are NOT here. They are set at the foot of the page by
			`annexe`, which is the treatment Aurélien chose (#50), and it is what
			keeps this a list rather than a method with a shopping list around it.

			The amounts beneath each line are already scaled by how much of that
			recipe this line asks for and converted to this reader's measures —
			both worked out in the Core, by the same code that words every other
			Ingredient Line's slot. A Component inside a Component nests here too;
			`componentAt` finds it by its path.
		-->
			{#snippet unfolded(component: Component)}
				{#if component.content}
					<ul class="mt-3 border-l-2 border-support-2 bg-ground-2 py-1 pl-3">
						{#each component.content.ingredients as item, index (index)}
							{@const within = componentAt(component.path, index)}
							{#if item.kind === 'section'}
								<li class="border-b border-rule py-3 pb-1 last:border-b-0">
									<h4 class="font-display text-label text-ink-2 uppercase">{item.text}</h4>
								</li>
							{:else}
								<li class="flex gap-3 border-b border-rule py-3 last:border-b-0">
									<span
										class="ingredient-marker shrink-0 {within ? 'bg-support-2' : 'bg-accent'}"
										aria-hidden="true"
									></span>
									<div class="min-w-0 flex-1">
										<span class="block text-line">{item.text}</span>
										{#if within}
											<span class="block text-read text-support-2">{within.said}</span>
										{:else if component.measured?.ingredients[index]}
											<span class="block text-read text-ink-2">
												{component.measured.ingredients[index]}
											</span>
										{/if}
										{#if within}
											{@render unfolded(within)}
										{/if}
									</div>
								</li>
							{/if}
						{/each}
					</ul>
					<!--
					WHERE THE METHOD WENT. Under treatment B a Component is in two
					places, and the second one is a long way down the page — so the
					row says where, and the saying is a link that takes you there.
					Absent for a Component with no Steps: there is nothing at the foot
					to point at.
				-->
					{#if component.content.steps.length}
						<a
							href="#annexe-{pathKey(component.path)}"
							class="mt-2 block text-read text-support-2 underline underline-offset-2"
						>
							{m.recipe_component_method_below()}
						</a>
					{/if}
				{/if}
			{/snippet}

			<!--
			THE ANNEXE (#50): one Component's own Steps, at the foot of the page,
			under a heading in this page's own Section grammar but in matcha — so it
			reads as belonging to the Component rather than to this recipe's method.

			Never spliced into the Method above it. Composition says WHAT and never
			WHEN: Kamosu does not know the dough is made the day before, and where
			the timing matters the cook writes a Step saying so.
		-->
			{#snippet annexe(component: Component)}
				{#if component.content}
					{@const number = numbering()}
					<div id="annexe-{pathKey(component.path)}" class="mt-8 scroll-mt-12">
						<h2
							class="mx-gutter mb-1 font-display text-label font-semibold text-support-2 uppercase"
						>
							{component.title} · {m.recipe_component_method()}
						</h2>
						<p class="mx-gutter mb-2 text-read text-ink-2">{component.said}</p>
						<ol class="mx-gutter border-l-2 border-support-2 bg-ground-2 py-1 pl-3">
							{#each component.content.steps as item, index (index)}
								{#if item.kind === 'section'}
									<li class="border-b border-rule py-4 pb-1">
										<h3 class="font-display text-label text-ink-2 uppercase">{item.text}</h3>
									</li>
								{:else}
									<li class="flex gap-3 border-b border-rule py-3 last:border-b-0">
										<span class="w-6 shrink-0 font-display text-line font-semibold text-accent">
											{number(false)}
										</span>
										<p class="min-w-0 flex-1 text-body">{item.text}</p>
									</li>
								{/if}
							{/each}
						</ol>
					</div>
				{/if}
			{/snippet}

			<!-- Ingredients ------------------------------------------------------ -->
			<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
				{m.recipe_ingredients()}
			</h2>
			<!--
				HOW MUCH, for the errands (#109). Under the heading because it is a
				fact about every line beneath it, and a row rather than a control in
				the meta strip because a third of the library has no Yield to put a
				control on. In your own recipe only, and not while a Divergence is
				shown: see `pageScaledTo`.
			-->
			{#if !divergence}
				<div class="mx-gutter mb-2">
					<div class="flex items-center justify-between gap-3">
						<p
							class="min-w-0 text-read {pageScaledTo ? 'font-semibold text-accent' : 'text-ink-2'}"
						>
							{pageScaledTo
								? m.recipe_how_much({ amount: said(pageScaledTo) ?? '' })
								: content.yield
									? m.recipe_how_much({ amount: said(content.yield) ?? '' })
									: m.recipe_how_much_as_written()}
						</p>
						<button
							type="button"
							class="tap-out h-8 shrink-0 text-read text-accent underline"
							aria-expanded={choosingHowMuch}
							onclick={() => (choosingHowMuch = !choosingHowMuch)}
						>
							{choosingHowMuch ? m.recipe_how_much_done() : m.recipe_how_much_change()}
						</button>
					</div>
					{#if choosingHowMuch}
						<div class="mt-2 border-y border-rule py-3">
							<HowMuch
								written={content.yield}
								wanted={pageScaledTo}
								room="page"
								onchoose={(chosen) => void chooseHowMuch(chosen)}
							/>
						</div>
					{/if}
					{#if howMuchFailed}
						<p class="mt-2 text-read text-support" role="alert">{m.recipe_how_much_offline()}</p>
					{/if}
				</div>
			{/if}
			<ul class="px-gutter">
				{#if marking && divergence}
					{#each divergence.ingredients as row, index (rowKey('ingredients', index))}
						{@const key = rowKey('ingredients', index)}
						{@const own = side === 'mine' ? row.mine : row.theirs}
						{#if row.kind === 'section'}
							<li class="border-b border-rule py-4 pb-1">
								<h3 class="font-display text-label text-ink-2 uppercase">
									{(own ?? row.mine ?? row.theirs)?.text}
								</h3>
							</li>
						{:else if row.state === 'same'}
							{@render ingredientLine(own?.text ?? '', own?.index ?? -1)}
						{:else}
							<!--
							A CHANGED LINE THAT NAMES A RECIPE still says which one, in
							the same slot (#50). It does not unfold: a marked row is
							already carrying two Kitchens' words and a take-his offer,
							and a dough opened inside that is #55's question rather
							than this ticket's. The sentence is what stops the row
							going silent about what it is.
						-->
							{@const marked = own ? componentAt([], own.index) : undefined}
							<MarkedRow
								{row}
								{side}
								{otherKitchen}
								beneath={marked?.said ?? (own ? beneathLine(own.index) : '')}
								open={open.has(key)}
								taken={taken.get(key)}
								onToggle={() => toggle(key)}
								onCarry={(what) => carry(key, what)}
								onFixReading={correctable && own ? () => toggleCorrector(own.index) : undefined}
							/>
							{#if correctable && own && correcting === own.index}
								<li class="border-b border-rule pb-3 pl-3">
									<Correcting
										{branchId}
										lineIndex={own.index}
										line={own.text}
										reading={readingAt(own.index)}
										componentTitle={marked?.title}
										onDone={(next, converted) => corrected(own.index, next, converted)}
										onCancel={() => (correcting = null)}
									/>
								</li>
							{/if}
						{/if}
					{/each}
				{:else}
					{#each content.ingredients as item, index (index)}
						{#if item.kind === 'section'}
							<li class="border-b border-rule py-4 pb-1">
								<h3 class="font-display text-label text-ink-2 uppercase">{item.text}</h3>
							</li>
						{:else}
							{@render ingredientLine(item.text, index)}
						{/if}
					{/each}
				{/if}
			</ul>

			<!--
			THE NUTRITION FIGURE CLOSES THE LIST (#84). Aurélien chose this on
			21 September 2026 against four treatments drawn on both surfaces —
			a fourth cell in the meta strip, a line directly under the strip,
			this, and a place beside the Source. The list is where what goes
			into the dish is already the subject, and the strip keeps the three
			cells #81 gave it rather than being squeezed to four on a phone.

			It always says what it counts: 308 on its own says nothing, and a
			serving and 100 g do not convert into each other without a weight
			the recipe does not carry (CONTEXT.md, "Nutrition"). Most recipes
			carry no figure, and then there is nothing here at all — no dash, no
			placeholder and no zero, because zero would be a claim.
		-->
			{#if nutritionText(content.nutrition ?? null)}
				<p class="mt-3 px-gutter text-read text-ink-2">
					{nutritionText(content.nutrition ?? null)}
				</p>
			{/if}
			{#if markOf('nutrition')}
				<p class="mt-1 px-gutter text-read text-accent">{markOf('nutrition')}</p>
			{/if}

			<!-- Method ----------------------------------------------------------- -->
			<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
				{m.recipe_method()}
			</h2>
			<ol class="px-gutter">
				{#if marking && divergence}
					{@const number = numbering()}
					{#each divergence.steps as row, index (rowKey('steps', index))}
						{@const key = rowKey('steps', index)}
						{@const own = side === 'mine' ? row.mine : row.theirs}
						{@const n = number(!own)}
						{#if row.kind === 'section'}
							<li class="border-b border-rule py-4 pb-1">
								<h3 class="font-display text-label text-ink-2 uppercase">
									{(own ?? row.mine ?? row.theirs)?.text}
								</h3>
							</li>
						{:else if row.state === 'same'}
							<li class="flex gap-3 border-b border-rule py-3">
								<span class="w-6 shrink-0 font-display text-line font-semibold text-accent"
									>{n}</span
								>
								<div class="min-w-0 flex-1">
									{@render stepWithAdditions(own?.text ?? '', own?.index ?? -1)}
								</div>
								{@render stepPhoto(own ? (content.steps[own.index]?.photo ?? null) : null, n)}
							</li>
						{:else}
							<MarkedRow
								{row}
								{side}
								{otherKitchen}
								number={n}
								conversions={own ? (measured.steps[own.index] ?? []) : []}
								photo={own ? (content.steps[own.index]?.photo ?? null) : null}
								open={open.has(key)}
								taken={taken.get(key)}
								onToggle={() => toggle(key)}
								onCarry={(what) => carry(key, what)}
							/>
						{/if}
					{/each}
				{:else}
					{@const number = numbering()}
					{#each content.steps as item, index (index)}
						{#if item.kind === 'section'}
							<li class="border-b border-rule py-4 pb-1">
								<h3 class="font-display text-label text-ink-2 uppercase">{item.text}</h3>
							</li>
						{:else}
							{@const n = number(false)}
							<li class="flex gap-3 border-b border-rule py-3">
								<span class="w-6 shrink-0 font-display text-line font-semibold text-accent">
									{n}
								</span>
								<div class="min-w-0 flex-1">
									{@render stepWithAdditions(item.text, index)}
								</div>
								{@render stepPhoto(item.photo, n)}
							</li>
						{/if}
					{/each}
				{/if}
			</ol>

			<!-- The annexe (#50): every Component's Steps, in the order the page met them. -->
			{#each annexes as component (component.path.join('.'))}
				{@render annexe(component)}
			{/each}

			{#if content.note}
				<div
					class="mx-gutter mt-6 border-l-2 border-accent py-1 pl-4 text-body whitespace-pre-wrap"
				>
					{content.note}
				</div>
			{/if}
			{#if markOf('note')}
				<p class="mx-gutter mt-1 text-read text-accent">{markOf('note')}</p>
			{/if}

			<!--
			What this recipe goes with (#105, #52). Aurélien's choice of
			22 September 2026: the shelf's own cards, here between the Method and
			Cooked, and `RelatedRecipes.svelte` holds why.

			NOT DRAWN WHEN YOU HAVE CROSSED TO THE OTHER BRANCH, for the reason
			the Tags row is not. A Divergence does not mark Related Recipes
			(ADR 0019) and the `divergence` Operation carries none, so there is
			nothing of theirs to show, and showing yours beside their recipe
			would say something false about whose shelf the links are on.
		-->
			{#if recipe}
				<RelatedRecipes
					branchId={recipe.branch_id}
					writes={recipe.writes}
					related={recipe.related_recipes}
				/>
			{/if}

			<!--
			Cooked (#59). How this dish has actually gone: how many times, when
			last, and each Person's most recent verdict with their name.

			There is no average here and no way to build one, which is ADR 0015
			working rather than a rule anybody has to remember — a rating is a
			word, and only the newest one each Person gave ever arrives, so a
			verdict somebody has since superseded cannot drag down a recipe that
			was fixed months ago. A Person who cooked and said nothing simply
			does not appear: silence is not a score of zero.
		-->
			{#if recipe}
				<h2 class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase">
					{m.recipe_cooked()}
				</h2>
				<div class="px-gutter">
					{#if recipe.cooked.count === 0 || !recipe.cooked.last_cooked_at}
						<p class="text-read text-ink-2">{m.recipe_cooked_never()}</p>
					{:else}
						{@const when = new Date(recipe.cooked.last_cooked_at).toLocaleDateString()}
						<p class="text-read text-ink-2">
							{recipe.cooked.count === 1
								? m.recipe_cooked_once({ when })
								: m.recipe_cooked_times({ count: recipe.cooked.count, when })}
						</p>
						<ul>
							{#each recipe.cooked.ratings as verdict (verdict.person_id)}
								<li class="flex items-baseline justify-between gap-3 border-b border-rule py-2">
									<span class="text-line">{verdict.name}</span>
									<span class="text-read text-accent uppercase">{ratingLabel(verdict.rating)}</span>
								</li>
							{/each}
						</ul>
					{/if}
					<!--
						Your own pictures of the dish, and the way to put one on the
						recipe (#110, option B): here, among how the dish has actually
						gone, because that is what they are. Nothing at all when you
						have none, which is nearly always; and only in your own
						Branch, for the reason the Tags row is.
					-->
					{#if myPictures.length > 0}
						<div class="mt-3">
							<h3 class="mb-2 text-label text-ink-2 uppercase">{m.recipe_my_photos()}</h3>
							<ul class="flex flex-wrap gap-2">
								{#each myPictures as picture (picture.attempt + picture.photograph)}
									<li>
										<AttemptPhoto
											id={picture.photograph}
											alt={m.promote_taken({ date: picture.taken })}
										/>
									</li>
								{/each}
							</ul>
							<NeedsServer
								label={m.recipe_use_photo()}
								waiting={m.offline_waits_edit()}
								onclick={() => (promotingPhoto = true)}
								shapeClass="mt-2 min-h-12 w-full px-4 text-body"
								lookClass="border border-rule text-accent"
							/>
						</div>
					{/if}
				</div>
			{/if}

			{#if promotingPhoto && recipe}
				<PhotoToRecipe
					{branchId}
					pictures={myPictures}
					onPromoted={(landed) => {
						promotingPhoto = false;
						// Nothing to name: a promotion changes a photograph and
						// no Ingredient Line, so no Component is left behind.
						afterSave({ ...landed, named: true });
					}}
					onClose={() => (promotingPhoto = false)}
				/>
			{/if}

			{#if copiedInto?.branchId === branchId}
				<p class="mx-gutter mt-4 text-read text-accent" role="status">
					{copiedInto.varied
						? m.write_saved_varied({ name: copiedInto.varied })
						: m.write_saved_copied()}
				</p>
			{:else if wrote}
				<p class="mx-gutter mt-4 text-read text-accent" role="status">
					{m.write_saved()}{#if wrote.collapsed}&nbsp;{m.write_saved_collapsed()}{/if}
				</p>
			{/if}
			<!--
				The Version landed and a line naming another recipe did not take
				(#87). Said apart from the save rather than instead of it: the
				recipe IS saved, and a line that reads correctly but is not a
				Component is exactly the failure nobody would otherwise notice.
			-->
			{#if (wrote && !wrote.named) || (copiedInto?.branchId === branchId && !copiedInto.named)}
				<p class="mx-gutter mt-2 text-read text-support" role="alert">
					{m.write_components_failed()}
				</p>
			{/if}

			<!--
			The Language offer, put to the cook (#106, ADR 0006) — the half of
			that ADR the interface never kept. It lands HERE, under the save's
			own line, which is Aurélien's choice of 22 September 2026 against a
			band at the top of the page: the offer arrives at the moment of
			saving, so it goes where the save already speaks.

			It belongs to one save. `language_offer` rides out on the save's
			answer and nothing stores it, so there is nothing to come back to —
			declining ends it, and `wrote` being cleared when the page moves to
			another recipe ends it too.
		-->
			{#if recipe && offeredLanguage}
				<LanguageOffer
					branchId={recipe.branch_id}
					filed={recipe.language}
					offered={offeredLanguage}
					onSaid={(landed) => {
						if (!follow(landed.branch_id)) reread += 1;
					}}
				/>
			{/if}

			{#if saved === 'yes'}
				<p class="mx-gutter mt-4 text-read text-accent" role="status">{m.divergence_saved()}</p>
			{:else if saved === 'failed'}
				<p class="mx-gutter mt-4 text-read text-support" role="alert">
					{m.divergence_save_failed()}
				</p>
			{/if}

			<!--
			Into the cooking screen (#61, ADR 0011). It is a link rather than a
			button that starts something: opening the screen IS starting the
			Attempt, and one already In Progress is handed back rather than
			doubled — so there is nothing here to press twice by mistake.
		-->
			<!--
			Promotion (#58, ADR 0005). Above `Cook this` because it is a question
			about the recipe you are standing in, and drawn at all only where a
			cooking departed from these words and nobody has decided about it yet.
			A recipe nobody cooked differently carries nothing here — which is
			every recipe, nearly always.
		-->
			{#if recipe}
				<Promotion
					{branchId}
					whose={recipe}
					{attempts}
					versions={recipe.versions}
					promoted={() => (reread += 1)}
				/>
			{/if}

			<!--
			Writing (#83). Drawn with `NeedsServer` because editing is on the
			server's side of the line and is never queued: an offline edit
			queue is a merge, and Kamosu does not merge (ADR 0013, #76). The
			phrase it wears offline was written for this button before the
			button existed.
		-->
			<NeedsServer
				label={m.write_edit()}
				waiting={m.offline_waits_edit()}
				onclick={() => (writing = true)}
				shapeClass="mx-gutter mt-6 block w-[calc(100%-2*var(--spacing-gutter))] p-4 text-center font-display text-body"
				lookClass="border border-rule text-accent"
			/>

			<a
				href="/cook/{branchId}{toSearch(pageScaledTo)}"
				class="mx-gutter mt-2 block w-[calc(100%-2*var(--spacing-gutter))] bg-accent p-4 text-center font-display text-body text-on-accent"
			>
				{m.recipe_cook_this()}
			</a>
			<!--
			Into the Thread, under the word a cook already knows for it (#133,
			Aurélien's choice A of 24 September 2026). The screen keeps the
			Thread's name in the code and the docs; the button says History
			because "The thread" told nobody what was behind it. It never says
			"version": on this page that word already means one of the recipe's
			versions in the strip at the top.

			The line under it is built like the Cooked line: how many saves, and
			when the last was. On a recipe straight from an import it says one,
			which is the honest answer to "is anything back there?".
		-->
			<a
				href="/recipes/{branchId}/thread"
				class="mx-gutter mt-2 block border border-rule p-4 text-center font-display text-body text-accent"
			>
				{m.recipe_the_thread()}
				{#if threadLine}
					<span class="mt-1 block font-sans text-read text-ink-2">{threadLine}</span>
				{/if}
			</a>
			<!--
			Saying what Language this recipe is in, and translating it (#106).
			Here among the acts rather than up beside the Tags row, because
			both mint a Version and the Tags row is explicitly the place where
			nothing does (ADR 0035, ADR 0006).

			`NeedsServer` for the reason editing is: both Operations are on the
			server's side of the line and neither is ever queued. An offline
			queue for a Language would be two Kitchens disagreeing about what a
			recipe is written in, and Kamosu does not merge (ADR 0013, #76).

			Drawn only in your own Branch, as the Tags row and the Related
			strip are: it changes the recipe, and the recipe you are standing
			in across a Divergence is not yours to change.
		-->
			{#if recipe}
				<NeedsServer
					label={m.recipe_language_title()}
					waiting={m.offline_waits_edit()}
					onclick={() => (sayingLanguage = true)}
					shapeClass="mx-gutter mt-2 block w-[calc(100%-2*var(--spacing-gutter))] p-4 text-center font-display text-body"
					lookClass="border border-rule text-accent"
				/>
			{/if}
			<!--
			Naming this version (#134, Aurélien's choice B of 24 September 2026):
			a line among the acts, directly below Language, opening a small
			sheet. `RenameSheet.svelte` holds why it says "version" and what it
			promises about the History.

			`NeedsServer` because a Branch's name is on the server's side of the
			line and never queued, like every change to what a recipe is.
		-->
			{#if offersRename && recipe}
				<NeedsServer
					label={renameLabel}
					waiting={m.offline_waits_version_name()}
					onclick={() => (renaming = true)}
					shapeClass="mx-gutter mt-2 block w-[calc(100%-2*var(--spacing-gutter))] p-4 text-center font-display text-body"
					lookClass="border border-rule text-accent"
				/>
			{/if}
			<!--
			Into the share screen (#65, ADR 0026). A link rather than a switch
			here on purpose: turning sharing on is one deliberate act taken on a
			screen that says what it means, not a toggle brushed past on the way
			to cooking.
		-->
			<a
				href="/recipes/{branchId}/share"
				class="mx-gutter mt-2 block border border-rule p-4 text-center font-display text-body text-accent"
			>
				{m.share_title()}
			</a>
			<!--
			A Sheet (#75, ADR 0023): this recipe, as it stands here, on paper.
			It waits for the server, since the server is what sets it.
		-->
			{#if printing === 'ready'}
				<!--
				The Sheet is already on the phone, so sharing it waits for nothing
				(#149): the same button, no longer one that needs the server.
			-->
				<button
					type="button"
					onclick={shareSheet}
					class="mx-gutter mt-2 block w-[calc(100%-2*var(--spacing-gutter))] border border-rule p-4 text-center font-display text-body text-accent"
				>
					{m.recipe_share_sheet()}
				</button>
			{:else}
				<NeedsServer
					label={settingSheet ? m.recipe_print_setting() : m.recipe_print_sheet()}
					waiting={m.offline_waits_print()}
					onclick={printSheet}
					disabled={settingSheet}
					shapeClass="mx-gutter mt-2 block w-[calc(100%-2*var(--spacing-gutter))] p-4 text-center font-display text-body"
					lookClass="border border-rule text-accent"
				/>
			{/if}
			{#if printing === 'failed'}
				<p class="mx-gutter mt-2 text-read text-support" role="alert">{m.recipe_print_failed()}</p>
			{:else if printing === 'stillSetting'}
				<p class="mx-gutter mt-2 text-read text-ink-2" role="status">
					{sharing ? m.recipe_share_still_going() : m.recipe_print_still_going()}
				</p>
			{/if}
			<!--
			Onto the Shopping List (#73, ADR 0024). A button and not a link: it
			is one act that finishes here, and pressing it again takes the
			recipe back off. What it stores is the choosing — the Branch, at
			whatever Version it is on when the list is next read.
		-->
			<button
				type="button"
				disabled={shopping}
				class="mx-gutter mt-2 block w-[calc(100%-2*var(--spacing-gutter))] border border-rule p-4 text-center font-display text-body {onTheList
					? 'text-ink-2'
					: 'text-accent'}"
				onclick={async () => {
					shopping = true;
					try {
						if (onTheList) {
							await kamosu.removeFromShoppingList({ branch_id: branchId });
							onTheList = false;
						} else {
							// At the amount on screen (#109): what the errands scaler is for.
							await kamosu.addToShoppingList(
								pageScaledTo
									? { branch_id: branchId, shopping_yield: pageScaledTo }
									: { branch_id: branchId },
							);
							onTheList = true;
						}
					} catch (error: unknown) {
						if (!(error instanceof OperationError)) throw error;
					} finally {
						shopping = false;
					}
				}}
			>
				{onTheList ? m.shopping_on_your_list() : m.shopping_add_this()}
			</button>

			<!--
			Deleting this recipe (#120). Set apart from the stack above, below a
			rule and a gap, as small underlined text rather than a seventh
			full-width button — the choice of 22 September 2026, over putting it
			in the stack.

			The reasoning is about the thumb, not the look. Everything above is
			the same shape in the same column, and *Add to shopping list* is the
			one people tap most often without reading; a destructive row
			directly beneath it is a mis-tap waiting to happen. Something that
			is not button-shaped, past the end of the actions, cannot be reached
			by the habit that reaches for those.

			NOT in Writing, where the mockup drew it: that screen holds an
			unsaved draft the whole time it is open, and a screen that can both
			lose your typing and destroy the recipe is asking two very different
			questions with one set of buttons.

			`NeedsServer` because deleting is a write against the recipes, which
			are the server's side of the line and never queued (ADR 0013). The
			outbox carries your own history, never the recipes — and it queues
			from an allowlist, so this is already true rather than arranged.

			Only on a recipe you write (#131): deleting is a change, and anybody
			else's version is theirs to keep or delete, so offering it would
			only ever be offering a refusal.
		-->
			{#if recipe?.writes}
				<div class="mx-gutter mt-8 border-t border-rule pt-4 text-center">
					<NeedsServer
						label={m.recipe_delete()}
						waiting={m.offline_waits_delete()}
						onclick={askToDelete}
						shapeClass="inline-block px-3 py-2 text-read"
						lookClass="text-support underline underline-offset-4"
						idleClass="text-ink-2 opacity-55"
					/>
				</div>
			{/if}
		{/if}

		<!-- Carried across and not yet saved. It becomes real only when an ordinary
	     Version is saved — there is no other kind of save here. -->
		{#if taken.size > 0 && divergence}
			<div
				class="fixed inset-x-0 bottom-tabbar z-30 mx-auto max-w-2xl border-t border-on-accent/25 bg-accent px-gutter py-3 text-on-accent"
			>
				<p class="mb-2 text-read">
					{taken.size === 1
						? m.divergence_unsaved_one({ kitchen: otherKitchen })
						: m.divergence_unsaved({
								count: taken.size,
								kitchen: otherKitchen,
							})}
				</p>
				<div class="flex gap-2">
					<!-- Saving a Version is editing the recipe: it waits for the server,
				     and what was carried across stays carried until then (#76). -->
					<NeedsServer
						label={m.divergence_save()}
						waiting={m.offline_waits_save()}
						onclick={startSaving}
						shapeClass="flex-1 p-2 text-center text-read"
						lookClass="bg-on-accent text-accent"
						idleClass="border border-on-accent/40 text-on-accent opacity-55"
					/>
					<button
						type="button"
						onclick={() => (taken = new Map())}
						class="flex-1 border border-on-accent/40 p-2 text-center text-read"
					>
						{m.divergence_undo()}
					</button>
				</div>
			</div>
		{/if}
	</div>
{/if}

{#if saving && divergence}
	<div class="fixed inset-0 z-40 bg-accent/40"></div>
	<div
		class="fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[78vh] max-w-2xl overflow-y-auto bg-ground px-gutter pt-4 pb-safe"
		role="dialog"
		aria-modal="true"
		aria-label={m.divergence_save()}
	>
		<h3 class="font-display text-title font-semibold">{m.divergence_save()}</h3>
		<label class="mt-4 block text-label text-ink-2 uppercase" for="what-changed">
			{m.divergence_what_changed()}
		</label>
		<textarea
			id="what-changed"
			rows="3"
			bind:value={changeNote}
			class="mt-1 w-full rounded-sm border border-rule bg-card p-3 text-body"></textarea>
		<p class="mt-2 text-read text-ink-2">
			{m.divergence_save_hint({ kitchen: otherKitchen })}
		</p>
		<NeedsServer
			label={m.divergence_save()}
			waiting={m.offline_waits_save()}
			onclick={save}
			shapeClass="mt-4 block w-full p-4 text-center font-display text-body"
			lookClass="bg-accent text-on-accent"
		/>
		<button
			type="button"
			onclick={() => (saving = false)}
			class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-accent"
		>
			{m.divergence_cancel()}
		</button>
	</div>
{/if}

<!--
	The confirmation for deleting this recipe (#120), in the one sheet Kamosu
	asks every irreversible thing through (#103).

	It names three facts and no more. The recipe's name, so there is no doubt
	which copy you meant — the route is per-Branch, so a delete offered here is
	never ambiguous. That the cooking history stays, because that is the thing
	a person would most expect a delete to take and the reassurance is the
	whole point of saying it. And the Share Link, **only when one is live**.

	No big figure leads it. The sheet's own note says the number should be the
	thing you are deciding about, and here there is not one: a large `11` over
	a delete button reads as eleven things going, which is the exact opposite
	of what this sheet is promising.

	THE COUNT IS THE HOUSEHOLD'S, NEVER THE READER'S, so every phrase here is
	impersonal — "all 11 times this was cooked", not "you cooked". A recipe is
	held by a Kitchen rather than a person (ADR 0007) and `cooked.count` tallies
	every member's Attempts across the whole Lineage, the translation's
	included. "You cooked this 11 times", said to somebody who cooked it twice,
	is a plain untruth in the one sheet that most needs to be believed. The
	Cooked section above is worded impersonally for exactly this reason; these
	must not drift apart.
-->
{#if confirmingDelete && recipe}
	{@const title = recipe.versions.at(-1)?.content.title ?? ''}
	{@const cookings = recipe.cooked.count}
	<Confirm
		title={m.recipe_delete_title({ title })}
		consequence="{cookings === 0
			? m.recipe_delete_keeps_none()
			: cookings === 1
				? m.recipe_delete_keeps_one()
				: m.recipe_delete_keeps({ count: cookings })} {m.recipe_delete_gone()}"
		act={m.recipe_delete_act()}
		busy={deleting}
		failed={deleteFailed}
		run={deleteRecipe}
		cancel={() => (confirmingDelete = false)}
	>
		{#if shareIsLive === 'yes'}
			<p class="text-body text-support">{m.recipe_delete_shared()}</p>
		{:else if shareIsLive === 'unknown'}
			<p class="text-body text-support">{m.recipe_delete_share_unknown()}</p>
		{/if}
	</Confirm>
{/if}

<!--
	The sheet naming this version (#134). What landed is read back rather than
	patched in, so the chip and the line under the title say what the Core now
	holds.
-->
{#if renaming && recipe}
	<RenameSheet
		branchId={recipe.branch_id}
		name={recipe.name}
		title={renameLabel}
		onRenamed={() => {
			renaming = false;
			reread += 1;
		}}
		onClose={() => (renaming = false)}
	/>
{/if}

<!--
	The Language sheet (#106): what this recipe is written in, and the way into
	translating it. Raised over the page rather than a route, as `TagSheet` and
	`RelatedSheet` are — going to another screen and coming back loses the
	place you were standing in.
-->
{#if sayingLanguage && recipe}
	<LanguageSheet
		branchId={recipe.branch_id}
		language={recipe.language}
		inAFamily={inALanguageFamily}
		taken={languagesTaken}
		onSaid={(landed) => {
			sayingLanguage = false;
			// Saying this about a recipe another Kitchen writes is a Copy like
			// any other change (ADR 0020), so the Version may have landed on a
			// new Branch and the page follows the recipe the cook now holds.
			if (!follow(landed.branch_id)) reread += 1;
		}}
		onTranslate={(into) => {
			sayingLanguage = false;
			// The writing screen opens on THIS recipe's words, which is what a
			// person replaces to translate it. It is the same screen as an
			// edit on purpose: a translation is a recipe.
			translatingInto = into;
			writing = true;
		}}
		onClose={() => (sayingLanguage = false)}
	/>
{/if}

<style>
	/* Whose recipe you are reading colours the page: indigo is yours, beni is
	   anybody else's (#131, screen choice 1). A variation of your own is
	   yours, although it is read against your main one as `theirs`, so the
	   colour follows who writes the page and not which side the marks take.
	   It is what lets the marking stay quiet on the lines themselves, and
	   `MarkedRow` reads `--whose` for its margin rule. */
	[data-whose='mine'] {
		--whose: var(--color-accent);
	}
	[data-whose='theirs'] {
		--whose: var(--color-support);
	}

	/* The wash under the title is `@utility wash` in ui/src/app.css. It moved
	   there in #65, which gave the Share Link page the same hero and the
	   messaging-app card a picture of it: one gradient rendered three ways is
	   exactly what the stylesheet exists to keep from drifting. Its fitted
	   height and stops, and why they are what they are, are recorded beside it.
	*/
</style>
