// Generated from the Catalogue by `just client`. Do not edit.
//
// Every Operation Kamosu offers is declared once in src/catalogue.rs and both
// Doors are built by walking that list (ADR 0001). These types are the third
// thing built from it, so the interface cannot ask for a shape the Core does
// not serve. Change the Catalogue and re-run `just client`; `just check`
// fails if what is committed here has drifted.

import type { JobAsk } from './job';

/** The version of this Kamosu and whether setup has happened. */
export type InstanceStatusInput = Record<string, never>;
/** What instance_status answers. */
export type InstanceStatusOutput = {
	setup_complete: boolean;
	version: string;
};

/** Set the Language and measures this Person reads in. */
export type SetReadingPreferencesInput = {
	reading_language: "en" | "fr" | "es";
	reading_measures: "us" | "metric" | "as_written";
};
/** What set_reading_preferences answers. */
export type SetReadingPreferencesOutput = {
	reading_language: string;
	reading_measures: string;
};

/** Change this Person's current reminder name. */
export type RenamePersonInput = {
	name: string;
};
/** What rename_person answers. */
export type RenamePersonOutput = {
	name: string;
};

/** Mint a one-use Invite link for a new Person. */
export type MintInviteInput = {
	is_operator?: boolean;
};
/** What mint_invite answers. */
export type MintInviteOutput = {
	link: string;
};

/** Disable an account so it can no longer obtain a Credential. */
export type DisableAccountInput = {
	name: string;
};
/** What disable_account answers. */
export type DisableAccountOutput = {
	disabled: boolean;
};

/** Delete an account while preserving its Hand in history. */
export type DeleteAccountInput = {
	name: string;
};
/** What delete_account answers. */
export type DeleteAccountOutput = {
	deleted: boolean;
};

/** Mint a one-use recovery link for a Person who forgot their password. */
export type MintRecoveryLinkInput = {
	name: string;
};
/** What mint_recovery_link answers. */
export type MintRecoveryLinkOutput = {
	link: string;
};

/** List this Person's browser Sessions by device and last use. */
export type ListSessionsInput = Record<string, never>;
/** What list_sessions answers. */
export type ListSessionsOutput = {
	sessions: {
		created_at: string;
		id: string;
		last_used_at: string | null;
		name: string;
		revoked: boolean;
	}[];
};

/** End one of your browser Sessions. */
export type RevokeSessionInput = {
	session_id: string;
};
/** What revoke_session answers. */
export type RevokeSessionOutput = {
	revoked: boolean;
};

/** Mint an Access Key for an agent to act as you, optionally read-only. */
export type MintAccessKeyInput = {
	name: string;
	read_only?: boolean;
};
/** What mint_access_key answers. */
export type MintAccessKeyOutput = {
	id: string;
	name: string;
	read_only: boolean;
	secret: string;
};

/** List this Person's Access Keys by name and last use. */
export type ListAccessKeysInput = Record<string, never>;
/** What list_access_keys answers. */
export type ListAccessKeysOutput = {
	access_keys: {
		created_at: string;
		id: string;
		last_used_at: string | null;
		name: string;
		read_only: boolean;
		revoked: boolean;
	}[];
};

/** End one of your Access Keys. */
export type RevokeAccessKeyInput = {
	access_key_id: string;
};
/** What revoke_access_key answers. */
export type RevokeAccessKeyOutput = {
	revoked: boolean;
};

/** Create a Kitchen: a new circle, held by its creator until they invite someone else in. */
export type CreateKitchenInput = {
	name: string;
};
/** What create_kitchen answers. */
export type CreateKitchenOutput = {
	hand_id: string;
	id: string;
	is_home: boolean;
	members: {
		name: string;
		person_id: string;
	}[];
	name: string;
	nickname: string | null;
};

/** List every Kitchen this Person cooks in. */
export type ListKitchensInput = Record<string, never>;
/** What list_kitchens answers. */
export type ListKitchensOutput = {
	kitchens: {
		hand_id: string;
		id: string;
		is_home: boolean;
		members: {
			name: string;
			person_id: string;
		}[];
		name: string;
		nickname: string | null;
	}[];
};

/** Change a Kitchen's shared Name. Any member may. */
export type RenameKitchenInput = {
	kitchen_id: string;
	name: string;
};
/** What rename_kitchen answers. */
export type RenameKitchenOutput = {
	name: string;
};

/** Set this member's own private Nickname for a Kitchen, seen by nobody else. An absent or empty Nickname clears it. */
export type SetKitchenNicknameInput = {
	kitchen_id: string;
	nickname: string | null;
};
/** What set_kitchen_nickname answers. */
export type SetKitchenNicknameOutput = {
	nickname: string | null;
};

/** Mint a one-use Invite for another Person to join this Kitchen. Any member may. */
export type InviteToKitchenInput = {
	kitchen_id: string;
};
/** What invite_to_kitchen answers. */
export type InviteToKitchenOutput = {
	invite_id: string;
	secret: string;
};

/** Open a Kitchen Invite: join the Kitchen it names. Spent on use. */
export type AcceptKitchenInviteInput = {
	secret: string;
};
/** What accept_kitchen_invite answers. */
export type AcceptKitchenInviteOutput = {
	hand_id: string;
	id: string;
	is_home: boolean;
	members: {
		name: string;
		person_id: string;
	}[];
	name: string;
	nickname: string | null;
};

/** Remove a Person from a Kitchen — including yourself, to leave. The last member cannot be removed. */
export type RemoveKitchenMemberInput = {
	kitchen_id: string;
	person_id: string;
};
/** What remove_kitchen_member answers. */
export type RemoveKitchenMemberOutput = {
	removed: boolean;
};

/** An Operator's power over a Kitchen: delete one nobody is left in. Nothing else about a Kitchen. */
export type DeleteKitchenInput = {
	kitchen_id: string;
};
/** What delete_kitchen answers. */
export type DeleteKitchenOutput = {
	deleted: boolean;
};

/** Create a Tag in a Kitchen, named in one Language. A word the Kitchen already files under returns the Tag it already has rather than making a second. */
export type CreateTagInput = {
	kitchen_id: string;
	language: "en" | "fr" | "es";
	name: string;
};
/** What create_tag answers. */
export type CreateTagOutput = {
	id: string;
	kitchen_id: string;
	language: string | null;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
};

/** List every Tag a Kitchen files by, each shown in the reader's Reading Language where it has a name there. */
export type ListTagsInput = {
	kitchen_id: string;
};
/** What list_tags answers. */
export type ListTagsOutput = {
	tags: {
		id: string;
		kitchen_id: string;
		language: string | null;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
	}[];
};

/** Name a Tag in one Language, or change the name it has there. Reaches every recipe carrying it at once, and mints no Version. */
export type RenameTagInput = {
	language: "en" | "fr" | "es";
	name: string;
	tag_id: string;
};
/** What rename_tag answers. */
export type RenameTagOutput = {
	id: string;
	kitchen_id: string;
	language: string | null;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
};

/** Merge two of a Kitchen's Tags into one: every recipe filed under the merged Tag is filed under the kept one instead. Mints no Version. */
export type MergeTagsInput = {
	keep_tag_id: string;
	merge_tag_id: string;
};
/** What merge_tags answers. */
export type MergeTagsOutput = {
	id: string;
	kitchen_id: string;
	language: string | null;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
};

/** Take a Tag out of a Kitchen's list and off every recipe carrying it. No recipe changes. */
export type DeleteTagInput = {
	tag_id: string;
};
/** What delete_tag answers. */
export type DeleteTagOutput = {
	deleted: boolean;
};

/** File a recipe under one of its Kitchen's Tags, or take it back out. Mints no Version: filing is not what a recipe is. */
export type SetRecipeTagInput = {
	branch_id: string;
	carried: boolean;
	tag_id: string;
};
/** What set_recipe_tag answers. */
export type SetRecipeTagOutput = {
	tags: {
		id: string;
		kitchen_id: string;
		language: string | null;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
	}[];
};

/** Relate two Recipes on the same Kitchen shelf, or take that single two-way, untyped link back off. It never changes either Recipe or travels in a Bundle or Share. */
export type SetRelatedRecipeInput = {
	branch_id: string;
	related: boolean;
	related_branch_id: string;
};
/** What set_related_recipe answers. */
export type SetRelatedRecipeOutput = {
	related_recipes: {
		branch_id: string | null;
		lineage_id: string;
		title: string;
	}[];
};

/** Create a Recipe: a Lineage, a Branch in this Kitchen, and a first Version. A title is all it needs. */
export type CreateRecipeInput = {
	cook_time_minutes?: number | null;
	ingredients?: {
		kind: "section" | "ingredient";
		text: string;
	}[];
	kitchen_id: string;
	language?: "en" | "fr" | "es";
	main_photo?: string | null;
	note?: string | null;
	prep_time_minutes?: number | null;
	source?: {
		link: string | null;
		text: string;
	} | null;
	steps?: {
		kind: "section" | "step";
		photo: string | null;
		text: string;
	}[];
	title: string;
	yield?: {
		amount: string;
		noun: string;
	} | null;
};
/** What create_recipe answers. */
export type CreateRecipeOutput = {
	branch_id: string;
	hand_id: string;
	head_version_id: string;
	kitchen_id: string;
	language: string;
	lineage_id: string;
	origin_address: string | null;
	related_recipes: {
		branch_id: string | null;
		lineage_id: string;
		title: string;
	}[];
	tags: {
		id: string;
		kitchen_id: string;
		language: string | null;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
	}[];
	versions: {
		change_note: string | null;
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			prep_time_minutes: number | null;
			source: {
				link: string | null;
				text: string;
			} | null;
			steps: {
				kind: "section" | "step";
				photo: string | null;
				text: string;
			}[];
			title: string;
			yield: {
				amount: string;
				noun: string;
			} | null;
		};
		created_at: string;
		hand_id: string;
		name: string | null;
		parent_version_id: string | null;
		readings: {
			amount: string | null;
			target: string | null;
			unit: string | null;
		} | null[];
		sequence: number;
		version_id: string;
	}[];
};

/** Save a new state of a Recipe onto a Branch — the whole recipe as written, replacing what was there. A rapid re-save by the same Hand collapses into the Version already being shaped rather than starting a new one. Changing a recipe your Kitchen did not write is a Copy: it starts a new Branch of the same Lineage, held by your Kitchen, starting at the Version you changed and carrying the whole chain behind it — the Branch you changed is left untouched. */
export type SaveRecipeVersionInput = {
	branch_id: string;
	change_note?: string;
	cook_time_minutes?: number | null;
	ingredients?: {
		kind: "section" | "ingredient";
		text: string;
	}[];
	kitchen_id?: string;
	main_photo?: string | null;
	name?: string;
	note?: string | null;
	prep_time_minutes?: number | null;
	source?: {
		link: string | null;
		text: string;
	} | null;
	steps?: {
		kind: "section" | "step";
		photo: string | null;
		text: string;
	}[];
	title: string;
	yield?: {
		amount: string;
		noun: string;
	} | null;
};
/** What save_recipe_version answers. */
export type SaveRecipeVersionOutput = {
	branch_id: string;
	collapsed: boolean;
	copied: boolean;
	parent_version_id: string | null;
	sequence: number;
	version_id: string;
};

/** Bring a batch of already-read recipes into your Home Kitchen, as a Job. Matched by foreign id against this Kitchen's ledger for the source kind, so re-running finds what it already made instead of doubling it; a recipe found changed is offered for review, never written over. Reading the outside source itself — a file, a page, a Bundle — is each importer's own job. */
export type ImportInput = {
	candidates: {
		cook_time_minutes?: number | null;
		foreign_id: string;
		ingredients?: {
			kind: "section" | "ingredient";
			text: string;
		}[];
		language?: "en" | "fr" | "es";
		main_photo?: string | null;
		note?: string | null;
		prep_time_minutes?: number | null;
		source?: {
			link: string | null;
			text: string;
		} | null;
		steps?: {
			kind: "section" | "step";
			photo: string | null;
			text: string;
		}[];
		title: string;
		yield?: {
			amount: string;
			noun: string;
		} | null;
	}[];
	source_kind: string;
};
/** What import eventually produces, read back through `get_job`. */
export type ImportOutput = {
	arrived: {
		branch_id: string;
		foreign_id: string;
		lineage_id: string;
		status: "created" | "unchanged";
		title: string;
	}[];
	import_id: string;
	kitchen_id: string;
	offered: {
		branch_id: string;
		candidate_version_id: string;
		foreign_id: string;
		lineage_id: string;
		title: string;
	}[];
	source_kind: string;
	unreadable: {
		foreign_id: string | null;
		reason: string;
	}[];
};

/** Bring in a recipe straight from a URL, as a Job. Reads the page's schema.org JSON-LD (#70) — no per-site scraping, no LLM fallback — and lands it in your Home Kitchen through the same ledger `import` uses, keyed by the page's own address. Fetching is bound to public addresses at the dialled address and at every redirect (ADR 0033), and — because a page's own text can tell an agent to fetch another URL — always takes the single depth-one lane, never more than one fetch in flight regardless of who is signed in. */
export type ImportWebLinkInput = {
	url: string;
};
/** What import_web_link eventually produces, read back through `get_job`. */
export type ImportWebLinkOutput = {
	arrived: {
		branch_id: string;
		foreign_id: string;
		lineage_id: string;
		status: "created" | "unchanged";
		title: string;
	}[];
	import_id: string;
	kitchen_id: string;
	offered: {
		branch_id: string;
		candidate_version_id: string;
		foreign_id: string;
		lineage_id: string;
		title: string;
	}[];
	source_kind: string;
	unreadable: {
		foreign_id: string | null;
		reason: string;
	}[];
};

/** Rename a Version — the one thing about it that can change later. An absent or empty name clears it. Targeted by the Branch's own sequence number, since the same content can recur more than once on one Branch, each occurrence named on its own. */
export type RenameVersionInput = {
	branch_id: string;
	name: string | null;
	sequence: number;
};
/** What rename_version answers. */
export type RenameVersionOutput = {
	name: string | null;
};

/** Upload a Photograph, base64-encoded — the fallback for a Door that cannot carry raw bytes (ADR 0001). A browser uses the out-of-band `POST /api/photographs` instead. Two uploads of the same picture answer the same id. */
export type UploadPhotographInput = {
	data: string;
};
/** What upload_photograph answers. */
export type UploadPhotographOutput = {
	photograph_id: string;
};

/** Read a Recipe: the Branch as it stands and its whole chain of Versions, oldest first. */
export type GetRecipeInput = {
	branch_id: string;
};
/** What get_recipe answers. */
export type GetRecipeOutput = {
	branch_id: string;
	hand_id: string;
	head_version_id: string;
	kitchen_id: string;
	language: string;
	lineage_id: string;
	origin_address: string | null;
	related_recipes: {
		branch_id: string | null;
		lineage_id: string;
		title: string;
	}[];
	tags: {
		id: string;
		kitchen_id: string;
		language: string | null;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
	}[];
	versions: {
		change_note: string | null;
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			prep_time_minutes: number | null;
			source: {
				link: string | null;
				text: string;
			} | null;
			steps: {
				kind: "section" | "step";
				photo: string | null;
				text: string;
			}[];
			title: string;
			yield: {
				amount: string;
				noun: string;
			} | null;
		};
		created_at: string;
		hand_id: string;
		name: string | null;
		parent_version_id: string | null;
		readings: {
			amount: string | null;
			target: string | null;
			unit: string | null;
		} | null[];
		sequence: number;
		version_id: string;
	}[];
};

/** Correct the Reading on one Ingredient Line of a Recipe's current state — an amount, a Unit and a target, sent together as the whole new Reading (never a per-field patch, the same convention save_recipe_version uses for the whole recipe). Mints no Version and appears in no history (ADR 0021). Amount, Unit and target left out together clears the Reading, taking the line back to fully unread. */
export type SetReadingInput = {
	amount?: string | null;
	branch_id: string;
	line_index: number;
	target?: string | null;
	unit?: string | null;
};
/** What set_reading answers. */
export type SetReadingOutput = {
	line_index: number;
	reading: {
		amount: string | null;
		target: string | null;
		unit: string | null;
	} | null;
};

/** Start cooking a Recipe: creates the Attempt, or hands back the one already In Progress for this Lineage — the cooking screen is that Attempt, never a second thing beside it. Pinned by fingerprint to the Branch's head Version at this moment. Anyone who can see the recipe may. */
export type StartAttemptInput = {
	branch_id: string;
};
/** What start_attempt answers. */
export type StartAttemptOutput = {
	cooking_yield: {
		amount: string;
		noun: string;
	} | null;
	created_at: string;
	current_step_index: number;
	finished_at: string | null;
	id: string;
	last_action_at: string;
	lineage_id: string;
	note: string | null;
	person_id: string;
	rating: number | null;
	resumable: boolean;
	ticked_ingredients: number[];
	version_id: string;
};

/** Move an In Progress Attempt forward: which Step, which Ingredients are ticked, and the Yield being cooked to — a fact about this cooking, never a deviation. Any of the three, each sent whole rather than patched. */
export type AdvanceAttemptInput = {
	attempt_id: string;
	cooking_yield?: {
		amount: string;
		noun: string;
	} | null;
	current_step_index?: number;
	ticked_ingredients?: number[];
};
/** What advance_attempt answers. */
export type AdvanceAttemptOutput = {
	cooking_yield: {
		amount: string;
		noun: string;
	} | null;
	created_at: string;
	current_step_index: number;
	finished_at: string | null;
	id: string;
	last_action_at: string;
	lineage_id: string;
	note: string | null;
	person_id: string;
	rating: number | null;
	resumable: boolean;
	ticked_ingredients: number[];
	version_id: string;
};

/** End an In Progress Attempt. Ending is not what makes the cooking real — starting already did — only what stops it being In Progress. */
export type FinishAttemptInput = {
	attempt_id: string;
};
/** What finish_attempt answers. */
export type FinishAttemptOutput = {
	cooking_yield: {
		amount: string;
		noun: string;
	} | null;
	created_at: string;
	current_step_index: number;
	finished_at: string | null;
	id: string;
	last_action_at: string;
	lineage_id: string;
	note: string | null;
	person_id: string;
	rating: number | null;
	resumable: boolean;
	ticked_ingredients: number[];
	version_id: string;
};

/** Change an Attempt's free text or its five-star rating, whether it is still In Progress or long finished — an Attempt is freely editable by its cook, unlike the recipe it was cooked from. */
export type EditAttemptInput = {
	attempt_id: string;
	note?: string | null;
	rating?: number | null;
};
/** What edit_attempt answers. */
export type EditAttemptOutput = {
	cooking_yield: {
		amount: string;
		noun: string;
	} | null;
	created_at: string;
	current_step_index: number;
	finished_at: string | null;
	id: string;
	last_action_at: string;
	lineage_id: string;
	note: string | null;
	person_id: string;
	rating: number | null;
	resumable: boolean;
	ticked_ingredients: number[];
	version_id: string;
};

/** Delete an Attempt outright — the explicit way a false start is undone, or any cooking record put away. Never soft-deleted: this is the whole of how an Attempt leaves. */
export type DeleteAttemptInput = {
	attempt_id: string;
};
/** What delete_attempt answers. */
export type DeleteAttemptOutput = {
	deleted: boolean;
};

/** Read the caller's own In Progress Attempt for a Lineage, if any — how two devices cooking the same dish stay in step, and whether resuming should still be offered. */
export type GetCurrentAttemptInput = {
	lineage_id: string;
};
/** What get_current_attempt answers. */
export type GetCurrentAttemptOutput = {
	attempt: {
		cooking_yield: {
			amount: string;
			noun: string;
		} | null;
		created_at: string;
		current_step_index: number;
		finished_at: string | null;
		id: string;
		last_action_at: string;
		lineage_id: string;
		note: string | null;
		person_id: string;
		rating: number | null;
		resumable: boolean;
		ticked_ingredients: number[];
		version_id: string;
	} | null;
};

/** List every Food this instance knows, each shown in the reader's Reading Language where it has a name there. */
export type ListFoodsInput = Record<string, never>;
/** What list_foods answers. */
export type ListFoodsOutput = {
	foods: {
		cup_weight_grams: number | null;
		id: string;
		language: string | null;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
		nutrition: null;
		reading_count: number;
	}[];
};

/** Read one Food: its names, its Cup Weight, and how many Readings currently point at it. */
export type GetFoodInput = {
	food_id: string;
};
/** What get_food answers. */
export type GetFoodOutput = {
	cup_weight_grams: number | null;
	id: string;
	language: string | null;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
	nutrition: null;
	reading_count: number;
};

/** Give a Food its name in one Language, or correct the one it has there. Any Person may — a Food is instance-wide, not a Kitchen's to guard. */
export type SetFoodNameInput = {
	food_id: string;
	language: "en" | "fr" | "es";
	name: string;
};
/** What set_food_name answers. */
export type SetFoodNameOutput = {
	cup_weight_grams: number | null;
	id: string;
	language: string | null;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
	nutrition: null;
	reading_count: number;
};

/** Take a Food's name in one Language back off. A Food's last remaining name may not be removed this way. */
export type RemoveFoodNameInput = {
	food_id: string;
	language: "en" | "fr" | "es";
};
/** What remove_food_name answers. */
export type RemoveFoodNameOutput = {
	cup_weight_grams: number | null;
	id: string;
	language: string | null;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
	nutrition: null;
	reading_count: number;
};

/** Set or clear a Food's Cup Weight — the one figure that turns a volume of it into a weight. Anyone may correct it. */
export type SetFoodCupWeightInput = {
	cup_weight_grams: number | null;
	food_id: string;
};
/** What set_food_cup_weight answers. */
export type SetFoodCupWeightOutput = {
	cup_weight_grams: number | null;
	id: string;
	language: string | null;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
	nutrition: null;
	reading_count: number;
};

/** Read one Job: its state, its progress, and its result or the reason it failed. Readable by the Person who asked, or by anyone when no Person did. */
export type GetJobInput = {
	job_id: string;
};
/** What get_job answers. */
export type GetJobOutput = {
	created_at: string;
	error: string | null;
	errorCode: number | null;
	id: string;
	operation: string;
	progress: {
		done?: number | null;
		message?: string | null;
		total?: number | null;
	};
	result: unknown;
	status: "queued" | "running" | "completed" | "failed" | "cancelled";
	updated_at: string;
};

/** Cancel a Job you asked for: acknowledged always, honoured while it still waits in line. */
export type CancelJobInput = {
	job_id: string;
};
/** What cancel_job answers. */
export type CancelJobOutput = {
	cancelled: boolean;
};

/** List the Jobs this Person has asked for, newest first. */
export type ListJobsInput = Record<string, never>;
/** What list_jobs answers. */
export type ListJobsOutput = {
	jobs: {
		created_at: string;
		id: string;
		operation: string;
		status: "queued" | "running" | "completed" | "failed" | "cancelled";
	}[];
};

/**
 * Every Operation, by the name both Doors use for it. An Immediate
 * Operation answers its output; a Job answers a job id at once and its
 * output arrives later, through `get_job` (ADR 0032).
 */
export interface Operations {
	instance_status: {
		input: InstanceStatusInput;
		output: InstanceStatusOutput;
		kind: 'immediate';
		permission: 'public';
	};
	set_reading_preferences: {
		input: SetReadingPreferencesInput;
		output: SetReadingPreferencesOutput;
		kind: 'immediate';
		permission: 'person';
	};
	rename_person: {
		input: RenamePersonInput;
		output: RenamePersonOutput;
		kind: 'immediate';
		permission: 'person';
	};
	mint_invite: {
		input: MintInviteInput;
		output: MintInviteOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	disable_account: {
		input: DisableAccountInput;
		output: DisableAccountOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	delete_account: {
		input: DeleteAccountInput;
		output: DeleteAccountOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	mint_recovery_link: {
		input: MintRecoveryLinkInput;
		output: MintRecoveryLinkOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	list_sessions: {
		input: ListSessionsInput;
		output: ListSessionsOutput;
		kind: 'immediate';
		permission: 'person';
	};
	revoke_session: {
		input: RevokeSessionInput;
		output: RevokeSessionOutput;
		kind: 'immediate';
		permission: 'person';
	};
	mint_access_key: {
		input: MintAccessKeyInput;
		output: MintAccessKeyOutput;
		kind: 'immediate';
		permission: 'person';
	};
	list_access_keys: {
		input: ListAccessKeysInput;
		output: ListAccessKeysOutput;
		kind: 'immediate';
		permission: 'person';
	};
	revoke_access_key: {
		input: RevokeAccessKeyInput;
		output: RevokeAccessKeyOutput;
		kind: 'immediate';
		permission: 'person';
	};
	create_kitchen: {
		input: CreateKitchenInput;
		output: CreateKitchenOutput;
		kind: 'immediate';
		permission: 'person';
	};
	list_kitchens: {
		input: ListKitchensInput;
		output: ListKitchensOutput;
		kind: 'immediate';
		permission: 'person';
	};
	rename_kitchen: {
		input: RenameKitchenInput;
		output: RenameKitchenOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_kitchen_nickname: {
		input: SetKitchenNicknameInput;
		output: SetKitchenNicknameOutput;
		kind: 'immediate';
		permission: 'person';
	};
	invite_to_kitchen: {
		input: InviteToKitchenInput;
		output: InviteToKitchenOutput;
		kind: 'immediate';
		permission: 'person';
	};
	accept_kitchen_invite: {
		input: AcceptKitchenInviteInput;
		output: AcceptKitchenInviteOutput;
		kind: 'immediate';
		permission: 'person';
	};
	remove_kitchen_member: {
		input: RemoveKitchenMemberInput;
		output: RemoveKitchenMemberOutput;
		kind: 'immediate';
		permission: 'person';
	};
	delete_kitchen: {
		input: DeleteKitchenInput;
		output: DeleteKitchenOutput;
		kind: 'immediate';
		permission: 'person';
	};
	create_tag: {
		input: CreateTagInput;
		output: CreateTagOutput;
		kind: 'immediate';
		permission: 'person';
	};
	list_tags: {
		input: ListTagsInput;
		output: ListTagsOutput;
		kind: 'immediate';
		permission: 'person';
	};
	rename_tag: {
		input: RenameTagInput;
		output: RenameTagOutput;
		kind: 'immediate';
		permission: 'person';
	};
	merge_tags: {
		input: MergeTagsInput;
		output: MergeTagsOutput;
		kind: 'immediate';
		permission: 'person';
	};
	delete_tag: {
		input: DeleteTagInput;
		output: DeleteTagOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_recipe_tag: {
		input: SetRecipeTagInput;
		output: SetRecipeTagOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_related_recipe: {
		input: SetRelatedRecipeInput;
		output: SetRelatedRecipeOutput;
		kind: 'immediate';
		permission: 'person';
	};
	create_recipe: {
		input: CreateRecipeInput;
		output: CreateRecipeOutput;
		kind: 'immediate';
		permission: 'person';
	};
	save_recipe_version: {
		input: SaveRecipeVersionInput;
		output: SaveRecipeVersionOutput;
		kind: 'immediate';
		permission: 'person';
	};
	import: {
		input: ImportInput;
		output: ImportOutput;
		kind: 'job';
		permission: 'person';
	};
	import_web_link: {
		input: ImportWebLinkInput;
		output: ImportWebLinkOutput;
		kind: 'job';
		permission: 'person';
	};
	rename_version: {
		input: RenameVersionInput;
		output: RenameVersionOutput;
		kind: 'immediate';
		permission: 'person';
	};
	upload_photograph: {
		input: UploadPhotographInput;
		output: UploadPhotographOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_recipe: {
		input: GetRecipeInput;
		output: GetRecipeOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_reading: {
		input: SetReadingInput;
		output: SetReadingOutput;
		kind: 'immediate';
		permission: 'person';
	};
	start_attempt: {
		input: StartAttemptInput;
		output: StartAttemptOutput;
		kind: 'immediate';
		permission: 'person';
	};
	advance_attempt: {
		input: AdvanceAttemptInput;
		output: AdvanceAttemptOutput;
		kind: 'immediate';
		permission: 'person';
	};
	finish_attempt: {
		input: FinishAttemptInput;
		output: FinishAttemptOutput;
		kind: 'immediate';
		permission: 'person';
	};
	edit_attempt: {
		input: EditAttemptInput;
		output: EditAttemptOutput;
		kind: 'immediate';
		permission: 'person';
	};
	delete_attempt: {
		input: DeleteAttemptInput;
		output: DeleteAttemptOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_current_attempt: {
		input: GetCurrentAttemptInput;
		output: GetCurrentAttemptOutput;
		kind: 'immediate';
		permission: 'person';
	};
	list_foods: {
		input: ListFoodsInput;
		output: ListFoodsOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_food: {
		input: GetFoodInput;
		output: GetFoodOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_food_name: {
		input: SetFoodNameInput;
		output: SetFoodNameOutput;
		kind: 'immediate';
		permission: 'person';
	};
	remove_food_name: {
		input: RemoveFoodNameInput;
		output: RemoveFoodNameOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_food_cup_weight: {
		input: SetFoodCupWeightInput;
		output: SetFoodCupWeightOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_job: {
		input: GetJobInput;
		output: GetJobOutput;
		kind: 'immediate';
		permission: 'public';
	};
	cancel_job: {
		input: CancelJobInput;
		output: CancelJobOutput;
		kind: 'immediate';
		permission: 'public';
	};
	list_jobs: {
		input: ListJobsInput;
		output: ListJobsOutput;
		kind: 'immediate';
		permission: 'person';
	};
}

export type OperationName = keyof Operations;

/** What asking for an Operation answers: a Job answers an id, nothing else does. */
export type Answer<N extends OperationName> =
	Operations[N]['kind'] extends 'job' ? JobAsk : Operations[N]['output'];

/**
 * The declarations themselves, verbatim. The screen-seam harness reads these
 * to answer as the real Operations do; nothing else needs them at runtime.
 */
export const CATALOGUE = [
	{
		"name": "instance_status",
		"summary": "The version of this Kamosu and whether setup has happened.",
		"permission": "public",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"setup_complete": {
					"type": "boolean"
				},
				"version": {
					"type": "string"
				}
			},
			"required": [
				"version",
				"setup_complete"
			],
			"type": "object"
		}
	},
	{
		"name": "set_reading_preferences",
		"summary": "Set the Language and measures this Person reads in.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"reading_language": {
					"enum": [
						"en",
						"fr",
						"es"
					]
				},
				"reading_measures": {
					"enum": [
						"us",
						"metric",
						"as_written"
					]
				}
			},
			"required": [
				"reading_language",
				"reading_measures"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"reading_language": {
					"type": "string"
				},
				"reading_measures": {
					"type": "string"
				}
			},
			"required": [
				"reading_language",
				"reading_measures"
			],
			"type": "object"
		}
	},
	{
		"name": "rename_person",
		"summary": "Change this Person's current reminder name.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": "string"
				}
			},
			"required": [
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": "string"
				}
			},
			"required": [
				"name"
			],
			"type": "object"
		}
	},
	{
		"name": "mint_invite",
		"summary": "Mint a one-use Invite link for a new Person.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"is_operator": {
					"default": false,
					"type": "boolean"
				}
			},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"link": {
					"type": "string"
				}
			},
			"required": [
				"link"
			],
			"type": "object"
		}
	},
	{
		"name": "disable_account",
		"summary": "Disable an account so it can no longer obtain a Credential.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": "string"
				}
			},
			"required": [
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"disabled": {
					"type": "boolean"
				}
			},
			"required": [
				"disabled"
			],
			"type": "object"
		}
	},
	{
		"name": "delete_account",
		"summary": "Delete an account while preserving its Hand in history.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": "string"
				}
			},
			"required": [
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"deleted": {
					"type": "boolean"
				}
			},
			"required": [
				"deleted"
			],
			"type": "object"
		}
	},
	{
		"name": "mint_recovery_link",
		"summary": "Mint a one-use recovery link for a Person who forgot their password.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": "string"
				}
			},
			"required": [
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"link": {
					"type": "string"
				}
			},
			"required": [
				"link"
			],
			"type": "object"
		}
	},
	{
		"name": "list_sessions",
		"summary": "List this Person's browser Sessions by device and last use.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"sessions": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
							"id": {
								"type": "string"
							},
							"last_used_at": {
								"type": [
									"string",
									"null"
								]
							},
							"name": {
								"type": "string"
							},
							"revoked": {
								"type": "boolean"
							}
						},
						"required": [
							"id",
							"name",
							"created_at",
							"last_used_at",
							"revoked"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"sessions"
			],
			"type": "object"
		}
	},
	{
		"name": "revoke_session",
		"summary": "End one of your browser Sessions.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"session_id": {
					"type": "string"
				}
			},
			"required": [
				"session_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"revoked": {
					"type": "boolean"
				}
			},
			"required": [
				"revoked"
			],
			"type": "object"
		}
	},
	{
		"name": "mint_access_key",
		"summary": "Mint an Access Key for an agent to act as you, optionally read-only.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": "string"
				},
				"read_only": {
					"default": false,
					"type": "boolean"
				}
			},
			"required": [
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"id": {
					"type": "string"
				},
				"name": {
					"type": "string"
				},
				"read_only": {
					"type": "boolean"
				},
				"secret": {
					"type": "string"
				}
			},
			"required": [
				"id",
				"name",
				"read_only",
				"secret"
			],
			"type": "object"
		}
	},
	{
		"name": "list_access_keys",
		"summary": "List this Person's Access Keys by name and last use.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"access_keys": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
							"id": {
								"type": "string"
							},
							"last_used_at": {
								"type": [
									"string",
									"null"
								]
							},
							"name": {
								"type": "string"
							},
							"read_only": {
								"type": "boolean"
							},
							"revoked": {
								"type": "boolean"
							}
						},
						"required": [
							"id",
							"name",
							"read_only",
							"created_at",
							"last_used_at",
							"revoked"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"access_keys"
			],
			"type": "object"
		}
	},
	{
		"name": "revoke_access_key",
		"summary": "End one of your Access Keys.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"access_key_id": {
					"type": "string"
				}
			},
			"required": [
				"access_key_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"revoked": {
					"type": "boolean"
				}
			},
			"required": [
				"revoked"
			],
			"type": "object"
		}
	},
	{
		"name": "create_kitchen",
		"summary": "Create a Kitchen: a new circle, held by its creator until they invite someone else in.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": "string"
				}
			},
			"required": [
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"hand_id": {
					"type": "string"
				},
				"id": {
					"type": "string"
				},
				"is_home": {
					"type": "boolean"
				},
				"members": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"name": {
								"type": "string"
							},
							"person_id": {
								"type": "string"
							}
						},
						"required": [
							"person_id",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"name": {
					"type": "string"
				},
				"nickname": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"id",
				"name",
				"hand_id",
				"is_home",
				"nickname",
				"members"
			],
			"type": "object"
		}
	},
	{
		"name": "list_kitchens",
		"summary": "List every Kitchen this Person cooks in.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchens": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"hand_id": {
								"type": "string"
							},
							"id": {
								"type": "string"
							},
							"is_home": {
								"type": "boolean"
							},
							"members": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"name": {
											"type": "string"
										},
										"person_id": {
											"type": "string"
										}
									},
									"required": [
										"person_id",
										"name"
									],
									"type": "object"
								},
								"type": "array"
							},
							"name": {
								"type": "string"
							},
							"nickname": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"name",
							"hand_id",
							"is_home",
							"nickname",
							"members"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"kitchens"
			],
			"type": "object"
		}
	},
	{
		"name": "rename_kitchen",
		"summary": "Change a Kitchen's shared Name. Any member may.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchen_id": {
					"type": "string"
				},
				"name": {
					"type": "string"
				}
			},
			"required": [
				"kitchen_id",
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": "string"
				}
			},
			"required": [
				"name"
			],
			"type": "object"
		}
	},
	{
		"name": "set_kitchen_nickname",
		"summary": "Set this member's own private Nickname for a Kitchen, seen by nobody else. An absent or empty Nickname clears it.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchen_id": {
					"type": "string"
				},
				"nickname": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"kitchen_id",
				"nickname"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"nickname": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"nickname"
			],
			"type": "object"
		}
	},
	{
		"name": "invite_to_kitchen",
		"summary": "Mint a one-use Invite for another Person to join this Kitchen. Any member may.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchen_id": {
					"type": "string"
				}
			},
			"required": [
				"kitchen_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"invite_id": {
					"type": "string"
				},
				"secret": {
					"type": "string"
				}
			},
			"required": [
				"invite_id",
				"secret"
			],
			"type": "object"
		}
	},
	{
		"name": "accept_kitchen_invite",
		"summary": "Open a Kitchen Invite: join the Kitchen it names. Spent on use.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"secret": {
					"type": "string"
				}
			},
			"required": [
				"secret"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"hand_id": {
					"type": "string"
				},
				"id": {
					"type": "string"
				},
				"is_home": {
					"type": "boolean"
				},
				"members": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"name": {
								"type": "string"
							},
							"person_id": {
								"type": "string"
							}
						},
						"required": [
							"person_id",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"name": {
					"type": "string"
				},
				"nickname": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"id",
				"name",
				"hand_id",
				"is_home",
				"nickname",
				"members"
			],
			"type": "object"
		}
	},
	{
		"name": "remove_kitchen_member",
		"summary": "Remove a Person from a Kitchen — including yourself, to leave. The last member cannot be removed.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchen_id": {
					"type": "string"
				},
				"person_id": {
					"type": "string"
				}
			},
			"required": [
				"kitchen_id",
				"person_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"removed": {
					"type": "boolean"
				}
			},
			"required": [
				"removed"
			],
			"type": "object"
		}
	},
	{
		"name": "delete_kitchen",
		"summary": "An Operator's power over a Kitchen: delete one nobody is left in. Nothing else about a Kitchen.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchen_id": {
					"type": "string"
				}
			},
			"required": [
				"kitchen_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"deleted": {
					"type": "boolean"
				}
			},
			"required": [
				"deleted"
			],
			"type": "object"
		}
	},
	{
		"name": "create_tag",
		"summary": "Create a Tag in a Kitchen, named in one Language. A word the Kitchen already files under returns the Tag it already has rather than making a second.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchen_id": {
					"type": "string"
				},
				"language": {
					"enum": [
						"en",
						"fr",
						"es"
					]
				},
				"name": {
					"type": "string"
				}
			},
			"required": [
				"kitchen_id",
				"language",
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"id": {
					"type": "string"
				},
				"kitchen_id": {
					"type": "string"
				},
				"language": {
					"type": [
						"string",
						"null"
					]
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"names": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"language": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"language",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"id",
				"kitchen_id",
				"name",
				"language",
				"names"
			],
			"type": "object"
		}
	},
	{
		"name": "list_tags",
		"summary": "List every Tag a Kitchen files by, each shown in the reader's Reading Language where it has a name there.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchen_id": {
					"type": "string"
				}
			},
			"required": [
				"kitchen_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"tags": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kitchen_id": {
								"type": "string"
							},
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							},
							"names": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"language": {
											"type": "string"
										},
										"name": {
											"type": "string"
										}
									},
									"required": [
										"language",
										"name"
									],
									"type": "object"
								},
								"type": "array"
							}
						},
						"required": [
							"id",
							"kitchen_id",
							"name",
							"language",
							"names"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"tags"
			],
			"type": "object"
		}
	},
	{
		"name": "rename_tag",
		"summary": "Name a Tag in one Language, or change the name it has there. Reaches every recipe carrying it at once, and mints no Version.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"language": {
					"enum": [
						"en",
						"fr",
						"es"
					]
				},
				"name": {
					"type": "string"
				},
				"tag_id": {
					"type": "string"
				}
			},
			"required": [
				"tag_id",
				"language",
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"id": {
					"type": "string"
				},
				"kitchen_id": {
					"type": "string"
				},
				"language": {
					"type": [
						"string",
						"null"
					]
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"names": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"language": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"language",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"id",
				"kitchen_id",
				"name",
				"language",
				"names"
			],
			"type": "object"
		}
	},
	{
		"name": "merge_tags",
		"summary": "Merge two of a Kitchen's Tags into one: every recipe filed under the merged Tag is filed under the kept one instead. Mints no Version.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"keep_tag_id": {
					"type": "string"
				},
				"merge_tag_id": {
					"type": "string"
				}
			},
			"required": [
				"keep_tag_id",
				"merge_tag_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"id": {
					"type": "string"
				},
				"kitchen_id": {
					"type": "string"
				},
				"language": {
					"type": [
						"string",
						"null"
					]
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"names": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"language": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"language",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"id",
				"kitchen_id",
				"name",
				"language",
				"names"
			],
			"type": "object"
		}
	},
	{
		"name": "delete_tag",
		"summary": "Take a Tag out of a Kitchen's list and off every recipe carrying it. No recipe changes.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"tag_id": {
					"type": "string"
				}
			},
			"required": [
				"tag_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"deleted": {
					"type": "boolean"
				}
			},
			"required": [
				"deleted"
			],
			"type": "object"
		}
	},
	{
		"name": "set_recipe_tag",
		"summary": "File a recipe under one of its Kitchen's Tags, or take it back out. Mints no Version: filing is not what a recipe is.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"carried": {
					"type": "boolean"
				},
				"tag_id": {
					"type": "string"
				}
			},
			"required": [
				"branch_id",
				"tag_id",
				"carried"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"tags": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kitchen_id": {
								"type": "string"
							},
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							},
							"names": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"language": {
											"type": "string"
										},
										"name": {
											"type": "string"
										}
									},
									"required": [
										"language",
										"name"
									],
									"type": "object"
								},
								"type": "array"
							}
						},
						"required": [
							"id",
							"kitchen_id",
							"name",
							"language",
							"names"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"tags"
			],
			"type": "object"
		}
	},
	{
		"name": "set_related_recipe",
		"summary": "Relate two Recipes on the same Kitchen shelf, or take that single two-way, untyped link back off. It never changes either Recipe or travels in a Bundle or Share.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"related": {
					"type": "boolean"
				},
				"related_branch_id": {
					"type": "string"
				}
			},
			"required": [
				"branch_id",
				"related_branch_id",
				"related"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"related_recipes": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": [
									"string",
									"null"
								]
							},
							"lineage_id": {
								"type": "string"
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"lineage_id",
							"branch_id",
							"title"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"related_recipes"
			],
			"type": "object"
		}
	},
	{
		"name": "create_recipe",
		"summary": "Create a Recipe: a Lineage, a Branch in this Kitchen, and a first Version. A title is all it needs.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"cook_time_minutes": {
					"description": "Whole minutes of cooking, including resting, proving, marinating and chilling.",
					"type": [
						"integer",
						"null"
					]
				},
				"ingredients": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"kind": {
								"enum": [
									"section",
									"ingredient"
								]
							},
							"text": {
								"type": "string"
							}
						},
						"required": [
							"kind",
							"text"
						],
						"type": "object"
					},
					"type": "array"
				},
				"kitchen_id": {
					"type": "string"
				},
				"language": {
					"enum": [
						"en",
						"fr",
						"es"
					]
				},
				"main_photo": {
					"type": [
						"string",
						"null"
					]
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"prep_time_minutes": {
					"description": "Whole minutes of active preparation.",
					"type": [
						"integer",
						"null"
					]
				},
				"source": {
					"additionalProperties": false,
					"properties": {
						"link": {
							"type": [
								"string",
								"null"
							]
						},
						"text": {
							"type": "string"
						}
					},
					"required": [
						"text",
						"link"
					],
					"type": [
						"object",
						"null"
					]
				},
				"steps": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"kind": {
								"enum": [
									"section",
									"step"
								]
							},
							"photo": {
								"type": [
									"string",
									"null"
								]
							},
							"text": {
								"type": "string"
							}
						},
						"required": [
							"kind",
							"text",
							"photo"
						],
						"type": "object"
					},
					"type": "array"
				},
				"title": {
					"type": "string"
				},
				"yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"type": "string"
						}
					},
					"required": [
						"amount",
						"noun"
					],
					"type": [
						"object",
						"null"
					]
				}
			},
			"required": [
				"kitchen_id",
				"title"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"hand_id": {
					"type": "string"
				},
				"head_version_id": {
					"type": "string"
				},
				"kitchen_id": {
					"type": "string"
				},
				"language": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"origin_address": {
					"type": [
						"string",
						"null"
					]
				},
				"related_recipes": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": [
									"string",
									"null"
								]
							},
							"lineage_id": {
								"type": "string"
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"lineage_id",
							"branch_id",
							"title"
						],
						"type": "object"
					},
					"type": "array"
				},
				"tags": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kitchen_id": {
								"type": "string"
							},
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							},
							"names": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"language": {
											"type": "string"
										},
										"name": {
											"type": "string"
										}
									},
									"required": [
										"language",
										"name"
									],
									"type": "object"
								},
								"type": "array"
							}
						},
						"required": [
							"id",
							"kitchen_id",
							"name",
							"language",
							"names"
						],
						"type": "object"
					},
					"type": "array"
				},
				"versions": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"change_note": {
								"type": [
									"string",
									"null"
								]
							},
							"content": {
								"additionalProperties": false,
								"properties": {
									"cook_time_minutes": {
										"description": "Whole minutes of cooking, including resting, proving, marinating and chilling.",
										"type": [
											"integer",
											"null"
										]
									},
									"ingredients": {
										"items": {
											"additionalProperties": false,
											"properties": {
												"kind": {
													"enum": [
														"section",
														"ingredient"
													]
												},
												"text": {
													"type": "string"
												}
											},
											"required": [
												"kind",
												"text"
											],
											"type": "object"
										},
										"type": "array"
									},
									"main_photo": {
										"type": [
											"string",
											"null"
										]
									},
									"note": {
										"type": [
											"string",
											"null"
										]
									},
									"prep_time_minutes": {
										"description": "Whole minutes of active preparation.",
										"type": [
											"integer",
											"null"
										]
									},
									"source": {
										"additionalProperties": false,
										"properties": {
											"link": {
												"type": [
													"string",
													"null"
												]
											},
											"text": {
												"type": "string"
											}
										},
										"required": [
											"text",
											"link"
										],
										"type": [
											"object",
											"null"
										]
									},
									"steps": {
										"items": {
											"additionalProperties": false,
											"properties": {
												"kind": {
													"enum": [
														"section",
														"step"
													]
												},
												"photo": {
													"type": [
														"string",
														"null"
													]
												},
												"text": {
													"type": "string"
												}
											},
											"required": [
												"kind",
												"text",
												"photo"
											],
											"type": "object"
										},
										"type": "array"
									},
									"title": {
										"type": "string"
									},
									"yield": {
										"additionalProperties": false,
										"properties": {
											"amount": {
												"type": "string"
											},
											"noun": {
												"type": "string"
											}
										},
										"required": [
											"amount",
											"noun"
										],
										"type": [
											"object",
											"null"
										]
									}
								},
								"required": [
									"title",
									"yield",
									"prep_time_minutes",
									"cook_time_minutes",
									"note",
									"main_photo",
									"source",
									"ingredients",
									"steps"
								],
								"type": "object"
							},
							"created_at": {
								"type": "string"
							},
							"hand_id": {
								"type": "string"
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							},
							"parent_version_id": {
								"type": [
									"string",
									"null"
								]
							},
							"readings": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"amount": {
											"type": [
												"string",
												"null"
											]
										},
										"target": {
											"type": [
												"string",
												"null"
											]
										},
										"unit": {
											"type": [
												"string",
												"null"
											]
										}
									},
									"required": [
										"amount",
										"unit",
										"target"
									],
									"type": [
										"object",
										"null"
									]
								},
								"type": "array"
							},
							"sequence": {
								"type": "integer"
							},
							"version_id": {
								"type": "string"
							}
						},
						"required": [
							"sequence",
							"version_id",
							"parent_version_id",
							"hand_id",
							"name",
							"change_note",
							"created_at",
							"content",
							"readings"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"branch_id",
				"lineage_id",
				"kitchen_id",
				"hand_id",
				"language",
				"origin_address",
				"head_version_id",
				"versions",
				"tags",
				"related_recipes"
			],
			"type": "object"
		}
	},
	{
		"name": "save_recipe_version",
		"summary": "Save a new state of a Recipe onto a Branch — the whole recipe as written, replacing what was there. A rapid re-save by the same Hand collapses into the Version already being shaped rather than starting a new one. Changing a recipe your Kitchen did not write is a Copy: it starts a new Branch of the same Lineage, held by your Kitchen, starting at the Version you changed and carrying the whole chain behind it — the Branch you changed is left untouched.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"change_note": {
					"type": "string"
				},
				"cook_time_minutes": {
					"description": "Whole minutes of cooking, including resting, proving, marinating and chilling.",
					"type": [
						"integer",
						"null"
					]
				},
				"ingredients": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"kind": {
								"enum": [
									"section",
									"ingredient"
								]
							},
							"text": {
								"type": "string"
							}
						},
						"required": [
							"kind",
							"text"
						],
						"type": "object"
					},
					"type": "array"
				},
				"kitchen_id": {
					"type": "string"
				},
				"main_photo": {
					"type": [
						"string",
						"null"
					]
				},
				"name": {
					"type": "string"
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"prep_time_minutes": {
					"description": "Whole minutes of active preparation.",
					"type": [
						"integer",
						"null"
					]
				},
				"source": {
					"additionalProperties": false,
					"properties": {
						"link": {
							"type": [
								"string",
								"null"
							]
						},
						"text": {
							"type": "string"
						}
					},
					"required": [
						"text",
						"link"
					],
					"type": [
						"object",
						"null"
					]
				},
				"steps": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"kind": {
								"enum": [
									"section",
									"step"
								]
							},
							"photo": {
								"type": [
									"string",
									"null"
								]
							},
							"text": {
								"type": "string"
							}
						},
						"required": [
							"kind",
							"text",
							"photo"
						],
						"type": "object"
					},
					"type": "array"
				},
				"title": {
					"type": "string"
				},
				"yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"type": "string"
						}
					},
					"required": [
						"amount",
						"noun"
					],
					"type": [
						"object",
						"null"
					]
				}
			},
			"required": [
				"branch_id",
				"title"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"collapsed": {
					"type": "boolean"
				},
				"copied": {
					"description": "True when this save was a Copy: branch_id names the new Branch it started, never the one asked for.",
					"type": "boolean"
				},
				"parent_version_id": {
					"type": [
						"string",
						"null"
					]
				},
				"sequence": {
					"type": "integer"
				},
				"version_id": {
					"type": "string"
				}
			},
			"required": [
				"branch_id",
				"version_id",
				"parent_version_id",
				"sequence",
				"collapsed",
				"copied"
			],
			"type": "object"
		}
	},
	{
		"name": "import",
		"summary": "Bring a batch of already-read recipes into your Home Kitchen, as a Job. Matched by foreign id against this Kitchen's ledger for the source kind, so re-running finds what it already made instead of doubling it; a recipe found changed is offered for review, never written over. Reading the outside source itself — a file, a page, a Bundle — is each importer's own job.",
		"permission": "person",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"candidates": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"cook_time_minutes": {
								"description": "Whole minutes of cooking, including resting, proving, marinating and chilling.",
								"type": [
									"integer",
									"null"
								]
							},
							"foreign_id": {
								"description": "The id this recipe had in the place it came from. Held in this Import's ledger, never on the recipe, so a re-run matches instead of doubling the library (ADR 0025).",
								"type": "string"
							},
							"ingredients": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"kind": {
											"enum": [
												"section",
												"ingredient"
											]
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"kind",
										"text"
									],
									"type": "object"
								},
								"type": "array"
							},
							"language": {
								"enum": [
									"en",
									"fr",
									"es"
								]
							},
							"main_photo": {
								"type": [
									"string",
									"null"
								]
							},
							"note": {
								"type": [
									"string",
									"null"
								]
							},
							"prep_time_minutes": {
								"description": "Whole minutes of active preparation.",
								"type": [
									"integer",
									"null"
								]
							},
							"source": {
								"additionalProperties": false,
								"properties": {
									"link": {
										"type": [
											"string",
											"null"
										]
									},
									"text": {
										"type": "string"
									}
								},
								"required": [
									"text",
									"link"
								],
								"type": [
									"object",
									"null"
								]
							},
							"steps": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"kind": {
											"enum": [
												"section",
												"step"
											]
										},
										"photo": {
											"type": [
												"string",
												"null"
											]
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"kind",
										"text",
										"photo"
									],
									"type": "object"
								},
								"type": "array"
							},
							"title": {
								"type": "string"
							},
							"yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"type": "string"
									}
								},
								"required": [
									"amount",
									"noun"
								],
								"type": [
									"object",
									"null"
								]
							}
						},
						"required": [
							"foreign_id",
							"title"
						],
						"type": "object"
					},
					"type": "array"
				},
				"source_kind": {
					"description": "Which outside source these candidates came from. One ledger is kept per Kitchen per source kind.",
					"type": "string"
				}
			},
			"required": [
				"source_kind",
				"candidates"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"arrived": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"foreign_id": {
								"type": "string"
							},
							"lineage_id": {
								"type": "string"
							},
							"status": {
								"enum": [
									"created",
									"unchanged"
								]
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"foreign_id",
							"status",
							"lineage_id",
							"branch_id",
							"title"
						],
						"type": "object"
					},
					"type": "array"
				},
				"import_id": {
					"type": "string"
				},
				"kitchen_id": {
					"type": "string"
				},
				"offered": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"candidate_version_id": {
								"description": "The Version this candidate's content became, held but not yet on the Branch.",
								"type": "string"
							},
							"foreign_id": {
								"type": "string"
							},
							"lineage_id": {
								"type": "string"
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"foreign_id",
							"lineage_id",
							"branch_id",
							"title",
							"candidate_version_id"
						],
						"type": "object"
					},
					"type": "array"
				},
				"source_kind": {
					"type": "string"
				},
				"unreadable": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"foreign_id": {
								"type": [
									"string",
									"null"
								]
							},
							"reason": {
								"type": "string"
							}
						},
						"required": [
							"foreign_id",
							"reason"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"import_id",
				"kitchen_id",
				"source_kind",
				"arrived",
				"offered",
				"unreadable"
			],
			"type": "object"
		}
	},
	{
		"name": "import_web_link",
		"summary": "Bring in a recipe straight from a URL, as a Job. Reads the page's schema.org JSON-LD (#70) — no per-site scraping, no LLM fallback — and lands it in your Home Kitchen through the same ledger `import` uses, keyed by the page's own address. Fetching is bound to public addresses at the dialled address and at every redirect (ADR 0033), and — because a page's own text can tell an agent to fetch another URL — always takes the single depth-one lane, never more than one fetch in flight regardless of who is signed in.",
		"permission": "person",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"url": {
					"description": "The recipe page's address. Fetched through the guarded client; only http:// and https:// are accepted.",
					"type": "string"
				}
			},
			"required": [
				"url"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"arrived": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"foreign_id": {
								"type": "string"
							},
							"lineage_id": {
								"type": "string"
							},
							"status": {
								"enum": [
									"created",
									"unchanged"
								]
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"foreign_id",
							"status",
							"lineage_id",
							"branch_id",
							"title"
						],
						"type": "object"
					},
					"type": "array"
				},
				"import_id": {
					"type": "string"
				},
				"kitchen_id": {
					"type": "string"
				},
				"offered": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"candidate_version_id": {
								"description": "The Version this candidate's content became, held but not yet on the Branch.",
								"type": "string"
							},
							"foreign_id": {
								"type": "string"
							},
							"lineage_id": {
								"type": "string"
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"foreign_id",
							"lineage_id",
							"branch_id",
							"title",
							"candidate_version_id"
						],
						"type": "object"
					},
					"type": "array"
				},
				"source_kind": {
					"type": "string"
				},
				"unreadable": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"foreign_id": {
								"type": [
									"string",
									"null"
								]
							},
							"reason": {
								"type": "string"
							}
						},
						"required": [
							"foreign_id",
							"reason"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"import_id",
				"kitchen_id",
				"source_kind",
				"arrived",
				"offered",
				"unreadable"
			],
			"type": "object"
		}
	},
	{
		"name": "rename_version",
		"summary": "Rename a Version — the one thing about it that can change later. An absent or empty name clears it. Targeted by the Branch's own sequence number, since the same content can recur more than once on one Branch, each occurrence named on its own.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"sequence": {
					"type": "integer"
				}
			},
			"required": [
				"branch_id",
				"sequence",
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"name"
			],
			"type": "object"
		}
	},
	{
		"name": "upload_photograph",
		"summary": "Upload a Photograph, base64-encoded — the fallback for a Door that cannot carry raw bytes (ADR 0001). A browser uses the out-of-band `POST /api/photographs` instead. Two uploads of the same picture answer the same id.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"data": {
					"description": "The picture, base64-encoded.",
					"type": "string"
				}
			},
			"required": [
				"data"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"photograph_id": {
					"type": "string"
				}
			},
			"required": [
				"photograph_id"
			],
			"type": "object"
		}
	},
	{
		"name": "get_recipe",
		"summary": "Read a Recipe: the Branch as it stands and its whole chain of Versions, oldest first.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				}
			},
			"required": [
				"branch_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"hand_id": {
					"type": "string"
				},
				"head_version_id": {
					"type": "string"
				},
				"kitchen_id": {
					"type": "string"
				},
				"language": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"origin_address": {
					"type": [
						"string",
						"null"
					]
				},
				"related_recipes": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": [
									"string",
									"null"
								]
							},
							"lineage_id": {
								"type": "string"
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"lineage_id",
							"branch_id",
							"title"
						],
						"type": "object"
					},
					"type": "array"
				},
				"tags": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kitchen_id": {
								"type": "string"
							},
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							},
							"names": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"language": {
											"type": "string"
										},
										"name": {
											"type": "string"
										}
									},
									"required": [
										"language",
										"name"
									],
									"type": "object"
								},
								"type": "array"
							}
						},
						"required": [
							"id",
							"kitchen_id",
							"name",
							"language",
							"names"
						],
						"type": "object"
					},
					"type": "array"
				},
				"versions": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"change_note": {
								"type": [
									"string",
									"null"
								]
							},
							"content": {
								"additionalProperties": false,
								"properties": {
									"cook_time_minutes": {
										"description": "Whole minutes of cooking, including resting, proving, marinating and chilling.",
										"type": [
											"integer",
											"null"
										]
									},
									"ingredients": {
										"items": {
											"additionalProperties": false,
											"properties": {
												"kind": {
													"enum": [
														"section",
														"ingredient"
													]
												},
												"text": {
													"type": "string"
												}
											},
											"required": [
												"kind",
												"text"
											],
											"type": "object"
										},
										"type": "array"
									},
									"main_photo": {
										"type": [
											"string",
											"null"
										]
									},
									"note": {
										"type": [
											"string",
											"null"
										]
									},
									"prep_time_minutes": {
										"description": "Whole minutes of active preparation.",
										"type": [
											"integer",
											"null"
										]
									},
									"source": {
										"additionalProperties": false,
										"properties": {
											"link": {
												"type": [
													"string",
													"null"
												]
											},
											"text": {
												"type": "string"
											}
										},
										"required": [
											"text",
											"link"
										],
										"type": [
											"object",
											"null"
										]
									},
									"steps": {
										"items": {
											"additionalProperties": false,
											"properties": {
												"kind": {
													"enum": [
														"section",
														"step"
													]
												},
												"photo": {
													"type": [
														"string",
														"null"
													]
												},
												"text": {
													"type": "string"
												}
											},
											"required": [
												"kind",
												"text",
												"photo"
											],
											"type": "object"
										},
										"type": "array"
									},
									"title": {
										"type": "string"
									},
									"yield": {
										"additionalProperties": false,
										"properties": {
											"amount": {
												"type": "string"
											},
											"noun": {
												"type": "string"
											}
										},
										"required": [
											"amount",
											"noun"
										],
										"type": [
											"object",
											"null"
										]
									}
								},
								"required": [
									"title",
									"yield",
									"prep_time_minutes",
									"cook_time_minutes",
									"note",
									"main_photo",
									"source",
									"ingredients",
									"steps"
								],
								"type": "object"
							},
							"created_at": {
								"type": "string"
							},
							"hand_id": {
								"type": "string"
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							},
							"parent_version_id": {
								"type": [
									"string",
									"null"
								]
							},
							"readings": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"amount": {
											"type": [
												"string",
												"null"
											]
										},
										"target": {
											"type": [
												"string",
												"null"
											]
										},
										"unit": {
											"type": [
												"string",
												"null"
											]
										}
									},
									"required": [
										"amount",
										"unit",
										"target"
									],
									"type": [
										"object",
										"null"
									]
								},
								"type": "array"
							},
							"sequence": {
								"type": "integer"
							},
							"version_id": {
								"type": "string"
							}
						},
						"required": [
							"sequence",
							"version_id",
							"parent_version_id",
							"hand_id",
							"name",
							"change_note",
							"created_at",
							"content",
							"readings"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"branch_id",
				"lineage_id",
				"kitchen_id",
				"hand_id",
				"language",
				"origin_address",
				"head_version_id",
				"versions",
				"tags",
				"related_recipes"
			],
			"type": "object"
		}
	},
	{
		"name": "set_reading",
		"summary": "Correct the Reading on one Ingredient Line of a Recipe's current state — an amount, a Unit and a target, sent together as the whole new Reading (never a per-field patch, the same convention save_recipe_version uses for the whole recipe). Mints no Version and appears in no history (ADR 0021). Amount, Unit and target left out together clears the Reading, taking the line back to fully unread.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"amount": {
					"type": [
						"string",
						"null"
					]
				},
				"branch_id": {
					"type": "string"
				},
				"line_index": {
					"minimum": 0,
					"type": "integer"
				},
				"target": {
					"type": [
						"string",
						"null"
					]
				},
				"unit": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"branch_id",
				"line_index"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"line_index": {
					"type": "integer"
				},
				"reading": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": [
								"string",
								"null"
							]
						},
						"target": {
							"type": [
								"string",
								"null"
							]
						},
						"unit": {
							"type": [
								"string",
								"null"
							]
						}
					},
					"required": [
						"amount",
						"unit",
						"target"
					],
					"type": [
						"object",
						"null"
					]
				}
			},
			"required": [
				"line_index",
				"reading"
			],
			"type": "object"
		}
	},
	{
		"name": "start_attempt",
		"summary": "Start cooking a Recipe: creates the Attempt, or hands back the one already In Progress for this Lineage — the cooking screen is that Attempt, never a second thing beside it. Pinned by fingerprint to the Branch's head Version at this moment. Anyone who can see the recipe may.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				}
			},
			"required": [
				"branch_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"type": "string"
						}
					},
					"required": [
						"amount",
						"noun"
					],
					"type": [
						"object",
						"null"
					]
				},
				"created_at": {
					"type": "string"
				},
				"current_step_index": {
					"minimum": 0,
					"type": "integer"
				},
				"finished_at": {
					"type": [
						"string",
						"null"
					]
				},
				"id": {
					"type": "string"
				},
				"last_action_at": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"person_id": {
					"type": "string"
				},
				"rating": {
					"maximum": 5,
					"minimum": 1,
					"type": [
						"integer",
						"null"
					]
				},
				"resumable": {
					"type": "boolean"
				},
				"ticked_ingredients": {
					"items": {
						"minimum": 0,
						"type": "integer"
					},
					"type": "array"
				},
				"version_id": {
					"type": "string"
				}
			},
			"required": [
				"id",
				"lineage_id",
				"person_id",
				"version_id",
				"current_step_index",
				"ticked_ingredients",
				"cooking_yield",
				"note",
				"rating",
				"finished_at",
				"resumable",
				"created_at",
				"last_action_at"
			],
			"type": "object"
		}
	},
	{
		"name": "advance_attempt",
		"summary": "Move an In Progress Attempt forward: which Step, which Ingredients are ticked, and the Yield being cooked to — a fact about this cooking, never a deviation. Any of the three, each sent whole rather than patched.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"attempt_id": {
					"type": "string"
				},
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"type": "string"
						}
					},
					"required": [
						"amount",
						"noun"
					],
					"type": [
						"object",
						"null"
					]
				},
				"current_step_index": {
					"minimum": 0,
					"type": "integer"
				},
				"ticked_ingredients": {
					"items": {
						"minimum": 0,
						"type": "integer"
					},
					"type": "array"
				}
			},
			"required": [
				"attempt_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"type": "string"
						}
					},
					"required": [
						"amount",
						"noun"
					],
					"type": [
						"object",
						"null"
					]
				},
				"created_at": {
					"type": "string"
				},
				"current_step_index": {
					"minimum": 0,
					"type": "integer"
				},
				"finished_at": {
					"type": [
						"string",
						"null"
					]
				},
				"id": {
					"type": "string"
				},
				"last_action_at": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"person_id": {
					"type": "string"
				},
				"rating": {
					"maximum": 5,
					"minimum": 1,
					"type": [
						"integer",
						"null"
					]
				},
				"resumable": {
					"type": "boolean"
				},
				"ticked_ingredients": {
					"items": {
						"minimum": 0,
						"type": "integer"
					},
					"type": "array"
				},
				"version_id": {
					"type": "string"
				}
			},
			"required": [
				"id",
				"lineage_id",
				"person_id",
				"version_id",
				"current_step_index",
				"ticked_ingredients",
				"cooking_yield",
				"note",
				"rating",
				"finished_at",
				"resumable",
				"created_at",
				"last_action_at"
			],
			"type": "object"
		}
	},
	{
		"name": "finish_attempt",
		"summary": "End an In Progress Attempt. Ending is not what makes the cooking real — starting already did — only what stops it being In Progress.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"attempt_id": {
					"type": "string"
				}
			},
			"required": [
				"attempt_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"type": "string"
						}
					},
					"required": [
						"amount",
						"noun"
					],
					"type": [
						"object",
						"null"
					]
				},
				"created_at": {
					"type": "string"
				},
				"current_step_index": {
					"minimum": 0,
					"type": "integer"
				},
				"finished_at": {
					"type": [
						"string",
						"null"
					]
				},
				"id": {
					"type": "string"
				},
				"last_action_at": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"person_id": {
					"type": "string"
				},
				"rating": {
					"maximum": 5,
					"minimum": 1,
					"type": [
						"integer",
						"null"
					]
				},
				"resumable": {
					"type": "boolean"
				},
				"ticked_ingredients": {
					"items": {
						"minimum": 0,
						"type": "integer"
					},
					"type": "array"
				},
				"version_id": {
					"type": "string"
				}
			},
			"required": [
				"id",
				"lineage_id",
				"person_id",
				"version_id",
				"current_step_index",
				"ticked_ingredients",
				"cooking_yield",
				"note",
				"rating",
				"finished_at",
				"resumable",
				"created_at",
				"last_action_at"
			],
			"type": "object"
		}
	},
	{
		"name": "edit_attempt",
		"summary": "Change an Attempt's free text or its five-star rating, whether it is still In Progress or long finished — an Attempt is freely editable by its cook, unlike the recipe it was cooked from.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"attempt_id": {
					"type": "string"
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"rating": {
					"maximum": 5,
					"minimum": 1,
					"type": [
						"integer",
						"null"
					]
				}
			},
			"required": [
				"attempt_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"type": "string"
						}
					},
					"required": [
						"amount",
						"noun"
					],
					"type": [
						"object",
						"null"
					]
				},
				"created_at": {
					"type": "string"
				},
				"current_step_index": {
					"minimum": 0,
					"type": "integer"
				},
				"finished_at": {
					"type": [
						"string",
						"null"
					]
				},
				"id": {
					"type": "string"
				},
				"last_action_at": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"person_id": {
					"type": "string"
				},
				"rating": {
					"maximum": 5,
					"minimum": 1,
					"type": [
						"integer",
						"null"
					]
				},
				"resumable": {
					"type": "boolean"
				},
				"ticked_ingredients": {
					"items": {
						"minimum": 0,
						"type": "integer"
					},
					"type": "array"
				},
				"version_id": {
					"type": "string"
				}
			},
			"required": [
				"id",
				"lineage_id",
				"person_id",
				"version_id",
				"current_step_index",
				"ticked_ingredients",
				"cooking_yield",
				"note",
				"rating",
				"finished_at",
				"resumable",
				"created_at",
				"last_action_at"
			],
			"type": "object"
		}
	},
	{
		"name": "delete_attempt",
		"summary": "Delete an Attempt outright — the explicit way a false start is undone, or any cooking record put away. Never soft-deleted: this is the whole of how an Attempt leaves.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"attempt_id": {
					"type": "string"
				}
			},
			"required": [
				"attempt_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"deleted": {
					"type": "boolean"
				}
			},
			"required": [
				"deleted"
			],
			"type": "object"
		}
	},
	{
		"name": "get_current_attempt",
		"summary": "Read the caller's own In Progress Attempt for a Lineage, if any — how two devices cooking the same dish stay in step, and whether resuming should still be offered.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"lineage_id": {
					"type": "string"
				}
			},
			"required": [
				"lineage_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"attempt": {
					"additionalProperties": false,
					"properties": {
						"cooking_yield": {
							"additionalProperties": false,
							"properties": {
								"amount": {
									"type": "string"
								},
								"noun": {
									"type": "string"
								}
							},
							"required": [
								"amount",
								"noun"
							],
							"type": [
								"object",
								"null"
							]
						},
						"created_at": {
							"type": "string"
						},
						"current_step_index": {
							"minimum": 0,
							"type": "integer"
						},
						"finished_at": {
							"type": [
								"string",
								"null"
							]
						},
						"id": {
							"type": "string"
						},
						"last_action_at": {
							"type": "string"
						},
						"lineage_id": {
							"type": "string"
						},
						"note": {
							"type": [
								"string",
								"null"
							]
						},
						"person_id": {
							"type": "string"
						},
						"rating": {
							"maximum": 5,
							"minimum": 1,
							"type": [
								"integer",
								"null"
							]
						},
						"resumable": {
							"type": "boolean"
						},
						"ticked_ingredients": {
							"items": {
								"minimum": 0,
								"type": "integer"
							},
							"type": "array"
						},
						"version_id": {
							"type": "string"
						}
					},
					"required": [
						"id",
						"lineage_id",
						"person_id",
						"version_id",
						"current_step_index",
						"ticked_ingredients",
						"cooking_yield",
						"note",
						"rating",
						"finished_at",
						"resumable",
						"created_at",
						"last_action_at"
					],
					"type": [
						"object",
						"null"
					]
				}
			},
			"required": [
				"attempt"
			],
			"type": "object"
		}
	},
	{
		"name": "list_foods",
		"summary": "List every Food this instance knows, each shown in the reader's Reading Language where it has a name there.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"foods": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"cup_weight_grams": {
								"type": [
									"number",
									"null"
								]
							},
							"id": {
								"type": "string"
							},
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							},
							"names": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"language": {
											"type": "string"
										},
										"name": {
											"type": "string"
										}
									},
									"required": [
										"language",
										"name"
									],
									"type": "object"
								},
								"type": "array"
							},
							"nutrition": {
								"type": "null"
							},
							"reading_count": {
								"minimum": 0,
								"type": "integer"
							}
						},
						"required": [
							"id",
							"name",
							"language",
							"names",
							"cup_weight_grams",
							"nutrition",
							"reading_count"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"foods"
			],
			"type": "object"
		}
	},
	{
		"name": "get_food",
		"summary": "Read one Food: its names, its Cup Weight, and how many Readings currently point at it.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"food_id": {
					"type": "string"
				}
			},
			"required": [
				"food_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cup_weight_grams": {
					"type": [
						"number",
						"null"
					]
				},
				"id": {
					"type": "string"
				},
				"language": {
					"type": [
						"string",
						"null"
					]
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"names": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"language": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"language",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"nutrition": {
					"type": "null"
				},
				"reading_count": {
					"minimum": 0,
					"type": "integer"
				}
			},
			"required": [
				"id",
				"name",
				"language",
				"names",
				"cup_weight_grams",
				"nutrition",
				"reading_count"
			],
			"type": "object"
		}
	},
	{
		"name": "set_food_name",
		"summary": "Give a Food its name in one Language, or correct the one it has there. Any Person may — a Food is instance-wide, not a Kitchen's to guard.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"food_id": {
					"type": "string"
				},
				"language": {
					"enum": [
						"en",
						"fr",
						"es"
					]
				},
				"name": {
					"type": "string"
				}
			},
			"required": [
				"food_id",
				"language",
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cup_weight_grams": {
					"type": [
						"number",
						"null"
					]
				},
				"id": {
					"type": "string"
				},
				"language": {
					"type": [
						"string",
						"null"
					]
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"names": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"language": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"language",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"nutrition": {
					"type": "null"
				},
				"reading_count": {
					"minimum": 0,
					"type": "integer"
				}
			},
			"required": [
				"id",
				"name",
				"language",
				"names",
				"cup_weight_grams",
				"nutrition",
				"reading_count"
			],
			"type": "object"
		}
	},
	{
		"name": "remove_food_name",
		"summary": "Take a Food's name in one Language back off. A Food's last remaining name may not be removed this way.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"food_id": {
					"type": "string"
				},
				"language": {
					"enum": [
						"en",
						"fr",
						"es"
					]
				}
			},
			"required": [
				"food_id",
				"language"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cup_weight_grams": {
					"type": [
						"number",
						"null"
					]
				},
				"id": {
					"type": "string"
				},
				"language": {
					"type": [
						"string",
						"null"
					]
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"names": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"language": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"language",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"nutrition": {
					"type": "null"
				},
				"reading_count": {
					"minimum": 0,
					"type": "integer"
				}
			},
			"required": [
				"id",
				"name",
				"language",
				"names",
				"cup_weight_grams",
				"nutrition",
				"reading_count"
			],
			"type": "object"
		}
	},
	{
		"name": "set_food_cup_weight",
		"summary": "Set or clear a Food's Cup Weight — the one figure that turns a volume of it into a weight. Anyone may correct it.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"cup_weight_grams": {
					"exclusiveMinimum": 0,
					"type": [
						"number",
						"null"
					]
				},
				"food_id": {
					"type": "string"
				}
			},
			"required": [
				"food_id",
				"cup_weight_grams"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cup_weight_grams": {
					"type": [
						"number",
						"null"
					]
				},
				"id": {
					"type": "string"
				},
				"language": {
					"type": [
						"string",
						"null"
					]
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"names": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"language": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"language",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"nutrition": {
					"type": "null"
				},
				"reading_count": {
					"minimum": 0,
					"type": "integer"
				}
			},
			"required": [
				"id",
				"name",
				"language",
				"names",
				"cup_weight_grams",
				"nutrition",
				"reading_count"
			],
			"type": "object"
		}
	},
	{
		"name": "get_job",
		"summary": "Read one Job: its state, its progress, and its result or the reason it failed. Readable by the Person who asked, or by anyone when no Person did.",
		"permission": "public",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"job_id": {
					"type": "string"
				}
			},
			"required": [
				"job_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"created_at": {
					"type": "string"
				},
				"error": {
					"type": [
						"string",
						"null"
					]
				},
				"errorCode": {
					"type": [
						"integer",
						"null"
					]
				},
				"id": {
					"type": "string"
				},
				"operation": {
					"type": "string"
				},
				"progress": {
					"additionalProperties": false,
					"properties": {
						"done": {
							"type": [
								"integer",
								"null"
							]
						},
						"message": {
							"type": [
								"string",
								"null"
							]
						},
						"total": {
							"type": [
								"integer",
								"null"
							]
						}
					},
					"type": "object"
				},
				"result": {},
				"status": {
					"enum": [
						"queued",
						"running",
						"completed",
						"failed",
						"cancelled"
					]
				},
				"updated_at": {
					"type": "string"
				}
			},
			"required": [
				"id",
				"operation",
				"status",
				"progress",
				"result",
				"error",
				"errorCode",
				"created_at",
				"updated_at"
			],
			"type": "object"
		}
	},
	{
		"name": "cancel_job",
		"summary": "Cancel a Job you asked for: acknowledged always, honoured while it still waits in line.",
		"permission": "public",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"job_id": {
					"type": "string"
				}
			},
			"required": [
				"job_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cancelled": {
					"type": "boolean"
				}
			},
			"required": [
				"cancelled"
			],
			"type": "object"
		}
	},
	{
		"name": "list_jobs",
		"summary": "List the Jobs this Person has asked for, newest first.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"jobs": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
							"id": {
								"type": "string"
							},
							"operation": {
								"type": "string"
							},
							"status": {
								"enum": [
									"queued",
									"running",
									"completed",
									"failed",
									"cancelled"
								]
							}
						},
						"required": [
							"id",
							"operation",
							"status",
							"created_at"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"jobs"
			],
			"type": "object"
		}
	},
] as const;

/** The camelCase method the client exposes for each Operation. */
export const METHOD_NAMES = {
	instance_status: 'instanceStatus',
	set_reading_preferences: 'setReadingPreferences',
	rename_person: 'renamePerson',
	mint_invite: 'mintInvite',
	disable_account: 'disableAccount',
	delete_account: 'deleteAccount',
	mint_recovery_link: 'mintRecoveryLink',
	list_sessions: 'listSessions',
	revoke_session: 'revokeSession',
	mint_access_key: 'mintAccessKey',
	list_access_keys: 'listAccessKeys',
	revoke_access_key: 'revokeAccessKey',
	create_kitchen: 'createKitchen',
	list_kitchens: 'listKitchens',
	rename_kitchen: 'renameKitchen',
	set_kitchen_nickname: 'setKitchenNickname',
	invite_to_kitchen: 'inviteToKitchen',
	accept_kitchen_invite: 'acceptKitchenInvite',
	remove_kitchen_member: 'removeKitchenMember',
	delete_kitchen: 'deleteKitchen',
	create_tag: 'createTag',
	list_tags: 'listTags',
	rename_tag: 'renameTag',
	merge_tags: 'mergeTags',
	delete_tag: 'deleteTag',
	set_recipe_tag: 'setRecipeTag',
	set_related_recipe: 'setRelatedRecipe',
	create_recipe: 'createRecipe',
	save_recipe_version: 'saveRecipeVersion',
	import: 'import',
	import_web_link: 'importWebLink',
	rename_version: 'renameVersion',
	upload_photograph: 'uploadPhotograph',
	get_recipe: 'getRecipe',
	set_reading: 'setReading',
	start_attempt: 'startAttempt',
	advance_attempt: 'advanceAttempt',
	finish_attempt: 'finishAttempt',
	edit_attempt: 'editAttempt',
	delete_attempt: 'deleteAttempt',
	get_current_attempt: 'getCurrentAttempt',
	list_foods: 'listFoods',
	get_food: 'getFood',
	set_food_name: 'setFoodName',
	remove_food_name: 'removeFoodName',
	set_food_cup_weight: 'setFoodCupWeight',
	get_job: 'getJob',
	cancel_job: 'cancelJob',
	list_jobs: 'listJobs',
} as const;

/** The typed client: one method per Operation, named as the Catalogue names it. */
export interface KamosuClient {
	/** The version of this Kamosu and whether setup has happened. */
	instanceStatus(input?: InstanceStatusInput): Promise<Answer<'instance_status'>>;
	/** Set the Language and measures this Person reads in. */
	setReadingPreferences(input: SetReadingPreferencesInput): Promise<Answer<'set_reading_preferences'>>;
	/** Change this Person's current reminder name. */
	renamePerson(input: RenamePersonInput): Promise<Answer<'rename_person'>>;
	/** Mint a one-use Invite link for a new Person. */
	mintInvite(input: MintInviteInput): Promise<Answer<'mint_invite'>>;
	/** Disable an account so it can no longer obtain a Credential. */
	disableAccount(input: DisableAccountInput): Promise<Answer<'disable_account'>>;
	/** Delete an account while preserving its Hand in history. */
	deleteAccount(input: DeleteAccountInput): Promise<Answer<'delete_account'>>;
	/** Mint a one-use recovery link for a Person who forgot their password. */
	mintRecoveryLink(input: MintRecoveryLinkInput): Promise<Answer<'mint_recovery_link'>>;
	/** List this Person's browser Sessions by device and last use. */
	listSessions(input?: ListSessionsInput): Promise<Answer<'list_sessions'>>;
	/** End one of your browser Sessions. */
	revokeSession(input: RevokeSessionInput): Promise<Answer<'revoke_session'>>;
	/** Mint an Access Key for an agent to act as you, optionally read-only. */
	mintAccessKey(input: MintAccessKeyInput): Promise<Answer<'mint_access_key'>>;
	/** List this Person's Access Keys by name and last use. */
	listAccessKeys(input?: ListAccessKeysInput): Promise<Answer<'list_access_keys'>>;
	/** End one of your Access Keys. */
	revokeAccessKey(input: RevokeAccessKeyInput): Promise<Answer<'revoke_access_key'>>;
	/** Create a Kitchen: a new circle, held by its creator until they invite someone else in. */
	createKitchen(input: CreateKitchenInput): Promise<Answer<'create_kitchen'>>;
	/** List every Kitchen this Person cooks in. */
	listKitchens(input?: ListKitchensInput): Promise<Answer<'list_kitchens'>>;
	/** Change a Kitchen's shared Name. Any member may. */
	renameKitchen(input: RenameKitchenInput): Promise<Answer<'rename_kitchen'>>;
	/** Set this member's own private Nickname for a Kitchen, seen by nobody else. An absent or empty Nickname clears it. */
	setKitchenNickname(input: SetKitchenNicknameInput): Promise<Answer<'set_kitchen_nickname'>>;
	/** Mint a one-use Invite for another Person to join this Kitchen. Any member may. */
	inviteToKitchen(input: InviteToKitchenInput): Promise<Answer<'invite_to_kitchen'>>;
	/** Open a Kitchen Invite: join the Kitchen it names. Spent on use. */
	acceptKitchenInvite(input: AcceptKitchenInviteInput): Promise<Answer<'accept_kitchen_invite'>>;
	/** Remove a Person from a Kitchen — including yourself, to leave. The last member cannot be removed. */
	removeKitchenMember(input: RemoveKitchenMemberInput): Promise<Answer<'remove_kitchen_member'>>;
	/** An Operator's power over a Kitchen: delete one nobody is left in. Nothing else about a Kitchen. */
	deleteKitchen(input: DeleteKitchenInput): Promise<Answer<'delete_kitchen'>>;
	/** Create a Tag in a Kitchen, named in one Language. A word the Kitchen already files under returns the Tag it already has rather than making a second. */
	createTag(input: CreateTagInput): Promise<Answer<'create_tag'>>;
	/** List every Tag a Kitchen files by, each shown in the reader's Reading Language where it has a name there. */
	listTags(input: ListTagsInput): Promise<Answer<'list_tags'>>;
	/** Name a Tag in one Language, or change the name it has there. Reaches every recipe carrying it at once, and mints no Version. */
	renameTag(input: RenameTagInput): Promise<Answer<'rename_tag'>>;
	/** Merge two of a Kitchen's Tags into one: every recipe filed under the merged Tag is filed under the kept one instead. Mints no Version. */
	mergeTags(input: MergeTagsInput): Promise<Answer<'merge_tags'>>;
	/** Take a Tag out of a Kitchen's list and off every recipe carrying it. No recipe changes. */
	deleteTag(input: DeleteTagInput): Promise<Answer<'delete_tag'>>;
	/** File a recipe under one of its Kitchen's Tags, or take it back out. Mints no Version: filing is not what a recipe is. */
	setRecipeTag(input: SetRecipeTagInput): Promise<Answer<'set_recipe_tag'>>;
	/** Relate two Recipes on the same Kitchen shelf, or take that single two-way, untyped link back off. It never changes either Recipe or travels in a Bundle or Share. */
	setRelatedRecipe(input: SetRelatedRecipeInput): Promise<Answer<'set_related_recipe'>>;
	/** Create a Recipe: a Lineage, a Branch in this Kitchen, and a first Version. A title is all it needs. */
	createRecipe(input: CreateRecipeInput): Promise<Answer<'create_recipe'>>;
	/** Save a new state of a Recipe onto a Branch — the whole recipe as written, replacing what was there. A rapid re-save by the same Hand collapses into the Version already being shaped rather than starting a new one. Changing a recipe your Kitchen did not write is a Copy: it starts a new Branch of the same Lineage, held by your Kitchen, starting at the Version you changed and carrying the whole chain behind it — the Branch you changed is left untouched. */
	saveRecipeVersion(input: SaveRecipeVersionInput): Promise<Answer<'save_recipe_version'>>;
	/** Bring a batch of already-read recipes into your Home Kitchen, as a Job. Matched by foreign id against this Kitchen's ledger for the source kind, so re-running finds what it already made instead of doubling it; a recipe found changed is offered for review, never written over. Reading the outside source itself — a file, a page, a Bundle — is each importer's own job. */
	import(input: ImportInput): Promise<Answer<'import'>>;
	/** Bring in a recipe straight from a URL, as a Job. Reads the page's schema.org JSON-LD (#70) — no per-site scraping, no LLM fallback — and lands it in your Home Kitchen through the same ledger `import` uses, keyed by the page's own address. Fetching is bound to public addresses at the dialled address and at every redirect (ADR 0033), and — because a page's own text can tell an agent to fetch another URL — always takes the single depth-one lane, never more than one fetch in flight regardless of who is signed in. */
	importWebLink(input: ImportWebLinkInput): Promise<Answer<'import_web_link'>>;
	/** Rename a Version — the one thing about it that can change later. An absent or empty name clears it. Targeted by the Branch's own sequence number, since the same content can recur more than once on one Branch, each occurrence named on its own. */
	renameVersion(input: RenameVersionInput): Promise<Answer<'rename_version'>>;
	/** Upload a Photograph, base64-encoded — the fallback for a Door that cannot carry raw bytes (ADR 0001). A browser uses the out-of-band `POST /api/photographs` instead. Two uploads of the same picture answer the same id. */
	uploadPhotograph(input: UploadPhotographInput): Promise<Answer<'upload_photograph'>>;
	/** Read a Recipe: the Branch as it stands and its whole chain of Versions, oldest first. */
	getRecipe(input: GetRecipeInput): Promise<Answer<'get_recipe'>>;
	/** Correct the Reading on one Ingredient Line of a Recipe's current state — an amount, a Unit and a target, sent together as the whole new Reading (never a per-field patch, the same convention save_recipe_version uses for the whole recipe). Mints no Version and appears in no history (ADR 0021). Amount, Unit and target left out together clears the Reading, taking the line back to fully unread. */
	setReading(input: SetReadingInput): Promise<Answer<'set_reading'>>;
	/** Start cooking a Recipe: creates the Attempt, or hands back the one already In Progress for this Lineage — the cooking screen is that Attempt, never a second thing beside it. Pinned by fingerprint to the Branch's head Version at this moment. Anyone who can see the recipe may. */
	startAttempt(input: StartAttemptInput): Promise<Answer<'start_attempt'>>;
	/** Move an In Progress Attempt forward: which Step, which Ingredients are ticked, and the Yield being cooked to — a fact about this cooking, never a deviation. Any of the three, each sent whole rather than patched. */
	advanceAttempt(input: AdvanceAttemptInput): Promise<Answer<'advance_attempt'>>;
	/** End an In Progress Attempt. Ending is not what makes the cooking real — starting already did — only what stops it being In Progress. */
	finishAttempt(input: FinishAttemptInput): Promise<Answer<'finish_attempt'>>;
	/** Change an Attempt's free text or its five-star rating, whether it is still In Progress or long finished — an Attempt is freely editable by its cook, unlike the recipe it was cooked from. */
	editAttempt(input: EditAttemptInput): Promise<Answer<'edit_attempt'>>;
	/** Delete an Attempt outright — the explicit way a false start is undone, or any cooking record put away. Never soft-deleted: this is the whole of how an Attempt leaves. */
	deleteAttempt(input: DeleteAttemptInput): Promise<Answer<'delete_attempt'>>;
	/** Read the caller's own In Progress Attempt for a Lineage, if any — how two devices cooking the same dish stay in step, and whether resuming should still be offered. */
	getCurrentAttempt(input: GetCurrentAttemptInput): Promise<Answer<'get_current_attempt'>>;
	/** List every Food this instance knows, each shown in the reader's Reading Language where it has a name there. */
	listFoods(input?: ListFoodsInput): Promise<Answer<'list_foods'>>;
	/** Read one Food: its names, its Cup Weight, and how many Readings currently point at it. */
	getFood(input: GetFoodInput): Promise<Answer<'get_food'>>;
	/** Give a Food its name in one Language, or correct the one it has there. Any Person may — a Food is instance-wide, not a Kitchen's to guard. */
	setFoodName(input: SetFoodNameInput): Promise<Answer<'set_food_name'>>;
	/** Take a Food's name in one Language back off. A Food's last remaining name may not be removed this way. */
	removeFoodName(input: RemoveFoodNameInput): Promise<Answer<'remove_food_name'>>;
	/** Set or clear a Food's Cup Weight — the one figure that turns a volume of it into a weight. Anyone may correct it. */
	setFoodCupWeight(input: SetFoodCupWeightInput): Promise<Answer<'set_food_cup_weight'>>;
	/** Read one Job: its state, its progress, and its result or the reason it failed. Readable by the Person who asked, or by anyone when no Person did. */
	getJob(input: GetJobInput): Promise<Answer<'get_job'>>;
	/** Cancel a Job you asked for: acknowledged always, honoured while it still waits in line. */
	cancelJob(input: CancelJobInput): Promise<Answer<'cancel_job'>>;
	/** List the Jobs this Person has asked for, newest first. */
	listJobs(input?: ListJobsInput): Promise<Answer<'list_jobs'>>;
}
