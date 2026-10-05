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
	// What the page is made of. The screen keeps its own state — the recipe it
	// is reading, the Thread around it, the Divergence, how much it is read at,
	// and what the last save did — and composes the rest. Each concern that
	// stands on its own lives beside it (#188):
	//
	//   · `sheet.svelte.ts` and `SheetAction`: printing a Sheet (#75, #149);
	//   · `DeleteRecipe`: deleting it (#120);
	//   · `on-the-phone.svelte.ts`: the kept copy and the refresh counter (#76);
	//   · `unfolding.svelte.ts`, `ComponentLine`, `Unfolded` and `Annexe`: its
	//     Components (#50);
	//   · `corrections.svelte.ts`: correcting a Reading in place (#32 item 193);
	//   · `marking.svelte.ts` and `Carrying`: the Divergence's marks and what
	//     was carried across;
	//   · `my-pictures.svelte.ts` and `MyPictures`: your pictures of the
	//     dish (#110);
	//   · `HowMuchRow`, `ShoppingListButton`, `MetaStrip`, `Cooked`,
	//     `language-family.ts`, `sharing-a-chain.ts` and `thread-line.ts`:
	//     the rest.
	//
	// The sheets a component raises — deleting's confirmation, the save of
	// what was carried, a picture onto the recipe — are drawn by that
	// component, on the reading page. Each is modal, so nothing can swap the
	// page to Writing while one is open.
	//
	// State that has to outlive the reading page being swapped out for writing
	// (#83) is made here, once, by a `.svelte.ts` module; a component drawn on
	// the reading page holds only what may be forgotten when it goes.

	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { DivergenceOutput, GetRecipeOutput, GetThreadOutput } from '$lib/api/catalogue';
	import Hero from './Hero.svelte';
	import VersionStrip from './VersionStrip.svelte';
	import { branchPlainName, yoursAmong } from '$lib/cookbook';
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
	import MetaStrip from './MetaStrip.svelte';
	import ComponentLine from './ComponentLine.svelte';
	import Unfolded from './Unfolded.svelte';
	import Annexe from './Annexe.svelte';
	import HowMuchRow from './HowMuchRow.svelte';
	import Cooked from './Cooked.svelte';
	import MyPictures from './MyPictures.svelte';
	import SheetAction from './SheetAction.svelte';
	import ShoppingListButton from './ShoppingListButton.svelte';
	import DeleteRecipe from './DeleteRecipe.svelte';
	import Carrying from './Carrying.svelte';
	import { asBranchLanguage, type WrittenLanguage } from '$lib/language';
	import { tappableLink } from '$lib/source';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import StepPhoto from '$lib/StepPhoto.svelte';
	import { thisDevice, type Device } from '$lib/offline/device.svelte';
	import { useFiles } from '$lib/api/files';
	import { same, toSearch, type Wanted } from '$lib/how-much';
	import { takePaste, type PastedDraft } from '$lib/pasted.svelte';
	import { nutritionText, numbering, rowKey } from './divergence';
	import { sheet } from './sheet.svelte';
	import { onThePhone } from './on-the-phone.svelte';
	import { unfolding } from './unfolding.svelte';
	import { corrections, type Slot } from './corrections.svelte';
	import { marking } from './marking.svelte';
	import { myPictures } from './my-pictures.svelte';
	import { languageFamily } from './language-family';
	import { threadLine } from './thread-line';
	import { sharingAChain } from './sharing-a-chain';

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
	const files = useFiles();

	let recipe = $state<GetRecipeOutput | undefined>(undefined);
	let divergence = $state<DivergenceOutput | undefined>(undefined);
	/**
	 * Every version of this recipe the reader may see, for the switch (#131):
	 * the Branches of the Lineage that share a history with this one, which is
	 * every Copy and variation of it and never a Translation (see below).
	 */
	let versions = $state<GetThreadOutput['branches']>([]);
	/** Your own version, the one the others are marked against (#136). */
	let yoursId = $state<string | undefined>();
	let failed = $state(false);
	/**
	 * The cookings of this dish, which the Thread already answers. Kept because
	 * one of them may hold an As Cooked nobody has decided about yet (#58) —
	 * the recipe screen is where Promotion is offered.
	 */
	let attempts = $state<GetThreadOutput['attempts']>([]);
	/**
	 * Every Branch of this Lineage the reader can reach, which the Thread
	 * already answers and which this screen used to read for one fact and
	 * throw away. It is what tells a recipe it exists in another Language
	 * (#106, ADR 0006): see `language-family.ts`.
	 */
	let lineageBranches = $state<GetThreadOutput['branches']>([]);
	/**
	 * Every Version on every Branch of this Lineage, which the Thread answers
	 * anyway. The button into it says how many are this Branch's own and when
	 * the last was saved (#133), so a cook can tell before tapping whether
	 * anything is behind it.
	 */
	let lineageVersions = $state<GetThreadOutput['versions']>([]);
	const historyLine = $derived(threadLine(lineageVersions, branchId));
	/** Bumped after a Promotion, to read the recipe back with its new Version. */
	let reread = $state(0);

	/** The Divergence's marks, and what has been carried across from it. */
	const divergent = marking({
		kamosu,
		divergence: () => divergence,
		otherKitchen: () => otherKitchen,
		// The list is rebuilt from a different set of rows, so nothing keyed by
		// a line's index or path may stay open.
		onToggled: () => {
			fixes.close();
			unfold.closeAll();
		},
		// A new Version can move a line's index, and corrections are keyed by
		// index. None survives the save.
		onSaved: () => fixes.forget(),
	});

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

	/** Printing a Sheet (#75), which outlives the reading page (see `sheet.svelte.ts`). */
	const paper = sheet(kamosu, files, () => device);

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

	/** This recipe among the other Languages of its Lineage (#106). */
	const family = $derived(languageFamily(branchId, recipe, lineageBranches));

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
		// A save moves the head Version, and corrections are keyed by a line's
		// index into the list that just changed underneath them.
		fixes.forget();
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

	/** Your own pictures of this dish (#110), read once per dish. */
	const pictures = myPictures(kamosu, () => recipe?.lineage_id);

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
	const offeredLanguage = $derived.by(() => {
		const offered =
			copiedInto?.branchId === branchId
				? copiedInto.languageOffer
				: (wrote?.language_offer ?? null);
		return offered ? asBranchLanguage(offered) : null;
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
			fixes.dropOverlay();
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

	// ---- on the phone (#76) ---------------------------------------------

	const phone = onThePhone(
		() => branchId,
		() => (reread += 1),
	);

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
	$effect(() => {
		if (saidFor !== branchId) {
			saidFor = branchId;
			untrack(() => {
				wrote = undefined;
				named = undefined;
				// The comparison was the last version's. Walking along the strip
				// to your own must not carry its marks, or anything taken from
				// it, onto a page that has nothing to compare (#131).
				divergence = undefined;
				divergent.reset();
				choosingHowMuch = false;
				howMuchFailed = false;
				// A Sheet being set was the last recipe's, and stops being
				// waited on (#117); a Sheet ready to share was that recipe's
				// too (#149).
				paper.leave();
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
				// Every Copy and variation of it, and never a Translation: see
				// `sharing-a-chain.ts` for why a Translation cannot be paired.
				versions = sharingAChain(thread, branchId);
				// **Marks compare against your own** (#131, screen choice 1): the
				// version in your own Cookbook that you wrote, unnamed or, once
				// named, kept longest (#136). On it there is nothing to mark; on
				// any other version the rows are read with yours as `mine`, and
				// the page draws `theirs`. With none of your own there is
				// nothing to compare against at all.
				const yours = yoursAmong(versions);
				yoursId = yours?.branch_id;
				if (!yours || yours.branch_id === branchId) return;

				// Read on its own, so a refusal costs the Divergence and never
				// the recipe. Whatever the Core declines to pair, the cook is
				// still holding a recipe they can read and cook from.
				divergence = await kamosu
					.divergence({ branch_id: yours.branch_id, other_branch_id: branchId })
					.catch((error: unknown) => {
						if (!(error instanceof OperationError)) throw error;
						return undefined;
					});
				if (current && divergence) divergent.side = 'theirs';
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

	const here = $derived(divergent.side === 'mine' ? divergence?.mine : divergence?.theirs);
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

	const unfold = unfolding(() => components);

	// ---- correcting a Reading --------------------------------------------

	const fixes = corrections({
		readings: () => readings,
		measured: () => measured,
		written: (index) => content?.ingredients?.[index]?.text ?? '',
	});

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

	function corrected(index: number, reading: Slot, measuredLine: string | null) {
		// On a page read at a named amount the answer's line was worded at the
		// cooking's amount instead, so none is laid over until the page is read
		// again at its own — rather than a figure for the wrong amount (#109).
		fixes.lay(index, reading, named === undefined ? measuredLine : null);

		// **A correction on a line that is, or was, a Component re-reads the
		// recipe** (#87). Everything else a correction changes is laid over the
		// Readings — but a Component is not a Reading on this page, it is the
		// Core's unfolding of one, and every part of that is worked out on the
		// server from the Reading that just changed: which recipe it names, how
		// much of it is wanted, and the inner recipe's own lines already scaled
		// by that share (ADR 0008).
		//
		// So the pointer is not the only thing worth refetching for. Correcting
		// `500 g` to `250 g` on a dough leaves the pointer alone and halves the
		// share, and without this the row would read 250 g above an unfolding
		// still scaled at 500 — the line contradicting its own panel.
		//
		// An ordinary line's correction still refetches nothing: there is
		// nothing on screen for it that the overlay does not already hold.
		const wasComponent = unfold.at([], index) !== undefined;
		const isNowComponent = (reading?.lineage_id ?? null) !== null;
		if (wasComponent || isNowComponent) {
			reread += 1;
		} else if (named !== undefined) {
			// `set_reading` words the corrected line at the amount a cooking is
			// at, and knows nothing of this page's: read it again at this one.
			void chooseHowMuch(named, false);
		}
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
				fixes.forget();
				follow(landed.branch_id);
				return;
			}
			afterSave(landed);
		}}
	/>
{:else}
	<div
		class="mx-auto pb-tabbar {divergence ? 'max-w-2xl' : 'recipe-page'}"
		data-side={divergent.side}
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
					yours={yoursId}
					compared={divergence ? { unshared: divergent.unshared, with: otherKitchen } : undefined}
					marks={divergent.marks}
					toggleMarks={divergent.toggleMarks}
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
			{#if divergent.markOf('source')}
				<p class="px-gutter pt-2 text-read text-accent">{divergent.markOf('source')}</p>
			{/if}
			{#if divergent.markOf('title')}
				<p class="px-gutter pt-2 text-read text-accent">{divergent.markOf('title')}</p>
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
					others={family.others}
				/>
			{/if}

			<MetaStrip {content} />
			{#each ['prep_time_minutes', 'cook_time_minutes', 'yield'] as const as name (name)}
				{#if divergent.markOf(name)}
					<p class="mt-1 px-gutter text-read text-accent">{divergent.markOf(name)}</p>
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

			{#if phone.onlyKept && phone.keptOn}
				<p class="mt-3 px-gutter text-read text-ink-2">
					{m.offline_kept_from({ date: phone.keptOn.toLocaleDateString() })}
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
				{@const component = unfold.at([], at)}
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
								aria-expanded={fixes.correcting === at}
								onclick={() => fixes.toggle(at)}
							>
								{@render written(text, at, Boolean(component))}
							</button>
						{:else}
							{@render written(text, at, Boolean(component))}
						{/if}
						{#if component}
							<ComponentLine
								{component}
								open={unfold.isOpen(component.path)}
								toggle={() => unfold.toggle(component.path)}
							/>
						{/if}
						{#if readable && fixes.correcting === at}
							<Correcting
								{branchId}
								lineIndex={at}
								line={text}
								reading={fixes.readingAt(at)}
								componentTitle={component?.title}
								onDone={(next, converted) => corrected(at, next, converted)}
								onCancel={fixes.close}
							/>
						{/if}
						{#if component && unfold.isOpen(component.path)}
							<Unfolded {component} at={unfold.at} />
						{/if}
					</div>
				</li>
			{/snippet}

			<!--
			The written Line, and beneath it the one subordinate line — smaller,
			quieter, and simply absent where there is nothing to say. Nothing here
			says whether it is a conversion or an echo, and nothing says whether
			Kamosu read the line at all (ADR 0002).

			A COMPONENT'S SLOT IS FILLED BY `ComponentLine` INSTEAD, outside this
			snippet — it is a target, and this one is rendered inside the button
			that opens the corrector.
		-->
			{#snippet written(text: string, at: number, isComponent: boolean)}
				<span class="block text-line">{text}</span>
				{#if !isComponent && pageScaledTo && at >= 0 && !measured.ingredients[at]}
					<!-- Asked for another amount, and this line did not move (#109). -->
					<span class="block text-read text-ink-2">{m.how_much_not_scaled()}</span>
				{:else if !isComponent && fixes.beneathLine(at)}
					<span class="block text-read text-ink-2">{fixes.beneathLine(at)}</span>
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
			TWO COLUMNS WHERE THE WINDOW IS ROOMY (#198, ADR 0044): the Ingredients
			on the left in two fifths, the Method on the right in three, and the
			Ingredients staying in view while the Method scrolls. Aurélien chose
			this on 5 October 2026 over two even halves that both scroll away, and
			over a narrower Ingredients column beside the photograph. Everywhere
			else, a phone and an upright tablet included, these are plain boxes
			and the page is the one column it was.

			The part that stays is a box INSIDE the left column, and the column
			is as tall as the Method. Making the column itself the part that
			stays held it for the whole page, and it rode over Related recipes,
			Cooked and the buttons; he found that on the iPad.

			NOT WHILE THE PAGE IS A DIVERGENCE. A recipe read against another
			Branch stays one column at reading width (#192, story 64): its marks
			and Ghosts are read down one list, and nobody chose a layout for
			them side by side.
		-->
			<div class={{ 'recipe-columns': !divergence }}>
				<div>
					<div class={{ 'stays-in-view': !divergence }}>
						<!-- Ingredients ------------------------------------------------------ -->
						<h2
							class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase"
						>
							{m.recipe_ingredients()}
						</h2>
						{#if !divergence}
							<HowMuchRow
								written={content.yield}
								scaledTo={pageScaledTo}
								bind:choosing={choosingHowMuch}
								failed={howMuchFailed}
								onchoose={(chosen) => void chooseHowMuch(chosen)}
							/>
						{/if}
						<ul class="px-gutter">
							{#if divergent.showing && divergence}
								{#each divergence.ingredients as row, index (rowKey('ingredients', index))}
									{@const key = rowKey('ingredients', index)}
									{@const own = divergent.side === 'mine' ? row.mine : row.theirs}
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
										{@const marked = own ? unfold.at([], own.index) : undefined}
										<MarkedRow
											{row}
											side={divergent.side}
											{otherKitchen}
											beneath={marked?.said ?? (own ? fixes.beneathLine(own.index) : '')}
											open={divergent.isOpen(key)}
											taken={divergent.taken.get(key)}
											onToggle={() => divergent.toggle(key)}
											onCarry={(what) => divergent.carry(key, what)}
											onFixReading={correctable && own ? () => fixes.toggle(own.index) : undefined}
										/>
										{#if correctable && own && fixes.correcting === own.index}
											<li class="border-b border-rule pb-3 pl-3">
												<Correcting
													{branchId}
													lineIndex={own.index}
													line={own.text}
													reading={fixes.readingAt(own.index)}
													componentTitle={marked?.title}
													onDone={(next, converted) => corrected(own.index, next, converted)}
													onCancel={fixes.close}
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
						{#if divergent.markOf('nutrition')}
							<p class="mt-1 px-gutter text-read text-accent">{divergent.markOf('nutrition')}</p>
						{/if}
					</div>
				</div>
				<div>
					<!-- Method ----------------------------------------------------------- -->
					<h2
						class="mx-gutter mt-8 mb-2 font-display text-label font-semibold text-accent uppercase"
					>
						{m.recipe_method()}
					</h2>
					<ol class="px-gutter">
						{#if divergent.showing && divergence}
							{@const number = numbering()}
							{#each divergence.steps as row, index (rowKey('steps', index))}
								{@const key = rowKey('steps', index)}
								{@const own = divergent.side === 'mine' ? row.mine : row.theirs}
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
										side={divergent.side}
										{otherKitchen}
										number={n}
										conversions={own ? (measured.steps[own.index] ?? []) : []}
										photo={own ? (content.steps[own.index]?.photo ?? null) : null}
										open={divergent.isOpen(key)}
										taken={divergent.taken.get(key)}
										onToggle={() => divergent.toggle(key)}
										onCarry={(what) => divergent.carry(key, what)}
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
					{#each unfold.annexes as component (component.path.join('.'))}
						<Annexe {component} />
					{/each}

					{#if content.note}
						<div
							class="mx-gutter mt-6 border-l-2 border-accent py-1 pl-4 text-body whitespace-pre-wrap"
						>
							{content.note}
						</div>
					{/if}
					{#if divergent.markOf('note')}
						<p class="mx-gutter mt-1 text-read text-accent">{divergent.markOf('note')}</p>
					{/if}
				</div>
			</div>

			<!--
			What follows the two columns goes back to the one column every other
			page is drawn in (#198). A button 1040 wide says no more than one 672
			wide, and a diary line that long is hard to read.
		-->
			<div class="roomy:mx-auto roomy:max-w-2xl">
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

				{#if recipe}
					<Cooked cooked={recipe.cooked}>
						<MyPictures
							{branchId}
							pictures={pictures.list}
							onPromoted={(landed) => {
								// Nothing to name: a promotion changes a photograph and
								// no Ingredient Line, so no Component is left behind.
								afterSave({ ...landed, named: true });
							}}
						/>
					</Cooked>
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

				{#if divergent.saved === 'yes'}
					<p class="mx-gutter mt-4 text-read text-accent" role="status">{m.divergence_saved()}</p>
				{:else if divergent.saved === 'failed'}
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
					{#if historyLine}
						<span class="mt-1 block font-sans text-read text-ink-2">{historyLine}</span>
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
				At the amount on screen: the page is what a Sheet prints (ADR 0023).
				Not while a Divergence is shown, where the amounts on screen are
				`divergence`'s and the scaler is not offered (`pageScaledTo`).
			-->
				<SheetAction
					sheet={paper}
					print={() => void paper.print(branchId, divergence ? undefined : named)}
				/>
				<ShoppingListButton {branchId} scaledTo={pageScaledTo} bind:onTheList />

				{#if recipe?.writes}
					<DeleteRecipe
						{branchId}
						title={recipe.versions.at(-1)?.content.title ?? ''}
						cookings={recipe.cooked.count}
					/>
				{/if}
			</div>
		{/if}

		{#if divergence}
			<Carrying marking={divergent} {otherKitchen} />
		{/if}
	</div>
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
		inAFamily={family.inAFamily}
		taken={family.taken}
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
