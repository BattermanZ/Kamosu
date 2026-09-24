// Generated from the Catalogue by `just client`. Do not edit.
//
// Every Operation Kamosu offers is declared once in src/catalogue.rs and both
// Doors are built by walking that list (ADR 0001). These types are the third
// thing built from it, so the interface cannot ask for a shape the Core does
// not serve. Change the Catalogue and re-run `just client`; `just check`
// fails if what is committed here has drifted.

import type { JobAsk } from './job';

/** The version of this Kamosu, whether setup has happened, and the shortest password it accepts. */
export type InstanceStatusInput = Record<string, never>;
/** What instance_status answers. */
export type InstanceStatusOutput = {
	password_minimum: number;
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
	reading_language: "en" | "fr" | "es";
	reading_measures: "us" | "metric" | "as_written";
};

/** The Language and measures this Person reads in. Reading Measures live on the account rather than in a browser, so every Door and every device reads the same recipe the same way; the default is American, a stated convention rather than a guess about anybody. */
export type GetReadingPreferencesInput = Record<string, never>;
/** What get_reading_preferences answers. */
export type GetReadingPreferencesOutput = {
	reading_language: "en" | "fr" | "es";
	reading_measures: "us" | "metric" | "as_written";
};

/** Who this Credential names: the Person's permanent id, which is also their Hand, and the name they currently go by — the name every Version they wrote shows here, and the one they sign in with. */
export type GetPersonInput = Record<string, never>;
/** What get_person answers. */
export type GetPersonOutput = {
	name: string;
	person_id: string;
};

/** Change this Person's current reminder name. Every Version they ever wrote shows the new one on this instance, since a Hand is named live and nothing is keyed on the name; no id or fingerprint moves. It is also the name they sign in with, so a name somebody else here signs in with is refused. A Bundle already sent keeps the name it left with. */
export type RenamePersonInput = {
	name: string;
};
/** What rename_person answers. */
export type RenamePersonOutput = {
	name: string;
};

/** Who holds an account on this instance: their name, whether they administer it, and whether the account is disabled. Nothing about what they cook — the Operator administers and does not read (ADR 0007), so no recipe, Attempt, Cookbook or Kitchen of theirs is reachable from here. */
export type ListAccountsInput = Record<string, never>;
/** What list_accounts answers. */
export type ListAccountsOutput = {
	accounts: {
		created_at: string;
		disabled: boolean;
		is_operator: boolean;
		is_you: boolean;
		name: string;
	}[];
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

/** Delete an account while preserving its Hand in history. The Person's name is freed for somebody new to sign in with; what they wrote keeps their Hand and the name they had. Disabling an account keeps the name. */
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

/** Make a Person an Operator, or stop them being one. The last Operator cannot be demoted (ADR 0007): an instance with nobody to administer it can never get one back, so the refusal is the point rather than a nicety. */
export type SetOperatorInput = {
	is_operator: boolean;
	name: string;
};
/** What set_operator answers. */
export type SetOperatorOutput = {
	is_operator: boolean;
	name: string;
};

/** Take away Photographs nothing has pointed at for a week, and their Display Copies with them. Runs daily on its own; this asks for it now. */
export type SweepPhotographsInput = Record<string, never>;
/** What sweep_photographs answers. */
export type SweepPhotographsOutput = {
	back_in_use: number;
	newly_unreferenced: number;
	referenced: number;
	swept: number;
	swept_photograph_ids: string[];
};

/** Take a Backup now, as a Job: one archive holding a consistent copy of the database and every Photograph, written beside the database under /data. Kamosu keeps three — one taken daily, one weekly, one monthly — and takes them on its own; this asks for one now. A Job because an archive is the size of the library. It is never sent anywhere: fetch the bytes at GET /api/backups/<name>. */
export type TakeBackupInput = Record<string, never>;
/** What take_backup eventually produces, read back through `get_job`. */
export type TakeBackupOutput = {
	backups: {
		name: string;
		size_bytes: number;
		slot: "daily" | "weekly" | "monthly";
		taken_at: string;
	}[];
	taken: string[];
};

/** The Backups this instance holds, newest first. Each can be fetched at GET /api/backups/<name>, under the same Credential as any Operation. */
export type ListBackupsInput = Record<string, never>;
/** What list_backups answers. */
export type ListBackupsOutput = {
	backups: {
		name: string;
		size_bytes: number;
		slot: "daily" | "weekly" | "monthly";
		taken_at: string;
	}[];
};

/** List this Person's browser Sessions by device and last use. `current` marks the Session asking, so it is never set when an Access Key asks. */
export type ListSessionsInput = Record<string, never>;
/** What list_sessions answers. */
export type ListSessionsOutput = {
	sessions: {
		created_at: string;
		current: boolean;
		id: string;
		last_used_at: string | null;
		name: string;
		revoked: boolean;
	}[];
};

/** Rename one of your browser Sessions. A Session is named for its device when it signs in; this corrects the guess or names an older one. An ended Session is not renamed. */
export type RenameSessionInput = {
	name: string;
	session_id: string;
};
/** What rename_session answers. */
export type RenameSessionOutput = {
	id: string;
	name: string;
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

/** Create a Kitchen: a group of People who see and cook from each other's Cookbooks. Its creator is its first member. */
export type CreateKitchenInput = {
	name: string;
};
/** What create_kitchen answers. */
export type CreateKitchenOutput = {
	cookbooks: {
		authors: {
			name: string;
			person_id: string;
		}[];
		id: string;
		name: string | null;
	}[];
	id: string;
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
		cookbooks: {
			authors: {
				name: string;
				person_id: string;
			}[];
			id: string;
			name: string | null;
		}[];
		id: string;
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
	cookbooks: {
		authors: {
			name: string;
			person_id: string;
		}[];
		id: string;
		name: string | null;
	}[];
	id: string;
	members: {
		name: string;
		person_id: string;
	}[];
	name: string;
	nickname: string | null;
};

/** Remove a Person from a Kitchen — including yourself, to leave. Their Cookbook leaves with them; each member who stays keeps a Branch of every recipe of theirs they cooked, and they keep one of every recipe they cooked from the others. */
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

/** What removing a Person from a Kitchen would leave each side, before anybody does it: how many recipes the members who stay keep, and how many the one leaving keeps. `person_id` defaults to you. */
export type PreviewLeavingKitchenInput = {
	kitchen_id: string;
	person_id?: string;
};
/** What preview_leaving_kitchen answers. */
export type PreviewLeavingKitchenOutput = {
	they_keep: number;
	you_keep: number;
};

/** Your own Cookbook: its name, who writes it, how many recipes it holds, the Kitchens that see it and the Invites still waiting. */
export type GetCookbookInput = Record<string, never>;
/** What get_cookbook answers. */
export type GetCookbookOutput = {
	authors: {
		name: string;
		person_id: string;
	}[];
	id: string;
	invites: {
		created_at: string;
		invite_id: string;
	}[];
	kitchens: {
		id: string;
		name: string;
	}[];
	name: string | null;
	recipe_count: number;
};

/** Give your Cookbook a name of its own, or clear it back to its Co-authors' names with an empty or null one. Any Co-author may. */
export type RenameCookbookInput = {
	name: string | null;
};
/** What rename_cookbook answers. */
export type RenameCookbookOutput = {
	authors: {
		name: string;
		person_id: string;
	}[];
	id: string;
	invites: {
		created_at: string;
		invite_id: string;
	}[];
	kitchens: {
		id: string;
		name: string;
	}[];
	name: string | null;
	recipe_count: number;
};

/** Mint a one-use Invite for somebody to write your Cookbook with you. When they accept, their recipes and yours become one Cookbook either of you changes. */
export type InviteToCookbookInput = Record<string, never>;
/** What invite_to_cookbook answers. */
export type InviteToCookbookOutput = {
	invite_id: string;
	secret: string;
};

/** End a Cookbook Invite nobody has used yet. */
export type CancelCookbookInviteInput = {
	invite_id: string;
};
/** What cancel_cookbook_invite answers. */
export type CancelCookbookInviteOutput = {
	ended: boolean;
};

/** What accepting a Cookbook Invite would do, before you say yes: whose Cookbook it is, and how many recipes on each side become one. */
export type ReadCookbookInviteInput = {
	secret: string;
};
/** What read_cookbook_invite answers. */
export type ReadCookbookInviteOutput = {
	already_yours: boolean;
	cookbook: {
		authors: {
			name: string;
			person_id: string;
		}[];
		id: string;
		invites: {
			created_at: string;
			invite_id: string;
		}[];
		kitchens: {
			id: string;
			name: string;
		}[];
		name: string | null;
		recipe_count: number;
	};
	their_recipes: number;
	together_recipes: number;
	your_recipes: number;
};

/** Open a Cookbook Invite: your Cookbook joins the one it names, and every recipe in either becomes one Cookbook you both change. Spent on use. */
export type AcceptCookbookInviteInput = {
	secret: string;
};
/** What accept_cookbook_invite answers. */
export type AcceptCookbookInviteOutput = {
	authors: {
		name: string;
		person_id: string;
	}[];
	id: string;
	invites: {
		created_at: string;
		invite_id: string;
	}[];
	kitchens: {
		id: string;
		name: string;
	}[];
	name: string | null;
	recipe_count: number;
};

/** Leave the Cookbook you write with others, taking your own Branch of every recipe in it with its whole history. Whoever started a recipe keeps the original; everyone else a copy. */
export type LeaveCookbookInput = Record<string, never>;
/** What leave_cookbook answers. */
export type LeaveCookbookOutput = {
	authors: {
		name: string;
		person_id: string;
	}[];
	id: string;
	invites: {
		created_at: string;
		invite_id: string;
	}[];
	kitchens: {
		id: string;
		name: string;
	}[];
	name: string | null;
	recipe_count: number;
};

/** Separate another Co-author from your Cookbook. They leave with a Branch of every recipe in it, as though they had left. */
export type RemoveCookbookAuthorInput = {
	person_id: string;
};
/** What remove_cookbook_author answers. */
export type RemoveCookbookAuthorOutput = {
	authors: {
		name: string;
		person_id: string;
	}[];
	id: string;
	invites: {
		created_at: string;
		invite_id: string;
	}[];
	kitchens: {
		id: string;
		name: string;
	}[];
	name: string | null;
	recipe_count: number;
};

/** Create a Tag in your own Cookbook, named in one Language. A word the Cookbook already files under returns the Tag it already has rather than making a second. */
export type CreateTagInput = {
	kitchen_id?: string;
	language: "en" | "fr" | "es";
	name: string;
};
/** What create_tag answers. */
export type CreateTagOutput = {
	cookbook_id: string;
	id: string;
	language: string | null;
	language_fallback: boolean;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
	recipes: number;
};

/** List every Tag your own Cookbook files by, each shown in the reader's Reading Language where it has a name there. With `everywhere`, every word any Cookbook you may see files by, one entry per word — what a shelf filters by. With a `kitchen_id`, the same for the Cookbooks seen in that one Kitchen of yours. */
export type ListTagsInput = {
	everywhere?: boolean;
	kitchen_id?: string;
};
/** What list_tags answers. */
export type ListTagsOutput = {
	tags: {
		cookbook_id: string;
		id: string;
		language: string | null;
		language_fallback: boolean;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
		recipes: number;
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
	cookbook_id: string;
	id: string;
	language: string | null;
	language_fallback: boolean;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
	recipes: number;
};

/** Merge two of a Kitchen's Tags into one: every recipe filed under the merged Tag is filed under the kept one instead. Mints no Version. */
export type MergeTagsInput = {
	keep_tag_id: string;
	merge_tag_id: string;
};
/** What merge_tags answers. */
export type MergeTagsOutput = {
	cookbook_id: string;
	id: string;
	language: string | null;
	language_fallback: boolean;
	name: string | null;
	names: {
		language: string;
		name: string;
	}[];
	recipes: number;
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
		cookbook_id: string;
		id: string;
		language: string | null;
		language_fallback: boolean;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
		recipes: number;
	}[];
};

/** Relate one of your Cookbook's Recipes to any Recipe you may see, or take that single two-way, untyped link back off. Your Cookbook keeps the link. It never changes either Recipe or travels in a Bundle or Share. Name the far end with `related_branch_id`, or with `related_lineage_id` where the Recipe there has since been deleted — exactly one of the two. */
export type SetRelatedRecipeInput = {
	branch_id: string;
	related: boolean;
	related_branch_id?: string;
	related_lineage_id?: string;
};
/** What set_related_recipe answers. */
export type SetRelatedRecipeOutput = {
	related_recipes: {
		branch_id: string | null;
		language: string | null;
		language_fallback: boolean;
		lineage_id: string;
		main_photo: string | null;
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
	kitchen_id?: string;
	language?: "en" | "fr" | "es" | "unknown";
	main_photo?: string | null;
	note?: string | null;
	nutrition?: {
		basis: "per_serving" | "per_100g";
		calories: number;
	} | null;
	prep_time_minutes?: number | null;
	source?: {
		link: string | null;
		text: string;
	} | null;
	steps?: {
		kind: "section" | "step";
		photo?: string | null;
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
	cookbook: {
		authors: {
			name: string;
			person_id: string;
		}[];
		id: string;
		name: string | null;
	};
	cooked: {
		count: number;
		last_cooked_at: string | null;
		ratings: {
			at: string;
			name: string;
			person_id: string;
			rating: "again" | "tweak" | "no";
		}[];
	};
	hand_id: string;
	head_version_id: string;
	language: string;
	lineage_id: string;
	name: string | null;
	origin_address: string | null;
	related_recipes: {
		branch_id: string | null;
		language: string | null;
		language_fallback: boolean;
		lineage_id: string;
		main_photo: string | null;
		title: string;
	}[];
	tags: {
		cookbook_id: string;
		id: string;
		language: string | null;
		language_fallback: boolean;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
		recipes: number;
	}[];
	translation: {
		source_branch_id: string | null;
		translates_version_id: string;
		versions_behind: number | null;
	} | null;
	versions: {
		change_note: string | null;
		components: {
			branch_id: string | null;
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			} | null;
			held: boolean;
			lineage_id: string;
			measured: {
				ingredients: (string | null)[];
				steps: (string | null)[];
			} | null;
			path: number[];
			readings: ({
				amount: string | null;
				lineage_id: string | null;
				target: string | null;
				unit: string | null;
			} | null)[] | null;
			said: string;
			share: number | null;
			stopped: boolean;
			title: string | null;
		}[];
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		cooking: {
			steps: ({
				timer_seconds: number | null;
				uses: number[];
			} | null)[];
		};
		created_at: string;
		hand_id: string;
		language: string | null;
		measured: {
			ingredients: (string | null)[];
			steps: (string | null)[];
		};
		name: string | null;
		parent_version_id: string | null;
		readings: ({
			amount: string | null;
			lineage_id: string | null;
			target: string | null;
			unit: string | null;
		} | null)[];
		scaled_to: {
			amount: string;
			noun: string;
		} | null;
		sequence: number;
		translates_version_id: string | null;
		version_id: string;
	}[];
	writes: boolean;
};

/** Save a new state of a Recipe onto a Branch — the whole recipe as written, replacing what was there. A rapid re-save by the same Hand collapses into the Version already being shaped rather than starting a new one. Changing a recipe your Cookbook did not write — a Kitchen-mate's, or one that arrived — is a Copy: it starts a new Branch of the same Lineage in your own Cookbook, starting at the Version you changed and carrying the whole chain behind it — the Branch you changed is left untouched. The Branch must be one you may see. */
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
	nutrition?: {
		basis: "per_serving" | "per_100g";
		calories: number;
	} | null;
	prep_time_minutes?: number | null;
	source?: {
		link: string | null;
		text: string;
	} | null;
	steps?: {
		kind: "section" | "step";
		photo?: string | null;
		text: string;
	}[];
	title: string;
	translates_version_id?: string;
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
	language: string;
	language_offer: string | null;
	parent_version_id: string | null;
	sequence: number;
	translates_version_id: string | null;
	version_id: string;
};

/** Start a variation of a recipe: a Branch of it, unchanged, in your own Cookbook, under a name you give it ("Vegetarian"). Changing one never changes the other. */
export type StartVariationInput = {
	branch_id: string;
	name: string;
};
/** What start_variation answers. */
export type StartVariationOutput = {
	branch_id: string;
	cookbook: {
		authors: {
			name: string;
			person_id: string;
		}[];
		id: string;
		name: string | null;
	};
	cooked: {
		count: number;
		last_cooked_at: string | null;
		ratings: {
			at: string;
			name: string;
			person_id: string;
			rating: "again" | "tweak" | "no";
		}[];
	};
	hand_id: string;
	head_version_id: string;
	language: string;
	lineage_id: string;
	name: string | null;
	origin_address: string | null;
	related_recipes: {
		branch_id: string | null;
		language: string | null;
		language_fallback: boolean;
		lineage_id: string;
		main_photo: string | null;
		title: string;
	}[];
	tags: {
		cookbook_id: string;
		id: string;
		language: string | null;
		language_fallback: boolean;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
		recipes: number;
	}[];
	translation: {
		source_branch_id: string | null;
		translates_version_id: string;
		versions_behind: number | null;
	} | null;
	versions: {
		change_note: string | null;
		components: {
			branch_id: string | null;
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			} | null;
			held: boolean;
			lineage_id: string;
			measured: {
				ingredients: (string | null)[];
				steps: (string | null)[];
			} | null;
			path: number[];
			readings: ({
				amount: string | null;
				lineage_id: string | null;
				target: string | null;
				unit: string | null;
			} | null)[] | null;
			said: string;
			share: number | null;
			stopped: boolean;
			title: string | null;
		}[];
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		cooking: {
			steps: ({
				timer_seconds: number | null;
				uses: number[];
			} | null)[];
		};
		created_at: string;
		hand_id: string;
		language: string | null;
		measured: {
			ingredients: (string | null)[];
			steps: (string | null)[];
		};
		name: string | null;
		parent_version_id: string | null;
		readings: ({
			amount: string | null;
			lineage_id: string | null;
			target: string | null;
			unit: string | null;
		} | null)[];
		scaled_to: {
			amount: string;
			noun: string;
		} | null;
		sequence: number;
		translates_version_id: string | null;
		version_id: string;
	}[];
	writes: boolean;
};

/** Name one of your Cookbook's Branches of a recipe, or clear its name. A Cookbook keeps one unnamed Branch of a recipe in each Language, so a second one needs a name. */
export type RenameBranchInput = {
	branch_id: string;
	name: string | null;
};
/** What rename_branch answers. */
export type RenameBranchOutput = {
	branch_id: string;
	name: string | null;
};

/** Take one recipe off the shelf for good. It is gone from the shelf, from search and from every member of its Kitchen, and nothing brings it back. **One Branch**: a translation is an ordinary Branch, so deleting the English one leaves the French one whole, and another Kitchen's copy of the same recipe is untouched. **The cooking history stays.** Every Attempt ever made from this recipe keeps its rating, its note and its Photographs, and the Cooked diary keeps each entry under the name the recipe was known by. So does a Shopping List holding it, which says it can no longer be read rather than quietly dropping it. No Version is ever deleted, by this or by anything else. A live Share Link stops working. */
export type DeleteRecipeInput = {
	branch_id: string;
};
/** What delete_recipe answers. */
export type DeleteRecipeOutput = {
	deleted: boolean;
};

/** Read a whole recipe pasted as text into a title, an ingredient list and a method. Decides only what each line IS — an Ingredient Line, a Step, a Section — and never what it says: every line comes back exactly as pasted, with no amount extracted, no rewording and no reordering (ADR 0002). Nothing is guessed beyond the split and the title: no Yield, no times, no Source, and no Component (ADR 0008). It writes nothing anywhere — what comes back is shown to whoever pasted it, who moves the boundary if it landed wrong, and only then is a recipe saved by an ordinary create_recipe or save_recipe_version. The boundary is the index in `lines` where the method starts, so moving it re-splits the same answer without asking again. */
export type ReadPastedRecipeInput = {
	text: string;
};
/** What read_pasted_recipe answers. */
export type ReadPastedRecipeOutput = {
	boundary: number;
	lines: {
		kind: "line" | "section";
		text: string;
	}[];
	title: string | null;
};

/** Translate a recipe: start an ordinary Branch of the same Lineage in another Language, whose first Version records which Version of the source it renders. There is no Translation object — what this makes is a Branch, and every Operation from here on is the ordinary one. Its chain starts fresh rather than carrying the source's, which is what separates it from a Copy: different words rendering the same dish, with a history of their own. An agent translating calls this under the Person's own Credential and is a scribe, not an author. */
export type StartTranslationInput = {
	branch_id: string;
	change_note?: string;
	cook_time_minutes?: number | null;
	ingredients?: {
		kind: "section" | "ingredient";
		text: string;
	}[];
	kitchen_id?: string;
	language: "en" | "fr" | "es";
	main_photo?: string | null;
	name?: string;
	note?: string | null;
	nutrition?: {
		basis: "per_serving" | "per_100g";
		calories: number;
	} | null;
	prep_time_minutes?: number | null;
	source?: {
		link: string | null;
		text: string;
	} | null;
	steps?: {
		kind: "section" | "step";
		photo?: string | null;
		text: string;
	}[];
	title: string;
	translates_version_id?: string;
	yield?: {
		amount: string;
		noun: string;
	} | null;
};
/** What start_translation answers. */
export type StartTranslationOutput = {
	branch_id: string;
	cookbook: {
		authors: {
			name: string;
			person_id: string;
		}[];
		id: string;
		name: string | null;
	};
	cooked: {
		count: number;
		last_cooked_at: string | null;
		ratings: {
			at: string;
			name: string;
			person_id: string;
			rating: "again" | "tweak" | "no";
		}[];
	};
	hand_id: string;
	head_version_id: string;
	language: string;
	lineage_id: string;
	name: string | null;
	origin_address: string | null;
	related_recipes: {
		branch_id: string | null;
		language: string | null;
		language_fallback: boolean;
		lineage_id: string;
		main_photo: string | null;
		title: string;
	}[];
	tags: {
		cookbook_id: string;
		id: string;
		language: string | null;
		language_fallback: boolean;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
		recipes: number;
	}[];
	translation: {
		source_branch_id: string | null;
		translates_version_id: string;
		versions_behind: number | null;
	} | null;
	versions: {
		change_note: string | null;
		components: {
			branch_id: string | null;
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			} | null;
			held: boolean;
			lineage_id: string;
			measured: {
				ingredients: (string | null)[];
				steps: (string | null)[];
			} | null;
			path: number[];
			readings: ({
				amount: string | null;
				lineage_id: string | null;
				target: string | null;
				unit: string | null;
			} | null)[] | null;
			said: string;
			share: number | null;
			stopped: boolean;
			title: string | null;
		}[];
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		cooking: {
			steps: ({
				timer_seconds: number | null;
				uses: number[];
			} | null)[];
		};
		created_at: string;
		hand_id: string;
		language: string | null;
		measured: {
			ingredients: (string | null)[];
			steps: (string | null)[];
		};
		name: string | null;
		parent_version_id: string | null;
		readings: ({
			amount: string | null;
			lineage_id: string | null;
			target: string | null;
			unit: string | null;
		} | null)[];
		scaled_to: {
			amount: string;
			noun: string;
		} | null;
		sequence: number;
		translates_version_id: string | null;
		version_id: string;
	}[];
	writes: boolean;
};

/** Say what Language a recipe is written in. The only thing that acts on a save's language offer — Kamosu detects and offers, and never changes a Language without the cook saying so. Changing it makes a Version, so the change leaves a trace in the recipe's own history. Setting it to `unknown` says the recipe is honestly more than one Language: from then on it is offered nothing, marked nothing, and shown to every reader whatever they read in. */
export type SetRecipeLanguageInput = {
	branch_id: string;
	language: "en" | "fr" | "es" | "unknown";
};
/** What set_recipe_language answers. */
export type SetRecipeLanguageOutput = {
	branch_id: string;
	language: string;
	sequence: number | null;
};

/** Bring a batch of already-read recipes into your own Cookbook, as a Job. Matched by foreign id against this Cookbook's ledger for the source kind, so re-running finds what it already made instead of doubling it; a recipe found changed is offered for review, never written over. Reading the outside source itself — a file, a page, a Bundle — is each importer's own job. */
export type ImportInput = {
	candidates: {
		cook_time_minutes?: number | null;
		foreign_id: string;
		ingredients?: {
			kind: "section" | "ingredient";
			text: string;
		}[];
		language?: "en" | "fr" | "es" | "unknown";
		main_photo?: string | null;
		note?: string | null;
		nutrition?: {
			basis: "per_serving" | "per_100g";
			calories: number;
		} | null;
		prep_time_minutes?: number | null;
		source?: {
			link: string | null;
			text: string;
		} | null;
		steps?: {
			kind: "section" | "step";
			photo?: string | null;
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
		bare?: boolean;
		branch_id: string;
		foreign_id: string;
		lineage_id: string;
		main_photo?: string | null;
		status: "created" | "extended" | "unchanged";
		subject?: boolean;
		title: string;
	}[];
	cookbook_id: string;
	import_id: string;
	left_out: {
		branch_id: string;
		count: number;
		foreign_id: string;
		icon?: string;
		title: string;
		what: "site_icon" | "extra_photos" | "unreadable_photos";
	}[];
	offered: {
		branch_id: string;
		candidate_version_id: string;
		foreign_id: string;
		lineage_id: string;
		title: string;
	}[];
	related_candidates: {
		recipes: {
			branch_id: string;
			ingredients: number;
			lineage_id: string;
			main_photo: string | null;
			title: string;
		}[];
		shared: ("name" | "page")[];
	}[];
	source_kind: string;
	unreadable: {
		foreign_id: string | null;
		kept_as?: {
			branch_id: string;
			lineage_id: string;
			title: string;
		};
		name?: string;
		reason: string;
	}[];
};

/** Bring in a Crouton library, as a Job: the whole export (a zip of .crumb files) or one .crumb. Each recipe lands in your own Cookbook through the same ledger `import` uses, keyed by its Crouton id, so running it again matches instead of doubling the library. Ingredient Lines are rebuilt from Crouton's split fields; the site's favicon and Crouton's nutrition text are left out. Send the file to POST /api/uploads and pass the `upload_id` it answers, or pass it base64-encoded as `data`. */
export type ImportCroutonInput = {
	data?: string;
	upload_id?: string;
};
/** What import_crouton eventually produces, read back through `get_job`. */
export type ImportCroutonOutput = {
	arrived: {
		bare?: boolean;
		branch_id: string;
		foreign_id: string;
		lineage_id: string;
		main_photo?: string | null;
		status: "created" | "extended" | "unchanged";
		subject?: boolean;
		title: string;
	}[];
	cookbook_id: string;
	import_id: string;
	left_out: {
		branch_id: string;
		count: number;
		foreign_id: string;
		icon?: string;
		title: string;
		what: "site_icon" | "extra_photos" | "unreadable_photos";
	}[];
	offered: {
		branch_id: string;
		candidate_version_id: string;
		foreign_id: string;
		lineage_id: string;
		title: string;
	}[];
	related_candidates: {
		recipes: {
			branch_id: string;
			ingredients: number;
			lineage_id: string;
			main_photo: string | null;
			title: string;
		}[];
		shared: ("name" | "page")[];
	}[];
	source_kind: string;
	unreadable: {
		foreign_id: string | null;
		kept_as?: {
			branch_id: string;
			lineage_id: string;
			title: string;
		};
		name?: string;
		reason: string;
	}[];
};

/** What has been brought into your Kitchens from outside, and what happened each time. One entry per source — a Crouton library, recipe files, web pages — each holding how many recipes its ledger remembers and every arrival you asked for, newest first. An arrival names the Job whose Report `get_job` serves, so what happened is read back long after the screen that started it closed. Listed is an event, never a mark on a recipe: an imported recipe is an ordinary recipe and says nothing about where it came from (ADR 0025). */
export type ListImportsInput = Record<string, never>;
/** What list_imports answers. */
export type ListImportsOutput = {
	imports: {
		arrivals: {
			arrived: number;
			created: number;
			created_at: string;
			job_id: string;
			offered: number;
			status: "queued" | "running" | "completed" | "failed" | "cancelled";
			unreadable: number;
		}[];
		created_at: string;
		import_id: string | null;
		remembered: number;
		source_kind: string;
	}[];
};

/** Throw an Import's ledger away whole — the memory of which outside recipe became which of yours. Every recipe it made stays exactly as it is. Once forgotten, importing the same file again brings everything in as new, so do this when the place it came from is gone. */
export type ForgetImportInput = {
	import_id: string;
};
/** What forget_import answers. */
export type ForgetImportOutput = {
	forgotten: number;
	import_id: string;
};

/** Bring in a recipe straight from a URL, as a Job. Reads the page's schema.org JSON-LD (#70) — no per-site scraping, no LLM fallback — and lands it in your own Cookbook through the same ledger `import` uses, keyed by the page's own address. Fetching is bound to public addresses at the dialled address and at every redirect (ADR 0033), and — because a page's own text can tell an agent to fetch another URL — always takes the single depth-one lane, never more than one fetch in flight regardless of who is signed in. */
export type ImportWebLinkInput = {
	url: string;
};
/** What import_web_link eventually produces, read back through `get_job`. */
export type ImportWebLinkOutput = {
	arrived: {
		bare?: boolean;
		branch_id: string;
		foreign_id: string;
		lineage_id: string;
		main_photo?: string | null;
		status: "created" | "extended" | "unchanged";
		subject?: boolean;
		title: string;
	}[];
	cookbook_id: string;
	import_id: string;
	left_out: {
		branch_id: string;
		count: number;
		foreign_id: string;
		icon?: string;
		title: string;
		what: "site_icon" | "extra_photos" | "unreadable_photos";
	}[];
	offered: {
		branch_id: string;
		candidate_version_id: string;
		foreign_id: string;
		lineage_id: string;
		title: string;
	}[];
	related_candidates: {
		recipes: {
			branch_id: string;
			ingredients: number;
			lineage_id: string;
			main_photo: string | null;
			title: string;
		}[];
		shared: ("name" | "page")[];
	}[];
	source_kind: string;
	unreadable: {
		foreign_id: string | null;
		kept_as?: {
			branch_id: string;
			lineage_id: string;
			title: string;
		};
		name?: string;
		reason: string;
	}[];
};

/** Rename a Version — the one thing about it that can change later. An absent or empty name clears it. Targeted by the Branch's own sequence number, since the same content can recur more than once on one Branch, each occurrence named on its own. Only the Person who saved that Version may rename it. */
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

/** The shelf, and searching it. With no query: everything the Kitchens this Person cooks in hold, merged, alphabetical, one entry per Lineage, each titled in the reader's Reading Language with a marked fallback. With a query: the same shelf narrowed to what matched, an exact title first, every entry quoting the line that matched. One Operation either way — Meaning Search arrives here rather than beside it (ADR 0027, ADR 0029). */
export type SearchRecipesInput = {
	kitchen_id?: string | null;
	mine?: boolean;
	query?: string | null;
	tag_id?: string | null;
};
/** What search_recipes answers. */
export type SearchRecipesOutput = {
	closest: boolean;
	query: string | null;
	recipes: {
		branch_id: string;
		language: string;
		language_fallback: boolean;
		lineage_id: string;
		main_photo: string | null;
		matched: {
			by: "words" | "meaning";
			line: string;
			step_number: number | null;
			where: "title" | "tag" | "ingredient" | "step" | "section" | "note" | "attempt";
		} | null;
		title: string;
		yield: {
			amount: string;
			noun: string;
		} | null;
	}[];
};

/** Home: the computed shelves that answer *show me something* rather than handing back a search box — cooked most, quick tonight, never cooked, recently opened. Each is one card per Lineage in the reader's Reading Language, in the same shape the library's shelf answers in. A shelf with nothing on it is left out rather than sent empty, so an instance holding no recipes answers with no shelves at all. All four are counted from recipes and Attempts that already exist, except *recently opened*, which reads what `note_recipe_opened` remembered (ADR 0011, ADR 0027). */
export type HomeShelvesInput = Record<string, never>;
/** What home_shelves answers. */
export type HomeShelvesOutput = {
	quick_tonight_minutes: number;
	shelves: {
		name: "cooked_most" | "quick_tonight" | "never_cooked" | "recently_opened";
		recipes: {
			branch_id: string;
			language: string;
			language_fallback: boolean;
			lineage_id: string;
			main_photo: string | null;
			matched: {
				by: "words" | "meaning";
				line: string;
				step_number: number | null;
				where: "title" | "tag" | "ingredient" | "step" | "section" | "note" | "attempt";
			} | null;
			title: string;
			yield: {
				amount: string;
				noun: string;
			} | null;
		}[];
	}[];
};

/** Remember that the caller opened this recipe, for Home's *recently opened* shelf. One fact per Person per Lineage — opening a recipe's French Branch and its English one is opening the same recipe — and opening it again moves the time rather than adding a row. It is private to the Person, never travels, and is in no fingerprint, Vault or Bundle: an instance that lost it would lose the order of one shelf and nothing else (ADR 0027). */
export type NoteRecipeOpenedInput = {
	branch_id: string;
};
/** What note_recipe_opened answers. */
export type NoteRecipeOpenedOutput = {
	lineage_id: string;
	opened_at: string;
};

/** Whether Meaning Search is on here, what model it would use, who accepted that model's terms — and whether this caller should be offered it. Answers on every instance, including the many that will never turn it on. */
export type MeaningSearchStatusInput = Record<string, never>;
/** What meaning_search_status answers. */
export type MeaningSearchStatusOutput = {
	accepted_at: string | null;
	accepted_by: string | null;
	accepted_via_access_key: boolean | null;
	declined_at: string | null;
	indexed_at: string | null;
	may_change: boolean;
	model: string;
	model_present: boolean;
	offer: boolean;
	on: boolean;
	prohibited_use_policy_url: string;
	recipes_not_yet_indexed: number;
	state: "unasked" | "declined" | "accepted" | "on";
	terms_url: string;
	terms_version: string;
};

/** Accept the terms of the model Meaning Search needs. Kamosu ships no weights (ADR 0029): the person who accepts the terms is the person the terms are about, and the acceptance keeps the Hand that made it and whether it arrived by login or by Access Key. Available at both Doors — a web-only carve-out would be the first hole in Parity, and would stop nothing anyway. */
export type AcceptMeaningSearchTermsInput = Record<string, never>;
/** What accept_meaning_search_terms answers. */
export type AcceptMeaningSearchTermsOutput = {
	state: "unasked" | "declined" | "accepted" | "on";
};

/** Decline the model's terms. Meaning Search stays off and the offer is never made again on this instance — a question already answered, asked twice, is a nag. */
export type DeclineMeaningSearchInput = Record<string, never>;
/** What decline_meaning_search answers. */
export type DeclineMeaningSearchOutput = {
	state: "unasked" | "declined" | "accepted" | "on";
};

/** Fetch the Meaning Search model into /data, as a Job. No weights ship in the image; this is the only way any arrive, and only after the terms have been accepted. The download is pinned to one revision and verified against a manifest, so a half-finished one is never mistaken for a model. */
export type DownloadMeaningModelInput = Record<string, never>;
/** What download_meaning_model eventually produces, read back through `get_job`. */
export type DownloadMeaningModelOutput = {
	model: string;
	repository: string;
	revision: string;
};

/** Read the library into the Meaning Search index, as a Job, and turn Meaning Search on. Incremental: what is read is what the index does not already hold, so the first run is the whole library and every later one is whatever changed. The index is derived from the recipes and can be rebuilt at any time. Kamosu also does this by itself, within the minute, whenever a recipe changes. */
export type BuildMeaningIndexInput = Record<string, never>;
/** What build_meaning_index eventually produces, read back through `get_job`. */
export type BuildMeaningIndexOutput = {
	indexed: number;
};

/** Stop matching on meaning and throw the index away. Discards nothing that cannot be rebuilt — the index is derived from the recipes — and keeps both the acceptance, which is history, and the downloaded weights, so turning it back on is a rebuild rather than another download. */
export type TurnOffMeaningSearchInput = Record<string, never>;
/** What turn_off_meaning_search answers. */
export type TurnOffMeaningSearchOutput = {
	state: "unasked" | "declined" | "accepted" | "on";
};

/** Read a Recipe: the Branch as it stands and its whole chain of Versions, oldest first. Each Version's `measured` lines are scaled to `wanted_yield` where one is given (null for the recipe as written), and otherwise to the Yield the caller's own In Progress Attempt is cooking to; `scaled_to` says which, or is null where the amounts are as written. Nothing is stored. */
export type GetRecipeInput = {
	branch_id: string;
	wanted_yield?: {
		amount: string;
		noun: string;
	} | null;
};
/** What get_recipe answers. */
export type GetRecipeOutput = {
	branch_id: string;
	cookbook: {
		authors: {
			name: string;
			person_id: string;
		}[];
		id: string;
		name: string | null;
	};
	cooked: {
		count: number;
		last_cooked_at: string | null;
		ratings: {
			at: string;
			name: string;
			person_id: string;
			rating: "again" | "tweak" | "no";
		}[];
	};
	hand_id: string;
	head_version_id: string;
	language: string;
	lineage_id: string;
	name: string | null;
	origin_address: string | null;
	related_recipes: {
		branch_id: string | null;
		language: string | null;
		language_fallback: boolean;
		lineage_id: string;
		main_photo: string | null;
		title: string;
	}[];
	tags: {
		cookbook_id: string;
		id: string;
		language: string | null;
		language_fallback: boolean;
		name: string | null;
		names: {
			language: string;
			name: string;
		}[];
		recipes: number;
	}[];
	translation: {
		source_branch_id: string | null;
		translates_version_id: string;
		versions_behind: number | null;
	} | null;
	versions: {
		change_note: string | null;
		components: {
			branch_id: string | null;
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			} | null;
			held: boolean;
			lineage_id: string;
			measured: {
				ingredients: (string | null)[];
				steps: (string | null)[];
			} | null;
			path: number[];
			readings: ({
				amount: string | null;
				lineage_id: string | null;
				target: string | null;
				unit: string | null;
			} | null)[] | null;
			said: string;
			share: number | null;
			stopped: boolean;
			title: string | null;
		}[];
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		cooking: {
			steps: ({
				timer_seconds: number | null;
				uses: number[];
			} | null)[];
		};
		created_at: string;
		hand_id: string;
		language: string | null;
		measured: {
			ingredients: (string | null)[];
			steps: (string | null)[];
		};
		name: string | null;
		parent_version_id: string | null;
		readings: ({
			amount: string | null;
			lineage_id: string | null;
			target: string | null;
			unit: string | null;
		} | null)[];
		scaled_to: {
			amount: string;
			noun: string;
		} | null;
		sequence: number;
		translates_version_id: string | null;
		version_id: string;
	}[];
	writes: boolean;
};

/** Read the Thread: every Version of every Branch of one Lineage this Person can see, oldest first per Branch, with every Attempt hanging off it. branch_id is only the entry point — any Branch of the Lineage answers the same Thread. */
export type GetThreadInput = {
	branch_id: string;
};
/** What get_thread answers. */
export type GetThreadOutput = {
	attempts: {
		as_cooked: {
			against: {
				ingredients: {
					from_branch_point: boolean;
					kind: string;
					mine: {
						index: number;
						kind: string;
						text: string;
					} | null;
					state: "same" | "changed" | "only-mine" | "only-theirs";
					theirs: {
						index: number;
						kind: string;
						text: string;
					} | null;
				}[];
				steps: {
					from_branch_point: boolean;
					kind: string;
					mine: {
						index: number;
						kind: string;
						text: string;
					} | null;
					state: "same" | "changed" | "only-mine" | "only-theirs";
					theirs: {
						index: number;
						kind: string;
						text: string;
					} | null;
				}[];
			};
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			promotion_declined: boolean;
			version_id: string;
		} | null;
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
		photographs: string[];
		rating: "again" | "tweak" | "no" | null;
		resumable: boolean;
		ticked_ingredients: number[];
		version_id: string;
	}[];
	branches: {
		arrived: boolean;
		branch_id: string;
		cookbook: {
			authors: {
				name: string;
				person_id: string;
			}[];
			id: string;
			name: string | null;
		};
		hand_id: string;
		hand_name: string | null;
		head_version_id: string;
		language: string;
		mine: boolean;
		name: string | null;
		translation: {
			source_branch_id: string | null;
			translates_version_id: string;
			versions_behind: number | null;
		} | null;
	}[];
	lineage_id: string;
	versions: {
		branch_id: string;
		change_note: string | null;
		created_at: string;
		hand_id: string;
		hand_name: string | null;
		language: string | null;
		name: string | null;
		parent_version_id: string | null;
		sequence: number;
		translates_version_id: string | null;
		version_id: string;
	}[];
};

/** Turn a Recipe's Share Link on, and answer the link. One permanent, unguessable address per Recipe, never expiring, freely passed on. Asking twice for a Recipe already shared answers the link it already has rather than minting a second one. The link's Secret is answered exactly once — here, at the moment it is minted — because only its hash is stored. The instance's public address is asked for at the first Share Link and stored once; a link is kept as a token rather than a URL, so setting the address later makes every link already minted render correctly. */
export type ShareRecipeInput = {
	branch_id: string;
	public_address?: string;
};
/** What share_recipe answers. */
export type ShareRecipeOutput = {
	created_at: string | null;
	public_address: string | null;
	share_id: string | null;
	shared: boolean;
	shared_by: string | null;
	url: string | null;
};

/** End a Recipe's Share Link. Permanent: the link stops working and turning sharing back on mints a new one, so a withdrawn link stays dead. It reaches no copy already sent, and Kamosu says so rather than letting that be discovered. */
export type EndShareLinkInput = {
	branch_id: string;
};
/** What end_share_link answers. */
export type EndShareLinkOutput = {
	created_at: string | null;
	public_address: string | null;
	share_id: string | null;
	shared: boolean;
	shared_by: string | null;
	url: string | null;
};

/** Whether a Recipe is shared, and by whom. The link's URL is answered only at the moment it is minted, since only the Secret's hash is stored — so this says a link exists without being able to reprint it. */
export type GetShareLinkInput = {
	branch_id: string;
};
/** What get_share_link answers. */
export type GetShareLinkOutput = {
	created_at: string | null;
	public_address: string | null;
	share_id: string | null;
	shared: boolean;
	shared_by: string | null;
	url: string | null;
};

/** Where this instance currently says it is reachable from outside, or nothing if it has never been asked. The Operator's half of `set_public_address`: changing an address you cannot see is a guess. */
export type GetPublicAddressInput = Record<string, never>;
/** What get_public_address answers. */
export type GetPublicAddressOutput = {
	public_address: string | null;
};

/** Change where this instance says it is reachable from outside. Kept in the database and never in an environment variable, so moving an instance is one act rather than a redeployment. It fixes the future, not the past: Share Links minted after it carry the new address, while a link already sent stays the text it was sent as and cannot be reissued — only the secret's hash is kept, so Kamosu can no longer print that link at all. */
export type SetPublicAddressInput = {
	public_address: string;
};
/** What set_public_address answers. */
export type SetPublicAddressOutput = {
	public_address: string;
};

/** Write a Bundle of one recipe: a plain zip holding a readable Markdown note per recipe with its Thread beneath it, its Photographs, and a hidden .kamosu/ sidecar carrying every Version complete back to the first, the Readings and the ids. It carries the Branch named, its Translations, and every Component it needs as a Passenger. This answers what the Bundle holds; fetch its bytes at GET /api/bundles/<branch_id> under the same Credential. Nothing is sent anywhere and nothing is changed. */
export type ExportBundleInput = {
	branch_id: string;
};
/** What export_bundle answers. */
export type ExportBundleOutput = {
	fetch_at: string;
	file_name: string;
	missing_photographs: string[];
	notes: string[];
	passengers: {
		lineage_id: string;
		title: string | null;
	}[];
	photographs: number;
	subjects: {
		lineage_id: string;
		title: string | null;
	}[];
};

/** Set a Sheet of one recipe: the Branch as it stands on this Person's screen, set for paper as a PDF. It carries the recipe and not the library — no Tags, Attempts, Thread or past Versions. Written Ingredient Lines are printed and Readings are not, except the amount beneath a line when a cooking has scaled the recipe; Components unfold after it, parent first, each already scaled. Letter for US Reading Measures, A4 otherwise. `wanted_yield` is the Yield the screen is scaled to, as `get_recipe` takes it. When the Job completes, fetch the PDF at GET /api/sheets/<job_id> under the same Credential. Nothing is changed. */
export type MakeSheetInput = {
	branch_id: string;
	wanted_yield?: {
		amount: string;
		noun: string;
	} | null;
};
/** What make_sheet eventually produces, read back through `get_job`. */
export type MakeSheetOutput = {
	fetch_at: string;
	file_name: string;
	kept_as: string;
	pages: number;
	paper: "a4" | "us-letter";
};

/** Set a Sheet of the recipe a Share Link shows, for anyone holding the link — no account needed. The recipe is printed as written, with its Components unfolded after it at the amount each line asks for. `language` picks one of the link's Translations; `locale` is the reader's locale (a US or Canadian one prints Letter, anything else A4) and decides nothing but the paper. When the Job completes, fetch the PDF at GET /api/sheets/<job_id>. */
export type MakeSharedSheetInput = {
	language?: string;
	locale?: string;
	token: string;
};
/** What make_shared_sheet eventually produces, read back through `get_job`. */
export type MakeSharedSheetOutput = {
	fetch_at: string;
	file_name: string;
	kept_as: string;
	pages: number;
	paper: "a4" | "us-letter";
};

/** Receive a Bundle into your own Cookbook, as a Job. Every recipe it carries is placed under the sender's Hands and travels on under the sender's ids, its Versions, Readings and Photographs exactly as they were sent, while your Cookbook holds it under an id of this instance's own; one your Cookbook already holds is extended by whatever the Bundle carries past it, so the same friend's next Bundle continues their recipe. Another Cookbook here holding it is no part of the question: each Cookbook receives its own copy. Receiving makes nothing of your own — changing what arrived does. A recipe whose history is damaged arrives as a new recipe of your own with no history, and the Import Report says so. Send the file to POST /api/uploads and pass the `upload_id` it answers, or pass it base64-encoded as `data`. */
export type ImportBundleInput = {
	data?: string;
	upload_id?: string;
};
/** What import_bundle eventually produces, read back through `get_job`. */
export type ImportBundleOutput = {
	arrived: {
		bare?: boolean;
		branch_id: string;
		foreign_id: string;
		lineage_id: string;
		main_photo?: string | null;
		status: "created" | "extended" | "unchanged";
		subject?: boolean;
		title: string;
	}[];
	cookbook_id: string;
	import_id: string;
	left_out: {
		branch_id: string;
		count: number;
		foreign_id: string;
		icon?: string;
		title: string;
		what: "site_icon" | "extra_photos" | "unreadable_photos";
	}[];
	offered: {
		branch_id: string;
		candidate_version_id: string;
		foreign_id: string;
		lineage_id: string;
		title: string;
	}[];
	related_candidates: {
		recipes: {
			branch_id: string;
			ingredients: number;
			lineage_id: string;
			main_photo: string | null;
			title: string;
		}[];
		shared: ("name" | "page")[];
	}[];
	source_kind: string;
	unreadable: {
		foreign_id: string | null;
		kept_as?: {
			branch_id: string;
			lineage_id: string;
			title: string;
		};
		name?: string;
		reason: string;
	}[];
};

/** Read a Recipe through its Share Link token: the Recipe as it stands, its Translations, and its Thread complete back to the first Version with every name and *what changed* line. Never an Attempt, a rating or an Attempt photograph. Public, because holding the token is the whole of the permission — this is what the Share Link page consumes, and the page is not an Operation, so Parity is untouched. */
export type ReadSharedRecipeInput = {
	token: string;
};
/** What read_shared_recipe answers. */
export type ReadSharedRecipeOutput = {
	ended: boolean;
	public_address: string | null;
	recipe: {
		branch_id: string;
		components: {
			branch_id: string | null;
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			} | null;
			held: boolean;
			lineage_id: string;
			measured: null;
			path: number[];
			readings: ({
				amount: string | null;
				lineage_id: string | null;
				target: string | null;
				unit: string | null;
			} | null)[] | null;
			said: string;
			share: number | null;
			stopped: boolean;
			title: string | null;
		}[];
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		language: string;
		lineage_id: string;
		readings: ({
			amount: string | null;
			lineage_id: string | null;
			target: string | null;
			unit: string | null;
		} | null)[];
		version_id: string;
	} | null;
	share_id: string;
	shared_by: string | null;
	thread: {
		change_note: string | null;
		created_at: string;
		hand: string;
		name: string | null;
		sequence: number;
	}[];
	translations: {
		branch_id: string;
		components: {
			branch_id: string | null;
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			} | null;
			held: boolean;
			lineage_id: string;
			measured: null;
			path: number[];
			readings: ({
				amount: string | null;
				lineage_id: string | null;
				target: string | null;
				unit: string | null;
			} | null)[] | null;
			said: string;
			share: number | null;
			stopped: boolean;
			title: string | null;
		}[];
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		language: string;
		lineage_id: string;
		readings: ({
			amount: string | null;
			lineage_id: string | null;
			target: string | null;
			unit: string | null;
		} | null)[];
		version_id: string;
	}[];
};

/** The last Version two Branches share, found by walking both chains back until they meet — never declared, always computed. A chain that does not converge on a shared first Version answers a damaged-Bundle error rather than a guess. */
export type BranchPointInput = {
	branch_a_id: string;
	branch_b_id: string;
};
/** What branch_point answers. */
export type BranchPointOutput = {
	version_id: string;
};

/** Two Branches of one Lineage laid over each other, so a screen can show two whole recipes with a switch between them rather than a difference (ADR 0014). Every row carries both sides' own words; a line only one side has is a Ghost. Which line is which is read against the Branch Point, never by an id stapled to a line (ADR 0019), and an uncertain reading declines to pair rather than claiming a connection. */
export type DivergenceInput = {
	branch_id: string;
	other_branch_id: string;
};
/** What divergence answers. */
export type DivergenceOutput = {
	branch_point_version_id: string;
	fields: {
		cook_time_minutes: {
			mine: unknown;
			same: boolean;
			theirs: unknown;
		};
		main_photo: {
			mine: unknown;
			same: boolean;
			theirs: unknown;
		};
		note: {
			mine: unknown;
			same: boolean;
			theirs: unknown;
		};
		nutrition: {
			mine: unknown;
			same: boolean;
			theirs: unknown;
		};
		prep_time_minutes: {
			mine: unknown;
			same: boolean;
			theirs: unknown;
		};
		source: {
			mine: unknown;
			same: boolean;
			theirs: unknown;
		};
		title: {
			mine: unknown;
			same: boolean;
			theirs: unknown;
		};
		yield: {
			mine: unknown;
			same: boolean;
			theirs: unknown;
		};
	};
	ingredients: {
		from_branch_point: boolean;
		kind: string;
		mine: {
			index: number;
			kind: string;
			text: string;
		} | null;
		state: "same" | "changed" | "only-mine" | "only-theirs";
		theirs: {
			index: number;
			kind: string;
			text: string;
		} | null;
	}[];
	lineage_id: string;
	mine: {
		arrived: boolean;
		branch_id: string;
		components: {
			branch_id: string | null;
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			} | null;
			held: boolean;
			lineage_id: string;
			measured: {
				ingredients: (string | null)[];
				steps: (string | null)[];
			} | null;
			path: number[];
			readings: ({
				amount: string | null;
				lineage_id: string | null;
				target: string | null;
				unit: string | null;
			} | null)[] | null;
			said: string;
			share: number | null;
			stopped: boolean;
			title: string | null;
		}[];
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		cookbook: {
			authors: {
				name: string;
				person_id: string;
			}[];
			id: string;
			name: string | null;
		};
		hand_id: string;
		hand_name: string | null;
		head_version_id: string;
		language: string;
		measured: {
			ingredients: (string | null)[];
			steps: (string | null)[];
		};
		mine: boolean;
		name: string | null;
		readings: ({
			amount: string | null;
			lineage_id: string | null;
			target: string | null;
			unit: string | null;
		} | null)[];
	};
	steps: {
		from_branch_point: boolean;
		kind: string;
		mine: {
			index: number;
			kind: string;
			text: string;
		} | null;
		state: "same" | "changed" | "only-mine" | "only-theirs";
		theirs: {
			index: number;
			kind: string;
			text: string;
		} | null;
	}[];
	theirs: {
		arrived: boolean;
		branch_id: string;
		components: {
			branch_id: string | null;
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			} | null;
			held: boolean;
			lineage_id: string;
			measured: {
				ingredients: (string | null)[];
				steps: (string | null)[];
			} | null;
			path: number[];
			readings: ({
				amount: string | null;
				lineage_id: string | null;
				target: string | null;
				unit: string | null;
			} | null)[] | null;
			said: string;
			share: number | null;
			stopped: boolean;
			title: string | null;
		}[];
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		cookbook: {
			authors: {
				name: string;
				person_id: string;
			}[];
			id: string;
			name: string | null;
		};
		hand_id: string;
		hand_name: string | null;
		head_version_id: string;
		language: string;
		measured: {
			ingredients: (string | null)[];
			steps: (string | null)[];
		};
		mine: boolean;
		name: string | null;
		readings: ({
			amount: string | null;
			lineage_id: string | null;
			target: string | null;
			unit: string | null;
		} | null)[];
	};
};

/** Correct the Reading on one Ingredient Line of a Recipe's current state — an amount, a Unit and a target, sent together as the whole new Reading (never a per-field patch, the same convention save_recipe_version uses for the whole recipe). Mints no Version and appears in no history (ADR 0021). All of them left out together clears the Reading, taking the line back to fully unread. The target is either a Food's written word or — as `lineage_id` — the Recipe this line names, which makes the Ingredient a Component (ADR 0008); never both, and a Lineage this instance does not hold is accepted, because a Component goes on naming its recipe when the recipe is gone. */
export type SetReadingInput = {
	amount?: string | null;
	branch_id: string;
	line_index: number;
	lineage_id?: string | null;
	target?: string | null;
	unit?: string | null;
};
/** What set_reading answers. */
export type SetReadingOutput = {
	line_index: number;
	measured: string | null;
	reading: {
		amount: string | null;
		lineage_id: string | null;
		target: string | null;
		unit: string | null;
	} | null;
};

/** Read every Ingredient Line in the library that nothing has read yet, as a Job, laying a Reading over each one Kamosu can make sense of. Touches no written line and makes no Version. A line already carrying a Reading is left alone, so a correction is never overwritten, and a line Kamosu cannot read is left unread, which is an ordinary state for a line rather than a failure. Kamosu also reads the lines of every recipe as it is written or imported, so this is for a library that predates it. */
export type ReadIngredientLinesInput = Record<string, never>;
/** What read_ingredient_lines eventually produces, read back through `get_job`. */
export type ReadIngredientLinesOutput = {
	read: number;
};

/** Start cooking a Recipe: creates the Attempt, or hands back the one already In Progress for this Lineage — the cooking screen is that Attempt, never a second thing beside it. Pinned by fingerprint to the Branch's head Version at this moment, or to version_id — an older Version read back from the Thread — when one is given. Anyone who can see the recipe may. */
export type StartAttemptInput = {
	attempt_id?: string;
	branch_id: string;
	started_at?: string;
	version_id?: string;
};
/** What start_attempt answers. */
export type StartAttemptOutput = {
	as_cooked: {
		against: {
			ingredients: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
			steps: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
		};
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		promotion_declined: boolean;
		version_id: string;
	} | null;
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
	photographs: string[];
	rating: "again" | "tweak" | "no" | null;
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
	written_at?: string;
};
/** What advance_attempt answers. */
export type AdvanceAttemptOutput = {
	as_cooked: {
		against: {
			ingredients: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
			steps: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
		};
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		promotion_declined: boolean;
		version_id: string;
	} | null;
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
	photographs: string[];
	rating: "again" | "tweak" | "no" | null;
	resumable: boolean;
	ticked_ingredients: number[];
	version_id: string;
};

/** End an In Progress Attempt, taking the judgement that lands with it: a rating, a note and Photographs, all optional. Ending is not what makes the cooking real — starting already did — only what stops it being In Progress, so a cook who says nothing still cooked. */
export type FinishAttemptInput = {
	add_photographs?: string[] | null;
	attempt_id: string;
	note?: string | null;
	photographs?: string[] | null;
	rating?: "again" | "tweak" | "no" | null;
	written_at?: string;
};
/** What finish_attempt answers. */
export type FinishAttemptOutput = {
	as_cooked: {
		against: {
			ingredients: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
			steps: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
		};
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		promotion_declined: boolean;
		version_id: string;
	} | null;
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
	photographs: string[];
	rating: "again" | "tweak" | "no" | null;
	resumable: boolean;
	ticked_ingredients: number[];
	version_id: string;
};

/** Change an Attempt's free text, its rating or its Photographs, whether it is still In Progress or long finished — an Attempt is freely editable by its cook, unlike the recipe it was cooked from. */
export type EditAttemptInput = {
	add_photographs?: string[] | null;
	attempt_id: string;
	note?: string | null;
	photographs?: string[] | null;
	rating?: "again" | "tweak" | "no" | null;
	written_at?: string;
};
/** What edit_attempt answers. */
export type EditAttemptOutput = {
	as_cooked: {
		against: {
			ingredients: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
			steps: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
		};
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		promotion_declined: boolean;
		version_id: string;
	} | null;
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
	photographs: string[];
	rating: "again" | "tweak" | "no" | null;
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

/** Make a picture taken while cooking the recipe's Main Photo, or a Step's photo — so the picture you actually took becomes the recipe's picture. This is an ordinary edit making a Version, with everything that follows from it: a rapid re-save folding into the Version already being shaped, and a Copy in your own Cookbook where you do not write the Branch's. The Branch must be one you may see. The Attempt keeps the picture too; promoting is not moving. */
export type PromoteAttemptPhotographInput = {
	attempt_id: string;
	branch_id: string;
	change_note?: string | null;
	kitchen_id?: string;
	photograph_id: string;
	step_index?: number | null;
};
/** What promote_attempt_photograph answers. */
export type PromoteAttemptPhotographOutput = {
	branch_id: string;
	collapsed: boolean;
	copied: boolean;
	language: string;
	language_offer: string | null;
	parent_version_id: string | null;
	sequence: number;
	translates_version_id: string | null;
	version_id: string;
};

/** Write down what you actually cooked, where it differed from the recipe: the whole recipe as you cooked it, in ordinary Ingredient Lines and ordinary Step text — a line reworded, one added, one dropped, a step grown. Not a record of differences; the same shape a Version takes. Sending back exactly what the recipe says, or null, stores nothing at all, because cooking a recipe as it is written changes nothing. Changes no recipe and makes no Version: that is Promotion, and it is a separate act. */
export type SetAsCookedInput = {
	as_cooked: {
		cook_time_minutes?: number | null;
		ingredients?: {
			kind: "section" | "ingredient";
			text: string;
		}[];
		main_photo?: string | null;
		note?: string | null;
		nutrition?: {
			basis: "per_serving" | "per_100g";
			calories: number;
		} | null;
		prep_time_minutes?: number | null;
		source?: {
			link: string | null;
			text: string;
		} | null;
		steps?: {
			kind: "section" | "step";
			photo?: string | null;
			text: string;
		}[];
		title: string;
		yield?: {
			amount: string;
			noun: string;
		} | null;
	} | null;
	attempt_id: string;
	written_at?: string;
};
/** What set_as_cooked answers. */
export type SetAsCookedOutput = {
	as_cooked: {
		against: {
			ingredients: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
			steps: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
		};
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		promotion_declined: boolean;
		version_id: string;
	} | null;
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
	photographs: string[];
	rating: "again" | "tweak" | "no" | null;
	resumable: boolean;
	ticked_ingredients: number[];
	version_id: string;
};

/** Say that the words a cooking used belong in the diary and not in the recipe — or take that back. It answers the offer and nothing else: what was cooked stays on the cooking, whole. Remembered, because a question already answered, asked twice, is a nag. */
export type DeclinePromotionInput = {
	attempt_id: string;
	declined: boolean;
};
/** What decline_promotion answers. */
export type DeclinePromotionOutput = {
	as_cooked: {
		against: {
			ingredients: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
			steps: {
				from_branch_point: boolean;
				kind: string;
				mine: {
					index: number;
					kind: string;
					text: string;
				} | null;
				state: "same" | "changed" | "only-mine" | "only-theirs";
				theirs: {
					index: number;
					kind: string;
					text: string;
				} | null;
			}[];
		};
		content: {
			cook_time_minutes: number | null;
			ingredients: {
				kind: "section" | "ingredient";
				text: string;
			}[];
			main_photo: string | null;
			note: string | null;
			nutrition: {
				basis: "per_serving" | "per_100g";
				calories: number;
			} | null;
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
		promotion_declined: boolean;
		version_id: string;
	} | null;
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
	photographs: string[];
	rating: "again" | "tweak" | "no" | null;
	resumable: boolean;
	ticked_ingredients: number[];
	version_id: string;
};

/** Promotion: turn what you cooked into a real Version of the recipe. Mechanical — the As Cooked is already a whole recipe, so nothing is retyped and nothing is reconciled. It is an ordinary edit and inherits all of one: a rapid re-save folds into the Version being shaped, and a Branch whose Cookbook you do not write becomes a Copy in your own. The Branch must be one you may see. Promoting a cooking of an older Version appends onto wherever the Branch stands now — a Version, never a merge. The Attempt is left exactly as it was, still saying which Version it cooked. */
export type PromoteAsCookedInput = {
	attempt_id: string;
	branch_id: string;
	change_note?: string | null;
	kitchen_id?: string;
	name?: string | null;
};
/** What promote_as_cooked answers. */
export type PromoteAsCookedOutput = {
	branch_id: string;
	collapsed: boolean;
	copied: boolean;
	language: string;
	language_offer: string | null;
	parent_version_id: string | null;
	sequence: number;
	translates_version_id: string | null;
	version_id: string;
};

/** Read the caller's own In Progress Attempt for a Lineage, if any — how two devices cooking the same dish stay in step, and whether resuming should still be offered. */
export type GetCurrentAttemptInput = {
	lineage_id: string;
};
/** What get_current_attempt answers. */
export type GetCurrentAttemptOutput = {
	attempt: {
		as_cooked: {
			against: {
				ingredients: {
					from_branch_point: boolean;
					kind: string;
					mine: {
						index: number;
						kind: string;
						text: string;
					} | null;
					state: "same" | "changed" | "only-mine" | "only-theirs";
					theirs: {
						index: number;
						kind: string;
						text: string;
					} | null;
				}[];
				steps: {
					from_branch_point: boolean;
					kind: string;
					mine: {
						index: number;
						kind: string;
						text: string;
					} | null;
					state: "same" | "changed" | "only-mine" | "only-theirs";
					theirs: {
						index: number;
						kind: string;
						text: string;
					} | null;
				}[];
			};
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			promotion_declined: boolean;
			version_id: string;
		} | null;
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
		photographs: string[];
		rating: "again" | "tweak" | "no" | null;
		resumable: boolean;
		ticked_ingredients: number[];
		version_id: string;
	} | null;
};

/** The cooking diary: every Attempt the caller has made, newest first, across every recipe — sorted by date rather than by recipe, which is what makes *what did I cook that week* answerable. Unfinished and In Progress cookings are in it too, because starting is what makes a cooking real. Each entry names the recipe it was cooked from, and still names it after that recipe has left the caller's shelf. */
export type ListAttemptsInput = Record<string, never>;
/** What list_attempts answers. */
export type ListAttemptsOutput = {
	attempts: {
		as_cooked: {
			against: {
				ingredients: {
					from_branch_point: boolean;
					kind: string;
					mine: {
						index: number;
						kind: string;
						text: string;
					} | null;
					state: "same" | "changed" | "only-mine" | "only-theirs";
					theirs: {
						index: number;
						kind: string;
						text: string;
					} | null;
				}[];
				steps: {
					from_branch_point: boolean;
					kind: string;
					mine: {
						index: number;
						kind: string;
						text: string;
					} | null;
					state: "same" | "changed" | "only-mine" | "only-theirs";
					theirs: {
						index: number;
						kind: string;
						text: string;
					} | null;
				}[];
			};
			content: {
				cook_time_minutes: number | null;
				ingredients: {
					kind: "section" | "ingredient";
					text: string;
				}[];
				main_photo: string | null;
				note: string | null;
				nutrition: {
					basis: "per_serving" | "per_100g";
					calories: number;
				} | null;
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
			promotion_declined: boolean;
			version_id: string;
		} | null;
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
		photographs: string[];
		rating: "again" | "tweak" | "no" | null;
		recipe: {
			branch_id: string | null;
			title: string;
			written_yield: {
				amount: string;
				noun: string;
			} | null;
		};
		resumable: boolean;
		ticked_ingredients: number[];
		version_id: string;
	}[];
};

/** Your Shopping List: the recipes you chose, and the rows worked out from them. Everyone has exactly one; it has no name and is never archived. The rows are computed on every read and stored nowhere, so editing a chosen recipe or correcting a Reading changes the list at once. A row names a Food in your Reading Language and merges every mention of it; amounts add where the Units honestly convert, saying about, and ride side by side where they do not. Nothing here is ticked off. */
export type GetShoppingListInput = Record<string, never>;
/** What get_shopping_list answers. */
export type GetShoppingListOutput = {
	chosen: {
		branch_id: string;
		gone: boolean;
		shopping_yield: {
			amount: string;
			noun: string;
		} | null;
		title: string;
		written_yield: {
			amount: string;
			noun: string;
		} | null;
	}[];
	rows: {
		id: string;
		kind: "food" | "line" | "loose";
		lines: {
			branch_id: string;
			recipe: string;
			text: string;
		}[];
		name: string;
		name_language: string | null;
		parts: {
			kind: "about" | "count" | "as_written" | "no_amount";
			sources: string[];
			text: string;
		}[];
		said: string | null;
	}[];
};

/** What one recipe puts on a Shopping List before anything is added up: each Ingredient Line, the Food it was read as and the name that Food goes by for you, how much it said, its Unit, and what a cup of the Food weighs. Every recipe this one includes is unfolded to the bottom and its lines are here too, already carrying their share, so a pizza's flour and its dough's flour add up to one thing to buy. Always the Branch's latest Version. It is how a phone with no network works out the list's rows itself for the recipes it holds (#77); get_shopping_list is the list itself. */
export type ShoppingBasisInput = {
	branch_id: string;
};
/** What shopping_basis answers. */
export type ShoppingBasisOutput = {
	branch_id: string;
	lines: {
		food: {
			amount: number | null;
			cup_weight_grams: number | null;
			id: string;
			name: string | null;
			name_language: string | null;
			unit: string | null;
			unit_id: string | null;
			unit_key: string | null;
		} | null;
		from: {
			branch_id: string;
			title: string;
		} | null;
		path: number[];
		said: string | null;
		text: string;
	}[];
	title: string;
	written_yield: {
		amount: string;
		noun: string;
	} | null;
};

/** Choose a recipe to shop for, at a Yield, a multiplier (a Yield with an empty noun) or as it is written. It holds the Branch at its latest Version, never a Lineage and never pinned, so a recipe edited between the planning and the shopping is right in the shop. Choosing one already on the list is not an error and makes no second entry: it moves that entry to the Yield given here, or back to the recipe as written when none is. Answers the whole list. */
export type AddToShoppingListInput = {
	branch_id: string;
	shopping_yield?: {
		amount: string;
		noun: string;
	} | null;
	written_at?: string;
};
/** What add_to_shopping_list answers. */
export type AddToShoppingListOutput = {
	chosen: {
		branch_id: string;
		gone: boolean;
		shopping_yield: {
			amount: string;
			noun: string;
		} | null;
		title: string;
		written_yield: {
			amount: string;
			noun: string;
		} | null;
	}[];
	rows: {
		id: string;
		kind: "food" | "line" | "loose";
		lines: {
			branch_id: string;
			recipe: string;
			text: string;
		}[];
		name: string;
		name_language: string | null;
		parts: {
			kind: "about" | "count" | "as_written" | "no_amount";
			sources: string[];
			text: string;
		}[];
		said: string | null;
	}[];
};

/** Take a recipe off your Shopping List. Works whether or not it can still be read, which is exactly the entry somebody most wants gone. Answers the whole list. */
export type RemoveFromShoppingListInput = {
	branch_id: string;
	written_at?: string;
};
/** What remove_from_shopping_list answers. */
export type RemoveFromShoppingListOutput = {
	chosen: {
		branch_id: string;
		gone: boolean;
		shopping_yield: {
			amount: string;
			noun: string;
		} | null;
		title: string;
		written_yield: {
			amount: string;
			noun: string;
		} | null;
	}[];
	rows: {
		id: string;
		kind: "food" | "line" | "loose";
		lines: {
			branch_id: string;
			recipe: string;
			text: string;
		}[];
		name: string;
		name_language: string | null;
		parts: {
			kind: "about" | "count" | "as_written" | "no_amount";
			sources: string[];
			text: string;
		}[];
		said: string | null;
	}[];
};

/** Say how much of a chosen recipe you are shopping for — an amount and its noun, a multiplier (an amount with an empty noun: twice the recipe is `2`), or null for the recipe as written. Every amount it contributes moves with it. Answers the whole list. */
export type SetShoppingYieldInput = {
	branch_id: string;
	shopping_yield?: {
		amount: string;
		noun: string;
	} | null;
	written_at?: string;
};
/** What set_shopping_yield answers. */
export type SetShoppingYieldOutput = {
	chosen: {
		branch_id: string;
		gone: boolean;
		shopping_yield: {
			amount: string;
			noun: string;
		} | null;
		title: string;
		written_yield: {
			amount: string;
			noun: string;
		} | null;
	}[];
	rows: {
		id: string;
		kind: "food" | "line" | "loose";
		lines: {
			branch_id: string;
			recipe: string;
			text: string;
		}[];
		name: string;
		name_language: string | null;
		parts: {
			kind: "about" | "count" | "as_written" | "no_amount";
			sources: string[];
			text: string;
		}[];
		said: string | null;
	}[];
};

/** Your Shopping List as plain text, ready to be carried out of Kamosu. Nothing is ticked off here, because the list leaves and something else holds the ticks — Apple Notes, through a Shortcut. The text opens with a header line, the date and the recipes it was built from (and any that can no longer be read), because a note accumulates and three trips appended with no divider are a wall. Under it, one flat alphabetical list with one Markdown checklist line (`- [ ] `) per thing to buy, so each line becomes one checkbox; a row whose amounts could not be added stays on its one line, naming the dish behind each amount. This only reads: emptying the list afterwards is a separate Operation, offered and never done on the way out. */
export type ShoppingListAsTextInput = Record<string, never>;
/** What shopping_list_as_text answers. */
export type ShoppingListAsTextOutput = {
	text: string;
};

/** Empty your Shopping List — every recipe chosen and every typed line at once. Offered after the list has left as text and never done on the way out: a list that emptied itself when it was sent would be silent and unrecoverable. Answers the whole list. */
export type EmptyShoppingListInput = {
	written_at?: string;
};
/** What empty_shopping_list answers. */
export type EmptyShoppingListOutput = {
	chosen: {
		branch_id: string;
		gone: boolean;
		shopping_yield: {
			amount: string;
			noun: string;
		} | null;
		title: string;
		written_yield: {
			amount: string;
			noun: string;
		} | null;
	}[];
	rows: {
		id: string;
		kind: "food" | "line" | "loose";
		lines: {
			branch_id: string;
			recipe: string;
			text: string;
		}[];
		name: string;
		name_language: string | null;
		parts: {
			kind: "about" | "count" | "as_written" | "no_amount";
			sources: string[];
			text: string;
		}[];
		said: string | null;
	}[];
};

/** Type a line straight onto your Shopping List — bin bags, coffee. Kept exactly as typed and never read, so it carries no amount and merges with nothing: typing flour beside a recipe that wants flour gives two lines. Answers the whole list. */
export type AddLooseItemInput = {
	item_id?: string;
	text: string;
	written_at?: string;
};
/** What add_loose_item answers. */
export type AddLooseItemOutput = {
	chosen: {
		branch_id: string;
		gone: boolean;
		shopping_yield: {
			amount: string;
			noun: string;
		} | null;
		title: string;
		written_yield: {
			amount: string;
			noun: string;
		} | null;
	}[];
	rows: {
		id: string;
		kind: "food" | "line" | "loose";
		lines: {
			branch_id: string;
			recipe: string;
			text: string;
		}[];
		name: string;
		name_language: string | null;
		parts: {
			kind: "about" | "count" | "as_written" | "no_amount";
			sources: string[];
			text: string;
		}[];
		said: string | null;
	}[];
};

/** Take one typed line off your Shopping List. Answers the whole list. */
export type RemoveLooseItemInput = {
	item_id: string;
	written_at?: string;
};
/** What remove_loose_item answers. */
export type RemoveLooseItemOutput = {
	chosen: {
		branch_id: string;
		gone: boolean;
		shopping_yield: {
			amount: string;
			noun: string;
		} | null;
		title: string;
		written_yield: {
			amount: string;
			noun: string;
		} | null;
	}[];
	rows: {
		id: string;
		kind: "food" | "line" | "loose";
		lines: {
			branch_id: string;
			recipe: string;
			text: string;
		}[];
		name: string;
		name_language: string | null;
		parts: {
			kind: "about" | "count" | "as_written" | "no_amount";
			sources: string[];
			text: string;
		}[];
		said: string | null;
	}[];
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

/** The Operator's worklist: every note that two Foods are probably one thing, with the words that said so. Evidence, never an instruction — nothing merges itself. */
export type ListMergeSuggestionsInput = Record<string, never>;
/** What list_merge_suggestions answers. */
export type ListMergeSuggestionsOutput = {
	suggestions: {
		created_at: string;
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
		reason: "arrived_as_one" | "name_typed_onto_another";
		words: {
			language: string;
			name: string;
		}[];
	}[];
};

/** Say how many Ingredient Lines a Merge would move, and how many Reading rows, without moving any of them. A Merge cannot be undone and refuses to run until this figure is said back to it, so this saying is its safety net rather than a courtesy. */
export type PreviewFoodMergeInput = {
	absorbed_food_id: string;
	survivor_food_id: string;
};
/** What preview_food_merge answers. */
export type PreviewFoodMergeOutput = {
	absorbed: {
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
	cup_weight_conflict: boolean;
	ingredient_lines: number;
	readings: number;
	survivor: {
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
};

/** Join two Foods into one: the survivor takes every name both had, every Reading pointing at the other points at it instead, and every Merge Suggestion naming either is cleared. ingredient_lines is the figure preview_food_merge announced, said back — a Merge that does not match it is refused. Where the two disagree about Cup Weight, cup_weight_grams says which of the two figures survives. There is no un-merge in v1. */
export type MergeFoodInput = {
	absorbed_food_id: string;
	cup_weight_grams?: number | null;
	ingredient_lines: number;
	survivor_food_id: string;
};
/** What merge_food answers. */
export type MergeFoodOutput = {
	food: {
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
	ingredient_lines: number;
	readings: number;
};

/** Delete a Food nothing points at. One a Reading still points at is refused: what a Food knows was expensive to learn and is never discarded by an unrelated act. */
export type DeleteFoodInput = {
	food_id: string;
};
/** What delete_food answers. */
export type DeleteFoodOutput = {
	deleted: boolean;
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
	get_reading_preferences: {
		input: GetReadingPreferencesInput;
		output: GetReadingPreferencesOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_person: {
		input: GetPersonInput;
		output: GetPersonOutput;
		kind: 'immediate';
		permission: 'person';
	};
	rename_person: {
		input: RenamePersonInput;
		output: RenamePersonOutput;
		kind: 'immediate';
		permission: 'person';
	};
	list_accounts: {
		input: ListAccountsInput;
		output: ListAccountsOutput;
		kind: 'immediate';
		permission: 'operator';
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
	set_operator: {
		input: SetOperatorInput;
		output: SetOperatorOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	sweep_photographs: {
		input: SweepPhotographsInput;
		output: SweepPhotographsOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	take_backup: {
		input: TakeBackupInput;
		output: TakeBackupOutput;
		kind: 'job';
		permission: 'operator';
	};
	list_backups: {
		input: ListBackupsInput;
		output: ListBackupsOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	list_sessions: {
		input: ListSessionsInput;
		output: ListSessionsOutput;
		kind: 'immediate';
		permission: 'person';
	};
	rename_session: {
		input: RenameSessionInput;
		output: RenameSessionOutput;
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
	preview_leaving_kitchen: {
		input: PreviewLeavingKitchenInput;
		output: PreviewLeavingKitchenOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_cookbook: {
		input: GetCookbookInput;
		output: GetCookbookOutput;
		kind: 'immediate';
		permission: 'person';
	};
	rename_cookbook: {
		input: RenameCookbookInput;
		output: RenameCookbookOutput;
		kind: 'immediate';
		permission: 'person';
	};
	invite_to_cookbook: {
		input: InviteToCookbookInput;
		output: InviteToCookbookOutput;
		kind: 'immediate';
		permission: 'person';
	};
	cancel_cookbook_invite: {
		input: CancelCookbookInviteInput;
		output: CancelCookbookInviteOutput;
		kind: 'immediate';
		permission: 'person';
	};
	read_cookbook_invite: {
		input: ReadCookbookInviteInput;
		output: ReadCookbookInviteOutput;
		kind: 'immediate';
		permission: 'person';
	};
	accept_cookbook_invite: {
		input: AcceptCookbookInviteInput;
		output: AcceptCookbookInviteOutput;
		kind: 'immediate';
		permission: 'person';
	};
	leave_cookbook: {
		input: LeaveCookbookInput;
		output: LeaveCookbookOutput;
		kind: 'immediate';
		permission: 'person';
	};
	remove_cookbook_author: {
		input: RemoveCookbookAuthorInput;
		output: RemoveCookbookAuthorOutput;
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
	start_variation: {
		input: StartVariationInput;
		output: StartVariationOutput;
		kind: 'immediate';
		permission: 'person';
	};
	rename_branch: {
		input: RenameBranchInput;
		output: RenameBranchOutput;
		kind: 'immediate';
		permission: 'person';
	};
	delete_recipe: {
		input: DeleteRecipeInput;
		output: DeleteRecipeOutput;
		kind: 'immediate';
		permission: 'person';
	};
	read_pasted_recipe: {
		input: ReadPastedRecipeInput;
		output: ReadPastedRecipeOutput;
		kind: 'immediate';
		permission: 'person';
	};
	start_translation: {
		input: StartTranslationInput;
		output: StartTranslationOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_recipe_language: {
		input: SetRecipeLanguageInput;
		output: SetRecipeLanguageOutput;
		kind: 'immediate';
		permission: 'person';
	};
	import: {
		input: ImportInput;
		output: ImportOutput;
		kind: 'job';
		permission: 'person';
	};
	import_crouton: {
		input: ImportCroutonInput;
		output: ImportCroutonOutput;
		kind: 'job';
		permission: 'person';
	};
	list_imports: {
		input: ListImportsInput;
		output: ListImportsOutput;
		kind: 'immediate';
		permission: 'person';
	};
	forget_import: {
		input: ForgetImportInput;
		output: ForgetImportOutput;
		kind: 'immediate';
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
	search_recipes: {
		input: SearchRecipesInput;
		output: SearchRecipesOutput;
		kind: 'immediate';
		permission: 'person';
	};
	home_shelves: {
		input: HomeShelvesInput;
		output: HomeShelvesOutput;
		kind: 'immediate';
		permission: 'person';
	};
	note_recipe_opened: {
		input: NoteRecipeOpenedInput;
		output: NoteRecipeOpenedOutput;
		kind: 'immediate';
		permission: 'person';
	};
	meaning_search_status: {
		input: MeaningSearchStatusInput;
		output: MeaningSearchStatusOutput;
		kind: 'immediate';
		permission: 'person';
	};
	accept_meaning_search_terms: {
		input: AcceptMeaningSearchTermsInput;
		output: AcceptMeaningSearchTermsOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	decline_meaning_search: {
		input: DeclineMeaningSearchInput;
		output: DeclineMeaningSearchOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	download_meaning_model: {
		input: DownloadMeaningModelInput;
		output: DownloadMeaningModelOutput;
		kind: 'job';
		permission: 'operator';
	};
	build_meaning_index: {
		input: BuildMeaningIndexInput;
		output: BuildMeaningIndexOutput;
		kind: 'job';
		permission: 'operator';
	};
	turn_off_meaning_search: {
		input: TurnOffMeaningSearchInput;
		output: TurnOffMeaningSearchOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	get_recipe: {
		input: GetRecipeInput;
		output: GetRecipeOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_thread: {
		input: GetThreadInput;
		output: GetThreadOutput;
		kind: 'immediate';
		permission: 'person';
	};
	share_recipe: {
		input: ShareRecipeInput;
		output: ShareRecipeOutput;
		kind: 'immediate';
		permission: 'person';
	};
	end_share_link: {
		input: EndShareLinkInput;
		output: EndShareLinkOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_share_link: {
		input: GetShareLinkInput;
		output: GetShareLinkOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_public_address: {
		input: GetPublicAddressInput;
		output: GetPublicAddressOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	set_public_address: {
		input: SetPublicAddressInput;
		output: SetPublicAddressOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	export_bundle: {
		input: ExportBundleInput;
		output: ExportBundleOutput;
		kind: 'immediate';
		permission: 'person';
	};
	make_sheet: {
		input: MakeSheetInput;
		output: MakeSheetOutput;
		kind: 'job';
		permission: 'person';
	};
	make_shared_sheet: {
		input: MakeSharedSheetInput;
		output: MakeSharedSheetOutput;
		kind: 'job';
		permission: 'public';
	};
	import_bundle: {
		input: ImportBundleInput;
		output: ImportBundleOutput;
		kind: 'job';
		permission: 'person';
	};
	read_shared_recipe: {
		input: ReadSharedRecipeInput;
		output: ReadSharedRecipeOutput;
		kind: 'immediate';
		permission: 'public';
	};
	branch_point: {
		input: BranchPointInput;
		output: BranchPointOutput;
		kind: 'immediate';
		permission: 'person';
	};
	divergence: {
		input: DivergenceInput;
		output: DivergenceOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_reading: {
		input: SetReadingInput;
		output: SetReadingOutput;
		kind: 'immediate';
		permission: 'person';
	};
	read_ingredient_lines: {
		input: ReadIngredientLinesInput;
		output: ReadIngredientLinesOutput;
		kind: 'job';
		permission: 'operator';
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
	promote_attempt_photograph: {
		input: PromoteAttemptPhotographInput;
		output: PromoteAttemptPhotographOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_as_cooked: {
		input: SetAsCookedInput;
		output: SetAsCookedOutput;
		kind: 'immediate';
		permission: 'person';
	};
	decline_promotion: {
		input: DeclinePromotionInput;
		output: DeclinePromotionOutput;
		kind: 'immediate';
		permission: 'person';
	};
	promote_as_cooked: {
		input: PromoteAsCookedInput;
		output: PromoteAsCookedOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_current_attempt: {
		input: GetCurrentAttemptInput;
		output: GetCurrentAttemptOutput;
		kind: 'immediate';
		permission: 'person';
	};
	list_attempts: {
		input: ListAttemptsInput;
		output: ListAttemptsOutput;
		kind: 'immediate';
		permission: 'person';
	};
	get_shopping_list: {
		input: GetShoppingListInput;
		output: GetShoppingListOutput;
		kind: 'immediate';
		permission: 'person';
	};
	shopping_basis: {
		input: ShoppingBasisInput;
		output: ShoppingBasisOutput;
		kind: 'immediate';
		permission: 'person';
	};
	add_to_shopping_list: {
		input: AddToShoppingListInput;
		output: AddToShoppingListOutput;
		kind: 'immediate';
		permission: 'person';
	};
	remove_from_shopping_list: {
		input: RemoveFromShoppingListInput;
		output: RemoveFromShoppingListOutput;
		kind: 'immediate';
		permission: 'person';
	};
	set_shopping_yield: {
		input: SetShoppingYieldInput;
		output: SetShoppingYieldOutput;
		kind: 'immediate';
		permission: 'person';
	};
	shopping_list_as_text: {
		input: ShoppingListAsTextInput;
		output: ShoppingListAsTextOutput;
		kind: 'immediate';
		permission: 'person';
	};
	empty_shopping_list: {
		input: EmptyShoppingListInput;
		output: EmptyShoppingListOutput;
		kind: 'immediate';
		permission: 'person';
	};
	add_loose_item: {
		input: AddLooseItemInput;
		output: AddLooseItemOutput;
		kind: 'immediate';
		permission: 'person';
	};
	remove_loose_item: {
		input: RemoveLooseItemInput;
		output: RemoveLooseItemOutput;
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
	list_merge_suggestions: {
		input: ListMergeSuggestionsInput;
		output: ListMergeSuggestionsOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	preview_food_merge: {
		input: PreviewFoodMergeInput;
		output: PreviewFoodMergeOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	merge_food: {
		input: MergeFoodInput;
		output: MergeFoodOutput;
		kind: 'immediate';
		permission: 'operator';
	};
	delete_food: {
		input: DeleteFoodInput;
		output: DeleteFoodOutput;
		kind: 'immediate';
		permission: 'operator';
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
		"summary": "The version of this Kamosu, whether setup has happened, and the shortest password it accepts.",
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
				"password_minimum": {
					"minimum": 1,
					"type": "integer"
				},
				"setup_complete": {
					"type": "boolean"
				},
				"version": {
					"type": "string"
				}
			},
			"required": [
				"version",
				"setup_complete",
				"password_minimum"
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
		}
	},
	{
		"name": "get_reading_preferences",
		"summary": "The Language and measures this Person reads in. Reading Measures live on the account rather than in a browser, so every Door and every device reads the same recipe the same way; the default is American, a stated convention rather than a guess about anybody.",
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
		}
	},
	{
		"name": "get_person",
		"summary": "Who this Credential names: the Person's permanent id, which is also their Hand, and the name they currently go by — the name every Version they wrote shows here, and the one they sign in with.",
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
		}
	},
	{
		"name": "rename_person",
		"summary": "Change this Person's current reminder name. Every Version they ever wrote shows the new one on this instance, since a Hand is named live and nothing is keyed on the name; no id or fingerprint moves. It is also the name they sign in with, so a name somebody else here signs in with is refused. A Bundle already sent keeps the name it left with.",
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
		"name": "list_accounts",
		"summary": "Who holds an account on this instance: their name, whether they administer it, and whether the account is disabled. Nothing about what they cook — the Operator administers and does not read (ADR 0007), so no recipe, Attempt, Cookbook or Kitchen of theirs is reachable from here.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"accounts": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
							"disabled": {
								"type": "boolean"
							},
							"is_operator": {
								"type": "boolean"
							},
							"is_you": {
								"type": "boolean"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"name",
							"is_operator",
							"disabled",
							"is_you",
							"created_at"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"accounts"
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
		"summary": "Delete an account while preserving its Hand in history. The Person's name is freed for somebody new to sign in with; what they wrote keeps their Hand and the name they had. Disabling an account keeps the name.",
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
		"name": "set_operator",
		"summary": "Make a Person an Operator, or stop them being one. The last Operator cannot be demoted (ADR 0007): an instance with nobody to administer it can never get one back, so the refusal is the point rather than a nicety.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"is_operator": {
					"type": "boolean"
				},
				"name": {
					"type": "string"
				}
			},
			"required": [
				"name",
				"is_operator"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"is_operator": {
					"type": "boolean"
				},
				"name": {
					"type": "string"
				}
			},
			"required": [
				"name",
				"is_operator"
			],
			"type": "object"
		}
	},
	{
		"name": "sweep_photographs",
		"summary": "Take away Photographs nothing has pointed at for a week, and their Display Copies with them. Runs daily on its own; this asks for it now.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"back_in_use": {
					"type": "integer"
				},
				"newly_unreferenced": {
					"type": "integer"
				},
				"referenced": {
					"type": "integer"
				},
				"swept": {
					"type": "integer"
				},
				"swept_photograph_ids": {
					"items": {
						"type": "string"
					},
					"type": "array"
				}
			},
			"required": [
				"referenced",
				"newly_unreferenced",
				"back_in_use",
				"swept",
				"swept_photograph_ids"
			],
			"type": "object"
		}
	},
	{
		"name": "take_backup",
		"summary": "Take a Backup now, as a Job: one archive holding a consistent copy of the database and every Photograph, written beside the database under /data. Kamosu keeps three — one taken daily, one weekly, one monthly — and takes them on its own; this asks for one now. A Job because an archive is the size of the library. It is never sent anywhere: fetch the bytes at GET /api/backups/<name>.",
		"permission": "operator",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"backups": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"name": {
								"type": "string"
							},
							"size_bytes": {
								"type": "integer"
							},
							"slot": {
								"enum": [
									"daily",
									"weekly",
									"monthly"
								]
							},
							"taken_at": {
								"type": "string"
							}
						},
						"required": [
							"name",
							"slot",
							"taken_at",
							"size_bytes"
						],
						"type": "object"
					},
					"type": "array"
				},
				"taken": {
					"items": {
						"type": "string"
					},
					"type": "array"
				}
			},
			"required": [
				"taken",
				"backups"
			],
			"type": "object"
		}
	},
	{
		"name": "list_backups",
		"summary": "The Backups this instance holds, newest first. Each can be fetched at GET /api/backups/<name>, under the same Credential as any Operation.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"backups": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"name": {
								"type": "string"
							},
							"size_bytes": {
								"type": "integer"
							},
							"slot": {
								"enum": [
									"daily",
									"weekly",
									"monthly"
								]
							},
							"taken_at": {
								"type": "string"
							}
						},
						"required": [
							"name",
							"slot",
							"taken_at",
							"size_bytes"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"backups"
			],
			"type": "object"
		}
	},
	{
		"name": "list_sessions",
		"summary": "List this Person's browser Sessions by device and last use. `current` marks the Session asking, so it is never set when an Access Key asks.",
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
							"current": {
								"type": "boolean"
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
							"revoked",
							"current"
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
		"name": "rename_session",
		"summary": "Rename one of your browser Sessions. A Session is named for its device when it signs in; this corrects the guess or names an older one. An ended Session is not renamed.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"name": {
					"type": "string"
				},
				"session_id": {
					"type": "string"
				}
			},
			"required": [
				"session_id",
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
				}
			},
			"required": [
				"id",
				"name"
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
		"summary": "Create a Kitchen: a group of People who see and cook from each other's Cookbooks. Its creator is its first member.",
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
				"cookbooks": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"authors": {
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
							"id": {
								"type": "string"
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"name",
							"authors"
						],
						"type": "object"
					},
					"type": "array"
				},
				"id": {
					"type": "string"
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
				"nickname",
				"members",
				"cookbooks"
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
							"cookbooks": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"authors": {
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
										"id": {
											"type": "string"
										},
										"name": {
											"type": [
												"string",
												"null"
											]
										}
									},
									"required": [
										"id",
										"name",
										"authors"
									],
									"type": "object"
								},
								"type": "array"
							},
							"id": {
								"type": "string"
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
							"nickname",
							"members",
							"cookbooks"
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
				"cookbooks": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"authors": {
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
							"id": {
								"type": "string"
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"name",
							"authors"
						],
						"type": "object"
					},
					"type": "array"
				},
				"id": {
					"type": "string"
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
				"nickname",
				"members",
				"cookbooks"
			],
			"type": "object"
		}
	},
	{
		"name": "remove_kitchen_member",
		"summary": "Remove a Person from a Kitchen — including yourself, to leave. Their Cookbook leaves with them; each member who stays keeps a Branch of every recipe of theirs they cooked, and they keep one of every recipe they cooked from the others.",
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
		"name": "preview_leaving_kitchen",
		"summary": "What removing a Person from a Kitchen would leave each side, before anybody does it: how many recipes the members who stay keep, and how many the one leaving keeps. `person_id` defaults to you.",
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
				"kitchen_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"they_keep": {
					"type": "integer"
				},
				"you_keep": {
					"type": "integer"
				}
			},
			"required": [
				"they_keep",
				"you_keep"
			],
			"type": "object"
		}
	},
	{
		"name": "get_cookbook",
		"summary": "Your own Cookbook: its name, who writes it, how many recipes it holds, the Kitchens that see it and the Invites still waiting.",
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
				"authors": {
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
				"id": {
					"type": "string"
				},
				"invites": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
							"invite_id": {
								"type": "string"
							}
						},
						"required": [
							"invite_id",
							"created_at"
						],
						"type": "object"
					},
					"type": "array"
				},
				"kitchens": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"id",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"recipe_count": {
					"type": "integer"
				}
			},
			"required": [
				"id",
				"name",
				"authors",
				"recipe_count",
				"kitchens",
				"invites"
			],
			"type": "object"
		}
	},
	{
		"name": "rename_cookbook",
		"summary": "Give your Cookbook a name of its own, or clear it back to its Co-authors' names with an empty or null one. Any Co-author may.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
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
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"authors": {
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
				"id": {
					"type": "string"
				},
				"invites": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
							"invite_id": {
								"type": "string"
							}
						},
						"required": [
							"invite_id",
							"created_at"
						],
						"type": "object"
					},
					"type": "array"
				},
				"kitchens": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"id",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"recipe_count": {
					"type": "integer"
				}
			},
			"required": [
				"id",
				"name",
				"authors",
				"recipe_count",
				"kitchens",
				"invites"
			],
			"type": "object"
		}
	},
	{
		"name": "invite_to_cookbook",
		"summary": "Mint a one-use Invite for somebody to write your Cookbook with you. When they accept, their recipes and yours become one Cookbook either of you changes.",
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
		"name": "cancel_cookbook_invite",
		"summary": "End a Cookbook Invite nobody has used yet.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"invite_id": {
					"type": "string"
				}
			},
			"required": [
				"invite_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"ended": {
					"type": "boolean"
				}
			},
			"required": [
				"ended"
			],
			"type": "object"
		}
	},
	{
		"name": "read_cookbook_invite",
		"summary": "What accepting a Cookbook Invite would do, before you say yes: whose Cookbook it is, and how many recipes on each side become one.",
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
				"already_yours": {
					"type": "boolean"
				},
				"cookbook": {
					"additionalProperties": false,
					"properties": {
						"authors": {
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
						"id": {
							"type": "string"
						},
						"invites": {
							"items": {
								"additionalProperties": false,
								"properties": {
									"created_at": {
										"type": "string"
									},
									"invite_id": {
										"type": "string"
									}
								},
								"required": [
									"invite_id",
									"created_at"
								],
								"type": "object"
							},
							"type": "array"
						},
						"kitchens": {
							"items": {
								"additionalProperties": false,
								"properties": {
									"id": {
										"type": "string"
									},
									"name": {
										"type": "string"
									}
								},
								"required": [
									"id",
									"name"
								],
								"type": "object"
							},
							"type": "array"
						},
						"name": {
							"type": [
								"string",
								"null"
							]
						},
						"recipe_count": {
							"type": "integer"
						}
					},
					"required": [
						"id",
						"name",
						"authors",
						"recipe_count",
						"kitchens",
						"invites"
					],
					"type": "object"
				},
				"their_recipes": {
					"type": "integer"
				},
				"together_recipes": {
					"description": "How many recipes the one Cookbook holds once joined: fewer than the two counts added up wherever both already hold a version of the same recipe.",
					"type": "integer"
				},
				"your_recipes": {
					"type": "integer"
				}
			},
			"required": [
				"cookbook",
				"their_recipes",
				"your_recipes",
				"together_recipes",
				"already_yours"
			],
			"type": "object"
		}
	},
	{
		"name": "accept_cookbook_invite",
		"summary": "Open a Cookbook Invite: your Cookbook joins the one it names, and every recipe in either becomes one Cookbook you both change. Spent on use.",
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
				"authors": {
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
				"id": {
					"type": "string"
				},
				"invites": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
							"invite_id": {
								"type": "string"
							}
						},
						"required": [
							"invite_id",
							"created_at"
						],
						"type": "object"
					},
					"type": "array"
				},
				"kitchens": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"id",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"recipe_count": {
					"type": "integer"
				}
			},
			"required": [
				"id",
				"name",
				"authors",
				"recipe_count",
				"kitchens",
				"invites"
			],
			"type": "object"
		}
	},
	{
		"name": "leave_cookbook",
		"summary": "Leave the Cookbook you write with others, taking your own Branch of every recipe in it with its whole history. Whoever started a recipe keeps the original; everyone else a copy.",
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
				"authors": {
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
				"id": {
					"type": "string"
				},
				"invites": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
							"invite_id": {
								"type": "string"
							}
						},
						"required": [
							"invite_id",
							"created_at"
						],
						"type": "object"
					},
					"type": "array"
				},
				"kitchens": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"id",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"recipe_count": {
					"type": "integer"
				}
			},
			"required": [
				"id",
				"name",
				"authors",
				"recipe_count",
				"kitchens",
				"invites"
			],
			"type": "object"
		}
	},
	{
		"name": "remove_cookbook_author",
		"summary": "Separate another Co-author from your Cookbook. They leave with a Branch of every recipe in it, as though they had left.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"person_id": {
					"type": "string"
				}
			},
			"required": [
				"person_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"authors": {
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
				"id": {
					"type": "string"
				},
				"invites": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
							"invite_id": {
								"type": "string"
							}
						},
						"required": [
							"invite_id",
							"created_at"
						],
						"type": "object"
					},
					"type": "array"
				},
				"kitchens": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"name": {
								"type": "string"
							}
						},
						"required": [
							"id",
							"name"
						],
						"type": "object"
					},
					"type": "array"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				},
				"recipe_count": {
					"type": "integer"
				}
			},
			"required": [
				"id",
				"name",
				"authors",
				"recipe_count",
				"kitchens",
				"invites"
			],
			"type": "object"
		}
	},
	{
		"name": "create_tag",
		"summary": "Create a Tag in your own Cookbook, named in one Language. A word the Cookbook already files under returns the Tag it already has rather than making a second.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchen_id": {
					"description": "Ignored. Everything you write lands in your own Cookbook (ADR 0041), so there is no Kitchen to name. Accepted so that a client which still sends it is not refused.",
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
				"language",
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"cookbook_id": {
					"type": "string"
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
				"language_fallback": {
					"type": "boolean"
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
				"recipes": {
					"type": "integer"
				}
			},
			"required": [
				"id",
				"cookbook_id",
				"name",
				"language",
				"names",
				"recipes",
				"language_fallback"
			],
			"type": "object"
		}
	},
	{
		"name": "list_tags",
		"summary": "List every Tag your own Cookbook files by, each shown in the reader's Reading Language where it has a name there. With `everywhere`, every word any Cookbook you may see files by, one entry per word — what a shelf filters by. With a `kitchen_id`, the same for the Cookbooks seen in that one Kitchen of yours.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"everywhere": {
					"type": "boolean"
				},
				"kitchen_id": {
					"description": "One of your Kitchens: every word its Cookbooks file by, one entry per word.",
					"type": "string"
				}
			},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"tags": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"cookbook_id": {
								"type": "string"
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
							"language_fallback": {
								"type": "boolean"
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
							"recipes": {
								"type": "integer"
							}
						},
						"required": [
							"id",
							"cookbook_id",
							"name",
							"language",
							"names",
							"recipes",
							"language_fallback"
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
				"cookbook_id": {
					"type": "string"
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
				"language_fallback": {
					"type": "boolean"
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
				"recipes": {
					"type": "integer"
				}
			},
			"required": [
				"id",
				"cookbook_id",
				"name",
				"language",
				"names",
				"recipes",
				"language_fallback"
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
				"cookbook_id": {
					"type": "string"
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
				"language_fallback": {
					"type": "boolean"
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
				"recipes": {
					"type": "integer"
				}
			},
			"required": [
				"id",
				"cookbook_id",
				"name",
				"language",
				"names",
				"recipes",
				"language_fallback"
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
							"cookbook_id": {
								"type": "string"
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
							"language_fallback": {
								"type": "boolean"
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
							"recipes": {
								"type": "integer"
							}
						},
						"required": [
							"id",
							"cookbook_id",
							"name",
							"language",
							"names",
							"recipes",
							"language_fallback"
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
		"summary": "Relate one of your Cookbook's Recipes to any Recipe you may see, or take that single two-way, untyped link back off. Your Cookbook keeps the link. It never changes either Recipe or travels in a Bundle or Share. Name the far end with `related_branch_id`, or with `related_lineage_id` where the Recipe there has since been deleted — exactly one of the two.",
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
					"description": "The Recipe at the far end, named by its Branch. The ordinary way to say it.",
					"type": "string"
				},
				"related_lineage_id": {
					"description": "The far end named by its Lineage instead. Use this to take off a link to a Recipe that has been deleted, which has no Branch left to name.",
					"type": "string"
				}
			},
			"required": [
				"branch_id",
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
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"language_fallback": {
								"type": "boolean"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"type": [
									"string",
									"null"
								]
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"lineage_id",
							"branch_id",
							"title",
							"main_photo",
							"language",
							"language_fallback"
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
					"description": "Ignored. Everything you write lands in your own Cookbook (ADR 0041), so there is no Kitchen to name. Accepted so that a client which still sends it is not refused.",
					"type": "string"
				},
				"language": {
					"description": "The Language this recipe is written in. Left out, it is detected from the recipe's own text, falling back to the writer's Reading Language where there is too little text to tell. `unknown` says the recipe is honestly more than one Language.",
					"enum": [
						"en",
						"fr",
						"es",
						"unknown"
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
				"nutrition": {
					"additionalProperties": false,
					"properties": {
						"basis": {
							"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
							"enum": [
								"per_serving",
								"per_100g"
							]
						},
						"calories": {
							"description": "Calories, zero or more.",
							"minimum": 0,
							"type": "number"
						}
					},
					"required": [
						"calories",
						"basis"
					],
					"type": [
						"object",
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
							"text"
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
				"cookbook": {
					"additionalProperties": false,
					"properties": {
						"authors": {
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
						"id": {
							"type": "string"
						},
						"name": {
							"type": [
								"string",
								"null"
							]
						}
					},
					"required": [
						"id",
						"name",
						"authors"
					],
					"type": "object"
				},
				"cooked": {
					"additionalProperties": false,
					"properties": {
						"count": {
							"minimum": 0,
							"type": "integer"
						},
						"last_cooked_at": {
							"type": [
								"string",
								"null"
							]
						},
						"ratings": {
							"items": {
								"additionalProperties": false,
								"properties": {
									"at": {
										"type": "string"
									},
									"name": {
										"type": "string"
									},
									"person_id": {
										"type": "string"
									},
									"rating": {
										"enum": [
											"again",
											"tweak",
											"no"
										],
										"type": "string"
									}
								},
								"required": [
									"person_id",
									"name",
									"rating",
									"at"
								],
								"type": "object"
							},
							"type": "array"
						}
					},
					"required": [
						"count",
						"last_cooked_at",
						"ratings"
					],
					"type": "object"
				},
				"hand_id": {
					"type": "string"
				},
				"head_version_id": {
					"type": "string"
				},
				"language": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
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
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"language_fallback": {
								"type": "boolean"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"type": [
									"string",
									"null"
								]
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"lineage_id",
							"branch_id",
							"title",
							"main_photo",
							"language",
							"language_fallback"
						],
						"type": "object"
					},
					"type": "array"
				},
				"tags": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"cookbook_id": {
								"type": "string"
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
							"language_fallback": {
								"type": "boolean"
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
							"recipes": {
								"type": "integer"
							}
						},
						"required": [
							"id",
							"cookbook_id",
							"name",
							"language",
							"names",
							"recipes",
							"language_fallback"
						],
						"type": "object"
					},
					"type": "array"
				},
				"translation": {
					"additionalProperties": false,
					"properties": {
						"source_branch_id": {
							"description": "The recipe this one translates. Null where that recipe is not on this instance — a Translation may arrive on its own, and how far behind it has fallen is then unanswerable rather than zero.",
							"type": [
								"string",
								"null"
							]
						},
						"translates_version_id": {
							"description": "The Version of the source this recipe's newest Version renders.",
							"type": "string"
						},
						"versions_behind": {
							"description": "How many Versions the source has moved on since the one this translates. Zero means up to date.",
							"type": [
								"integer",
								"null"
							]
						}
					},
					"required": [
						"translates_version_id",
						"source_branch_id",
						"versions_behind"
					],
					"type": [
						"object",
						"null"
					]
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
							"components": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
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
												"nutrition": {
													"additionalProperties": false,
													"properties": {
														"basis": {
															"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
															"enum": [
																"per_serving",
																"per_100g"
															]
														},
														"calories": {
															"description": "Calories, zero or more.",
															"minimum": 0,
															"type": "number"
														}
													},
													"required": [
														"calories",
														"basis"
													],
													"type": [
														"object",
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
												"nutrition",
												"ingredients",
												"steps"
											],
											"type": [
												"object",
												"null"
											]
										},
										"held": {
											"type": "boolean"
										},
										"lineage_id": {
											"type": "string"
										},
										"measured": {
											"additionalProperties": false,
											"properties": {
												"ingredients": {
													"items": {
														"type": [
															"string",
															"null"
														]
													},
													"type": "array"
												},
												"steps": {
													"items": {
														"type": [
															"string",
															"null"
														]
													},
													"type": "array"
												}
											},
											"required": [
												"ingredients",
												"steps"
											],
											"type": [
												"object",
												"null"
											]
										},
										"path": {
											"items": {
												"type": "integer"
											},
											"type": "array"
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
													"lineage_id": {
														"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
													"target",
													"lineage_id"
												],
												"type": [
													"object",
													"null"
												]
											},
											"type": [
												"array",
												"null"
											]
										},
										"said": {
											"type": "string"
										},
										"share": {
											"type": [
												"number",
												"null"
											]
										},
										"stopped": {
											"type": "boolean"
										},
										"title": {
											"type": [
												"string",
												"null"
											]
										}
									},
									"required": [
										"path",
										"lineage_id",
										"held",
										"stopped",
										"branch_id",
										"title",
										"share",
										"said",
										"content",
										"readings",
										"measured"
									],
									"type": "object"
								},
								"type": "array"
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
									"nutrition": {
										"additionalProperties": false,
										"properties": {
											"basis": {
												"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
												"enum": [
													"per_serving",
													"per_100g"
												]
											},
											"calories": {
												"description": "Calories, zero or more.",
												"minimum": 0,
												"type": "number"
											}
										},
										"required": [
											"calories",
											"basis"
										],
										"type": [
											"object",
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
									"nutrition",
									"ingredients",
									"steps"
								],
								"type": "object"
							},
							"cooking": {
								"additionalProperties": false,
								"properties": {
									"steps": {
										"items": {
											"additionalProperties": false,
											"properties": {
												"timer_seconds": {
													"minimum": 1,
													"type": [
														"integer",
														"null"
													]
												},
												"uses": {
													"items": {
														"minimum": 0,
														"type": "integer"
													},
													"type": "array"
												}
											},
											"required": [
												"uses",
												"timer_seconds"
											],
											"type": [
												"object",
												"null"
											]
										},
										"type": "array"
									}
								},
								"required": [
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
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"measured": {
								"additionalProperties": false,
								"properties": {
									"ingredients": {
										"items": {
											"type": [
												"string",
												"null"
											]
										},
										"type": "array"
									},
									"steps": {
										"items": {
											"type": [
												"string",
												"null"
											]
										},
										"type": "array"
									}
								},
								"required": [
									"ingredients",
									"steps"
								],
								"type": "object"
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
										"lineage_id": {
											"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
										"target",
										"lineage_id"
									],
									"type": [
										"object",
										"null"
									]
								},
								"type": "array"
							},
							"scaled_to": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"sequence": {
								"type": "integer"
							},
							"translates_version_id": {
								"type": [
									"string",
									"null"
								]
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
							"readings",
							"measured",
							"cooking",
							"components",
							"translates_version_id",
							"language",
							"scaled_to"
						],
						"type": "object"
					},
					"type": "array"
				},
				"writes": {
					"type": "boolean"
				}
			},
			"required": [
				"branch_id",
				"lineage_id",
				"cookbook",
				"name",
				"writes",
				"hand_id",
				"language",
				"origin_address",
				"head_version_id",
				"versions",
				"translation",
				"tags",
				"related_recipes",
				"cooked"
			],
			"type": "object"
		}
	},
	{
		"name": "save_recipe_version",
		"summary": "Save a new state of a Recipe onto a Branch — the whole recipe as written, replacing what was there. A rapid re-save by the same Hand collapses into the Version already being shaped rather than starting a new one. Changing a recipe your Cookbook did not write — a Kitchen-mate's, or one that arrived — is a Copy: it starts a new Branch of the same Lineage in your own Cookbook, starting at the Version you changed and carrying the whole chain behind it — the Branch you changed is left untouched. The Branch must be one you may see.",
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
					"description": "Ignored. Everything you write lands in your own Cookbook (ADR 0041), so there is no Kitchen to name. Accepted so that a client which still sends it is not refused.",
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
				"nutrition": {
					"additionalProperties": false,
					"properties": {
						"basis": {
							"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
							"enum": [
								"per_serving",
								"per_100g"
							]
						},
						"calories": {
							"description": "Calories, zero or more.",
							"minimum": 0,
							"type": "number"
						}
					},
					"required": [
						"calories",
						"basis"
					],
					"type": [
						"object",
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
							"text"
						],
						"type": "object"
					},
					"type": "array"
				},
				"title": {
					"type": "string"
				},
				"translates_version_id": {
					"description": "For a Translation, the Version of the source this save now renders — how bringing a Translation up to date is said. Left out, whatever the Version being replaced pointed at is carried forward, so editing a Translation's wording never claims it has caught up.",
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
				"language": {
					"description": "The Language this recipe still carries. A save never changes it.",
					"type": "string"
				},
				"language_offer": {
					"description": "The Language this text reads as, when that disagrees with the one the recipe carries — an offer to put to the cook, never a change. Null when they agree, when there is too little text to tell, and always when the Language is unknown.",
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
				"sequence": {
					"type": "integer"
				},
				"translates_version_id": {
					"description": "The Version of the source this Version renders, for a Translation. Carried forward from the Version replaced unless this save named a new one; null on a recipe that translates nothing.",
					"type": [
						"string",
						"null"
					]
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
				"copied",
				"language",
				"language_offer",
				"translates_version_id"
			],
			"type": "object"
		}
	},
	{
		"name": "start_variation",
		"summary": "Start a variation of a recipe: a Branch of it, unchanged, in your own Cookbook, under a name you give it (\"Vegetarian\"). Changing one never changes the other.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"name": {
					"type": "string"
				}
			},
			"required": [
				"branch_id",
				"name"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"cookbook": {
					"additionalProperties": false,
					"properties": {
						"authors": {
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
						"id": {
							"type": "string"
						},
						"name": {
							"type": [
								"string",
								"null"
							]
						}
					},
					"required": [
						"id",
						"name",
						"authors"
					],
					"type": "object"
				},
				"cooked": {
					"additionalProperties": false,
					"properties": {
						"count": {
							"minimum": 0,
							"type": "integer"
						},
						"last_cooked_at": {
							"type": [
								"string",
								"null"
							]
						},
						"ratings": {
							"items": {
								"additionalProperties": false,
								"properties": {
									"at": {
										"type": "string"
									},
									"name": {
										"type": "string"
									},
									"person_id": {
										"type": "string"
									},
									"rating": {
										"enum": [
											"again",
											"tweak",
											"no"
										],
										"type": "string"
									}
								},
								"required": [
									"person_id",
									"name",
									"rating",
									"at"
								],
								"type": "object"
							},
							"type": "array"
						}
					},
					"required": [
						"count",
						"last_cooked_at",
						"ratings"
					],
					"type": "object"
				},
				"hand_id": {
					"type": "string"
				},
				"head_version_id": {
					"type": "string"
				},
				"language": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
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
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"language_fallback": {
								"type": "boolean"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"type": [
									"string",
									"null"
								]
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"lineage_id",
							"branch_id",
							"title",
							"main_photo",
							"language",
							"language_fallback"
						],
						"type": "object"
					},
					"type": "array"
				},
				"tags": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"cookbook_id": {
								"type": "string"
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
							"language_fallback": {
								"type": "boolean"
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
							"recipes": {
								"type": "integer"
							}
						},
						"required": [
							"id",
							"cookbook_id",
							"name",
							"language",
							"names",
							"recipes",
							"language_fallback"
						],
						"type": "object"
					},
					"type": "array"
				},
				"translation": {
					"additionalProperties": false,
					"properties": {
						"source_branch_id": {
							"description": "The recipe this one translates. Null where that recipe is not on this instance — a Translation may arrive on its own, and how far behind it has fallen is then unanswerable rather than zero.",
							"type": [
								"string",
								"null"
							]
						},
						"translates_version_id": {
							"description": "The Version of the source this recipe's newest Version renders.",
							"type": "string"
						},
						"versions_behind": {
							"description": "How many Versions the source has moved on since the one this translates. Zero means up to date.",
							"type": [
								"integer",
								"null"
							]
						}
					},
					"required": [
						"translates_version_id",
						"source_branch_id",
						"versions_behind"
					],
					"type": [
						"object",
						"null"
					]
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
							"components": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
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
												"nutrition": {
													"additionalProperties": false,
													"properties": {
														"basis": {
															"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
															"enum": [
																"per_serving",
																"per_100g"
															]
														},
														"calories": {
															"description": "Calories, zero or more.",
															"minimum": 0,
															"type": "number"
														}
													},
													"required": [
														"calories",
														"basis"
													],
													"type": [
														"object",
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
												"nutrition",
												"ingredients",
												"steps"
											],
											"type": [
												"object",
												"null"
											]
										},
										"held": {
											"type": "boolean"
										},
										"lineage_id": {
											"type": "string"
										},
										"measured": {
											"additionalProperties": false,
											"properties": {
												"ingredients": {
													"items": {
														"type": [
															"string",
															"null"
														]
													},
													"type": "array"
												},
												"steps": {
													"items": {
														"type": [
															"string",
															"null"
														]
													},
													"type": "array"
												}
											},
											"required": [
												"ingredients",
												"steps"
											],
											"type": [
												"object",
												"null"
											]
										},
										"path": {
											"items": {
												"type": "integer"
											},
											"type": "array"
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
													"lineage_id": {
														"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
													"target",
													"lineage_id"
												],
												"type": [
													"object",
													"null"
												]
											},
											"type": [
												"array",
												"null"
											]
										},
										"said": {
											"type": "string"
										},
										"share": {
											"type": [
												"number",
												"null"
											]
										},
										"stopped": {
											"type": "boolean"
										},
										"title": {
											"type": [
												"string",
												"null"
											]
										}
									},
									"required": [
										"path",
										"lineage_id",
										"held",
										"stopped",
										"branch_id",
										"title",
										"share",
										"said",
										"content",
										"readings",
										"measured"
									],
									"type": "object"
								},
								"type": "array"
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
									"nutrition": {
										"additionalProperties": false,
										"properties": {
											"basis": {
												"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
												"enum": [
													"per_serving",
													"per_100g"
												]
											},
											"calories": {
												"description": "Calories, zero or more.",
												"minimum": 0,
												"type": "number"
											}
										},
										"required": [
											"calories",
											"basis"
										],
										"type": [
											"object",
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
									"nutrition",
									"ingredients",
									"steps"
								],
								"type": "object"
							},
							"cooking": {
								"additionalProperties": false,
								"properties": {
									"steps": {
										"items": {
											"additionalProperties": false,
											"properties": {
												"timer_seconds": {
													"minimum": 1,
													"type": [
														"integer",
														"null"
													]
												},
												"uses": {
													"items": {
														"minimum": 0,
														"type": "integer"
													},
													"type": "array"
												}
											},
											"required": [
												"uses",
												"timer_seconds"
											],
											"type": [
												"object",
												"null"
											]
										},
										"type": "array"
									}
								},
								"required": [
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
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"measured": {
								"additionalProperties": false,
								"properties": {
									"ingredients": {
										"items": {
											"type": [
												"string",
												"null"
											]
										},
										"type": "array"
									},
									"steps": {
										"items": {
											"type": [
												"string",
												"null"
											]
										},
										"type": "array"
									}
								},
								"required": [
									"ingredients",
									"steps"
								],
								"type": "object"
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
										"lineage_id": {
											"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
										"target",
										"lineage_id"
									],
									"type": [
										"object",
										"null"
									]
								},
								"type": "array"
							},
							"scaled_to": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"sequence": {
								"type": "integer"
							},
							"translates_version_id": {
								"type": [
									"string",
									"null"
								]
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
							"readings",
							"measured",
							"cooking",
							"components",
							"translates_version_id",
							"language",
							"scaled_to"
						],
						"type": "object"
					},
					"type": "array"
				},
				"writes": {
					"type": "boolean"
				}
			},
			"required": [
				"branch_id",
				"lineage_id",
				"cookbook",
				"name",
				"writes",
				"hand_id",
				"language",
				"origin_address",
				"head_version_id",
				"versions",
				"translation",
				"tags",
				"related_recipes",
				"cooked"
			],
			"type": "object"
		}
	},
	{
		"name": "rename_branch",
		"summary": "Name one of your Cookbook's Branches of a recipe, or clear its name. A Cookbook keeps one unnamed Branch of a recipe in each Language, so a second one needs a name.",
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
				}
			},
			"required": [
				"branch_id",
				"name"
			],
			"type": "object"
		},
		"output_schema": {
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
				}
			},
			"required": [
				"branch_id",
				"name"
			],
			"type": "object"
		}
	},
	{
		"name": "delete_recipe",
		"summary": "Take one recipe off the shelf for good. It is gone from the shelf, from search and from every member of its Kitchen, and nothing brings it back. **One Branch**: a translation is an ordinary Branch, so deleting the English one leaves the French one whole, and another Kitchen's copy of the same recipe is untouched. **The cooking history stays.** Every Attempt ever made from this recipe keeps its rating, its note and its Photographs, and the Cooked diary keeps each entry under the name the recipe was known by. So does a Shopping List holding it, which says it can no longer be read rather than quietly dropping it. No Version is ever deleted, by this or by anything else. A live Share Link stops working.",
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
		"name": "read_pasted_recipe",
		"summary": "Read a whole recipe pasted as text into a title, an ingredient list and a method. Decides only what each line IS — an Ingredient Line, a Step, a Section — and never what it says: every line comes back exactly as pasted, with no amount extracted, no rewording and no reordering (ADR 0002). Nothing is guessed beyond the split and the title: no Yield, no times, no Source, and no Component (ADR 0008). It writes nothing anywhere — what comes back is shown to whoever pasted it, who moves the boundary if it landed wrong, and only then is a recipe saved by an ordinary create_recipe or save_recipe_version. The boundary is the index in `lines` where the method starts, so moving it re-splits the same answer without asking again.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"text": {
					"type": "string"
				}
			},
			"required": [
				"text"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"boundary": {
					"minimum": 0,
					"type": "integer"
				},
				"lines": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"kind": {
								"enum": [
									"line",
									"section"
								]
							},
							"text": {
								"type": "string"
							}
						},
						"required": [
							"text",
							"kind"
						],
						"type": "object"
					},
					"type": "array"
				},
				"title": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"title",
				"lines",
				"boundary"
			],
			"type": "object"
		}
	},
	{
		"name": "start_translation",
		"summary": "Translate a recipe: start an ordinary Branch of the same Lineage in another Language, whose first Version records which Version of the source it renders. There is no Translation object — what this makes is a Branch, and every Operation from here on is the ordinary one. Its chain starts fresh rather than carrying the source's, which is what separates it from a Copy: different words rendering the same dish, with a history of their own. An agent translating calls this under the Person's own Credential and is a scribe, not an author.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"description": "The recipe being translated.",
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
					"description": "Ignored. Everything you write lands in your own Cookbook (ADR 0041), so there is no Kitchen to name. Accepted so that a client which still sends it is not refused.",
					"type": "string"
				},
				"language": {
					"description": "The Language this rendering is written in — necessarily a different one from the recipe it translates. Never `unknown`: a recipe that is honestly more than one Language can neither be a Translation nor have one.",
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
				"name": {
					"type": "string"
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"nutrition": {
					"additionalProperties": false,
					"properties": {
						"basis": {
							"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
							"enum": [
								"per_serving",
								"per_100g"
							]
						},
						"calories": {
							"description": "Calories, zero or more.",
							"minimum": 0,
							"type": "number"
						}
					},
					"required": [
						"calories",
						"basis"
					],
					"type": [
						"object",
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
							"text"
						],
						"type": "object"
					},
					"type": "array"
				},
				"title": {
					"type": "string"
				},
				"translates_version_id": {
					"description": "Which Version of the source this renders. Defaults to where the source stands now.",
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
				"language",
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
				"cookbook": {
					"additionalProperties": false,
					"properties": {
						"authors": {
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
						"id": {
							"type": "string"
						},
						"name": {
							"type": [
								"string",
								"null"
							]
						}
					},
					"required": [
						"id",
						"name",
						"authors"
					],
					"type": "object"
				},
				"cooked": {
					"additionalProperties": false,
					"properties": {
						"count": {
							"minimum": 0,
							"type": "integer"
						},
						"last_cooked_at": {
							"type": [
								"string",
								"null"
							]
						},
						"ratings": {
							"items": {
								"additionalProperties": false,
								"properties": {
									"at": {
										"type": "string"
									},
									"name": {
										"type": "string"
									},
									"person_id": {
										"type": "string"
									},
									"rating": {
										"enum": [
											"again",
											"tweak",
											"no"
										],
										"type": "string"
									}
								},
								"required": [
									"person_id",
									"name",
									"rating",
									"at"
								],
								"type": "object"
							},
							"type": "array"
						}
					},
					"required": [
						"count",
						"last_cooked_at",
						"ratings"
					],
					"type": "object"
				},
				"hand_id": {
					"type": "string"
				},
				"head_version_id": {
					"type": "string"
				},
				"language": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
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
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"language_fallback": {
								"type": "boolean"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"type": [
									"string",
									"null"
								]
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"lineage_id",
							"branch_id",
							"title",
							"main_photo",
							"language",
							"language_fallback"
						],
						"type": "object"
					},
					"type": "array"
				},
				"tags": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"cookbook_id": {
								"type": "string"
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
							"language_fallback": {
								"type": "boolean"
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
							"recipes": {
								"type": "integer"
							}
						},
						"required": [
							"id",
							"cookbook_id",
							"name",
							"language",
							"names",
							"recipes",
							"language_fallback"
						],
						"type": "object"
					},
					"type": "array"
				},
				"translation": {
					"additionalProperties": false,
					"properties": {
						"source_branch_id": {
							"description": "The recipe this one translates. Null where that recipe is not on this instance — a Translation may arrive on its own, and how far behind it has fallen is then unanswerable rather than zero.",
							"type": [
								"string",
								"null"
							]
						},
						"translates_version_id": {
							"description": "The Version of the source this recipe's newest Version renders.",
							"type": "string"
						},
						"versions_behind": {
							"description": "How many Versions the source has moved on since the one this translates. Zero means up to date.",
							"type": [
								"integer",
								"null"
							]
						}
					},
					"required": [
						"translates_version_id",
						"source_branch_id",
						"versions_behind"
					],
					"type": [
						"object",
						"null"
					]
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
							"components": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
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
												"nutrition": {
													"additionalProperties": false,
													"properties": {
														"basis": {
															"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
															"enum": [
																"per_serving",
																"per_100g"
															]
														},
														"calories": {
															"description": "Calories, zero or more.",
															"minimum": 0,
															"type": "number"
														}
													},
													"required": [
														"calories",
														"basis"
													],
													"type": [
														"object",
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
												"nutrition",
												"ingredients",
												"steps"
											],
											"type": [
												"object",
												"null"
											]
										},
										"held": {
											"type": "boolean"
										},
										"lineage_id": {
											"type": "string"
										},
										"measured": {
											"additionalProperties": false,
											"properties": {
												"ingredients": {
													"items": {
														"type": [
															"string",
															"null"
														]
													},
													"type": "array"
												},
												"steps": {
													"items": {
														"type": [
															"string",
															"null"
														]
													},
													"type": "array"
												}
											},
											"required": [
												"ingredients",
												"steps"
											],
											"type": [
												"object",
												"null"
											]
										},
										"path": {
											"items": {
												"type": "integer"
											},
											"type": "array"
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
													"lineage_id": {
														"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
													"target",
													"lineage_id"
												],
												"type": [
													"object",
													"null"
												]
											},
											"type": [
												"array",
												"null"
											]
										},
										"said": {
											"type": "string"
										},
										"share": {
											"type": [
												"number",
												"null"
											]
										},
										"stopped": {
											"type": "boolean"
										},
										"title": {
											"type": [
												"string",
												"null"
											]
										}
									},
									"required": [
										"path",
										"lineage_id",
										"held",
										"stopped",
										"branch_id",
										"title",
										"share",
										"said",
										"content",
										"readings",
										"measured"
									],
									"type": "object"
								},
								"type": "array"
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
									"nutrition": {
										"additionalProperties": false,
										"properties": {
											"basis": {
												"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
												"enum": [
													"per_serving",
													"per_100g"
												]
											},
											"calories": {
												"description": "Calories, zero or more.",
												"minimum": 0,
												"type": "number"
											}
										},
										"required": [
											"calories",
											"basis"
										],
										"type": [
											"object",
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
									"nutrition",
									"ingredients",
									"steps"
								],
								"type": "object"
							},
							"cooking": {
								"additionalProperties": false,
								"properties": {
									"steps": {
										"items": {
											"additionalProperties": false,
											"properties": {
												"timer_seconds": {
													"minimum": 1,
													"type": [
														"integer",
														"null"
													]
												},
												"uses": {
													"items": {
														"minimum": 0,
														"type": "integer"
													},
													"type": "array"
												}
											},
											"required": [
												"uses",
												"timer_seconds"
											],
											"type": [
												"object",
												"null"
											]
										},
										"type": "array"
									}
								},
								"required": [
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
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"measured": {
								"additionalProperties": false,
								"properties": {
									"ingredients": {
										"items": {
											"type": [
												"string",
												"null"
											]
										},
										"type": "array"
									},
									"steps": {
										"items": {
											"type": [
												"string",
												"null"
											]
										},
										"type": "array"
									}
								},
								"required": [
									"ingredients",
									"steps"
								],
								"type": "object"
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
										"lineage_id": {
											"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
										"target",
										"lineage_id"
									],
									"type": [
										"object",
										"null"
									]
								},
								"type": "array"
							},
							"scaled_to": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"sequence": {
								"type": "integer"
							},
							"translates_version_id": {
								"type": [
									"string",
									"null"
								]
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
							"readings",
							"measured",
							"cooking",
							"components",
							"translates_version_id",
							"language",
							"scaled_to"
						],
						"type": "object"
					},
					"type": "array"
				},
				"writes": {
					"type": "boolean"
				}
			},
			"required": [
				"branch_id",
				"lineage_id",
				"cookbook",
				"name",
				"writes",
				"hand_id",
				"language",
				"origin_address",
				"head_version_id",
				"versions",
				"translation",
				"tags",
				"related_recipes",
				"cooked"
			],
			"type": "object"
		}
	},
	{
		"name": "set_recipe_language",
		"summary": "Say what Language a recipe is written in. The only thing that acts on a save's language offer — Kamosu detects and offers, and never changes a Language without the cook saying so. Changing it makes a Version, so the change leaves a trace in the recipe's own history. Setting it to `unknown` says the recipe is honestly more than one Language: from then on it is offered nothing, marked nothing, and shown to every reader whatever they read in.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"language": {
					"enum": [
						"en",
						"fr",
						"es",
						"unknown"
					]
				}
			},
			"required": [
				"branch_id",
				"language"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"language": {
					"type": "string"
				},
				"sequence": {
					"description": "The sequence of the Version this change made, or null when the recipe was already in that Language and nothing changed.",
					"type": [
						"integer",
						"null"
					]
				}
			},
			"required": [
				"branch_id",
				"language",
				"sequence"
			],
			"type": "object"
		}
	},
	{
		"name": "import",
		"summary": "Bring a batch of already-read recipes into your own Cookbook, as a Job. Matched by foreign id against this Cookbook's ledger for the source kind, so re-running finds what it already made instead of doubling it; a recipe found changed is offered for review, never written over. Reading the outside source itself — a file, a page, a Bundle — is each importer's own job.",
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
								"description": "The Language this recipe is written in. Left out, it is detected from the recipe's own text, falling back to the writer's Reading Language where there is too little text to tell. `unknown` says the recipe is honestly more than one Language.",
								"enum": [
									"en",
									"fr",
									"es",
									"unknown"
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
							"nutrition": {
								"additionalProperties": false,
								"properties": {
									"basis": {
										"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
										"enum": [
											"per_serving",
											"per_100g"
										]
									},
									"calories": {
										"description": "Calories, zero or more.",
										"minimum": 0,
										"type": "number"
									}
								},
								"required": [
									"calories",
									"basis"
								],
								"type": [
									"object",
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
										"text"
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
					"description": "Which outside source these candidates came from. One ledger is kept per Cookbook per source kind.",
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
							"bare": {
								"description": "It came as a bare name (and usually a link): no Ingredient Line and no Step. A real recipe that arrived correctly, not a failure.",
								"type": "boolean"
							},
							"branch_id": {
								"description": "The Branch as this instance holds it — the id every other Operation and every URL takes. On a recipe received from elsewhere it is not the id it travelled under.",
								"type": "string"
							},
							"foreign_id": {
								"description": "What the source called this recipe: from a Bundle, the Branch id it travels under, which is the sender's.",
								"type": "string"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"description": "The Photograph the recipe arrived with as its Main Photo, or null.",
								"type": [
									"string",
									"null"
								]
							},
							"status": {
								"enum": [
									"created",
									"extended",
									"unchanged"
								]
							},
							"subject": {
								"description": "From a Bundle: whether this recipe is what the Bundle is about, rather than a Passenger that travelled because something needed it.",
								"type": "boolean"
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
				"cookbook_id": {
					"type": "string"
				},
				"import_id": {
					"type": "string"
				},
				"left_out": {
					"description": "What the importer read and deliberately did not bring in, recipe by recipe: `site_icon` is a source site's favicon, which is not a photograph of the dish (ADR 0017); `extra_photos` are pictures past the first, since a recipe keeps one Main Photo and Kamosu does not guess which Step another shows; `unreadable_photos` could not be decoded.",
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"count": {
								"type": "integer"
							},
							"foreign_id": {
								"type": "string"
							},
							"icon": {
								"description": "For a `site_icon`: the icon itself as a `data:` URI, so the Report can show what was left out. Carried only here, never stored as a Photograph.",
								"type": "string"
							},
							"title": {
								"type": "string"
							},
							"what": {
								"enum": [
									"site_icon",
									"extra_photos",
									"unreadable_photos"
								]
							}
						},
						"required": [
							"foreign_id",
							"branch_id",
							"title",
							"what",
							"count"
						],
						"type": "object"
					},
					"type": "array"
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
				"related_candidates": {
					"description": "Pairs of recipes this Import landed that share a name or a web page, offered as Related Recipes to tick with `set_related_recipe` — never joined into one Lineage (ADR 0025). A pair already related is not offered.",
					"items": {
						"additionalProperties": false,
						"properties": {
							"recipes": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"ingredients": {
											"type": "integer"
										},
										"lineage_id": {
											"type": "string"
										},
										"main_photo": {
											"type": [
												"string",
												"null"
											]
										},
										"title": {
											"type": "string"
										}
									},
									"required": [
										"lineage_id",
										"branch_id",
										"title",
										"main_photo",
										"ingredients"
									],
									"type": "object"
								},
								"type": "array"
							},
							"shared": {
								"items": {
									"enum": [
										"name",
										"page"
									]
								},
								"type": "array"
							}
						},
						"required": [
							"recipes",
							"shared"
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
							"kept_as": {
								"additionalProperties": false,
								"description": "From a damaged Bundle: the new recipe of your own its words were kept as, with no history and no link to the recipe it came from.",
								"properties": {
									"branch_id": {
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
									"lineage_id",
									"branch_id",
									"title"
								],
								"type": "object"
							},
							"name": {
								"description": "What the source called it, where even its own id could not be read — a Crouton file's name.",
								"type": "string"
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
				"cookbook_id",
				"source_kind",
				"arrived",
				"offered",
				"unreadable",
				"left_out",
				"related_candidates"
			],
			"type": "object"
		}
	},
	{
		"name": "import_crouton",
		"summary": "Bring in a Crouton library, as a Job: the whole export (a zip of .crumb files) or one .crumb. Each recipe lands in your own Cookbook through the same ledger `import` uses, keyed by its Crouton id, so running it again matches instead of doubling the library. Ingredient Lines are rebuilt from Crouton's split fields; the site's favicon and Crouton's nutrition text are left out. Send the file to POST /api/uploads and pass the `upload_id` it answers, or pass it base64-encoded as `data`.",
		"permission": "person",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"data": {
					"description": "The export itself, base64-encoded — for a Door that can send only JSON.",
					"type": "string"
				},
				"upload_id": {
					"description": "The id POST /api/uploads answered for the export. Used once, then deleted.",
					"type": "string"
				}
			},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"arrived": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"bare": {
								"description": "It came as a bare name (and usually a link): no Ingredient Line and no Step. A real recipe that arrived correctly, not a failure.",
								"type": "boolean"
							},
							"branch_id": {
								"description": "The Branch as this instance holds it — the id every other Operation and every URL takes. On a recipe received from elsewhere it is not the id it travelled under.",
								"type": "string"
							},
							"foreign_id": {
								"description": "What the source called this recipe: from a Bundle, the Branch id it travels under, which is the sender's.",
								"type": "string"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"description": "The Photograph the recipe arrived with as its Main Photo, or null.",
								"type": [
									"string",
									"null"
								]
							},
							"status": {
								"enum": [
									"created",
									"extended",
									"unchanged"
								]
							},
							"subject": {
								"description": "From a Bundle: whether this recipe is what the Bundle is about, rather than a Passenger that travelled because something needed it.",
								"type": "boolean"
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
				"cookbook_id": {
					"type": "string"
				},
				"import_id": {
					"type": "string"
				},
				"left_out": {
					"description": "What the importer read and deliberately did not bring in, recipe by recipe: `site_icon` is a source site's favicon, which is not a photograph of the dish (ADR 0017); `extra_photos` are pictures past the first, since a recipe keeps one Main Photo and Kamosu does not guess which Step another shows; `unreadable_photos` could not be decoded.",
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"count": {
								"type": "integer"
							},
							"foreign_id": {
								"type": "string"
							},
							"icon": {
								"description": "For a `site_icon`: the icon itself as a `data:` URI, so the Report can show what was left out. Carried only here, never stored as a Photograph.",
								"type": "string"
							},
							"title": {
								"type": "string"
							},
							"what": {
								"enum": [
									"site_icon",
									"extra_photos",
									"unreadable_photos"
								]
							}
						},
						"required": [
							"foreign_id",
							"branch_id",
							"title",
							"what",
							"count"
						],
						"type": "object"
					},
					"type": "array"
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
				"related_candidates": {
					"description": "Pairs of recipes this Import landed that share a name or a web page, offered as Related Recipes to tick with `set_related_recipe` — never joined into one Lineage (ADR 0025). A pair already related is not offered.",
					"items": {
						"additionalProperties": false,
						"properties": {
							"recipes": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"ingredients": {
											"type": "integer"
										},
										"lineage_id": {
											"type": "string"
										},
										"main_photo": {
											"type": [
												"string",
												"null"
											]
										},
										"title": {
											"type": "string"
										}
									},
									"required": [
										"lineage_id",
										"branch_id",
										"title",
										"main_photo",
										"ingredients"
									],
									"type": "object"
								},
								"type": "array"
							},
							"shared": {
								"items": {
									"enum": [
										"name",
										"page"
									]
								},
								"type": "array"
							}
						},
						"required": [
							"recipes",
							"shared"
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
							"kept_as": {
								"additionalProperties": false,
								"description": "From a damaged Bundle: the new recipe of your own its words were kept as, with no history and no link to the recipe it came from.",
								"properties": {
									"branch_id": {
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
									"lineage_id",
									"branch_id",
									"title"
								],
								"type": "object"
							},
							"name": {
								"description": "What the source called it, where even its own id could not be read — a Crouton file's name.",
								"type": "string"
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
				"cookbook_id",
				"source_kind",
				"arrived",
				"offered",
				"unreadable",
				"left_out",
				"related_candidates"
			],
			"type": "object"
		}
	},
	{
		"name": "list_imports",
		"summary": "What has been brought into your Kitchens from outside, and what happened each time. One entry per source — a Crouton library, recipe files, web pages — each holding how many recipes its ledger remembers and every arrival you asked for, newest first. An arrival names the Job whose Report `get_job` serves, so what happened is read back long after the screen that started it closed. Listed is an event, never a mark on a recipe: an imported recipe is an ordinary recipe and says nothing about where it came from (ADR 0025).",
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
				"imports": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"arrivals": {
								"description": "Every run of this source the caller asked for, newest first.",
								"items": {
									"additionalProperties": false,
									"properties": {
										"arrived": {
											"type": "integer"
										},
										"created": {
											"description": "Of those that arrived, how many were new rather than already held.",
											"type": "integer"
										},
										"created_at": {
											"type": "string"
										},
										"job_id": {
											"description": "Pass to `get_job` for the Report itself.",
											"type": "string"
										},
										"offered": {
											"type": "integer"
										},
										"status": {
											"enum": [
												"queued",
												"running",
												"completed",
												"failed",
												"cancelled"
											]
										},
										"unreadable": {
											"type": "integer"
										}
									},
									"required": [
										"job_id",
										"status",
										"created_at",
										"arrived",
										"created",
										"offered",
										"unreadable"
									],
									"type": "object"
								},
								"type": "array"
							},
							"created_at": {
								"type": "string"
							},
							"import_id": {
								"description": "The ledger's id, which `forget_import` takes. Null once it has been forgotten: the arrivals and their Reports outlive the ledger, because forgetting throws away what became what, not what happened.",
								"type": [
									"string",
									"null"
								]
							},
							"remembered": {
								"description": "How many recipes this ledger can still match on a re-run. The Kitchen's figure, not the caller's.",
								"type": "integer"
							},
							"source_kind": {
								"description": "Which outside source this is — `crouton`, `bundle`, `web`, or whatever an `import` caller named.",
								"type": "string"
							}
						},
						"required": [
							"import_id",
							"source_kind",
							"created_at",
							"remembered",
							"arrivals"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"imports"
			],
			"type": "object"
		}
	},
	{
		"name": "forget_import",
		"summary": "Throw an Import's ledger away whole — the memory of which outside recipe became which of yours. Every recipe it made stays exactly as it is. Once forgotten, importing the same file again brings everything in as new, so do this when the place it came from is gone.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"import_id": {
					"type": "string"
				}
			},
			"required": [
				"import_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"forgotten": {
					"description": "How many ledger entries were thrown away.",
					"type": "integer"
				},
				"import_id": {
					"type": "string"
				}
			},
			"required": [
				"import_id",
				"forgotten"
			],
			"type": "object"
		}
	},
	{
		"name": "import_web_link",
		"summary": "Bring in a recipe straight from a URL, as a Job. Reads the page's schema.org JSON-LD (#70) — no per-site scraping, no LLM fallback — and lands it in your own Cookbook through the same ledger `import` uses, keyed by the page's own address. Fetching is bound to public addresses at the dialled address and at every redirect (ADR 0033), and — because a page's own text can tell an agent to fetch another URL — always takes the single depth-one lane, never more than one fetch in flight regardless of who is signed in.",
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
							"bare": {
								"description": "It came as a bare name (and usually a link): no Ingredient Line and no Step. A real recipe that arrived correctly, not a failure.",
								"type": "boolean"
							},
							"branch_id": {
								"description": "The Branch as this instance holds it — the id every other Operation and every URL takes. On a recipe received from elsewhere it is not the id it travelled under.",
								"type": "string"
							},
							"foreign_id": {
								"description": "What the source called this recipe: from a Bundle, the Branch id it travels under, which is the sender's.",
								"type": "string"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"description": "The Photograph the recipe arrived with as its Main Photo, or null.",
								"type": [
									"string",
									"null"
								]
							},
							"status": {
								"enum": [
									"created",
									"extended",
									"unchanged"
								]
							},
							"subject": {
								"description": "From a Bundle: whether this recipe is what the Bundle is about, rather than a Passenger that travelled because something needed it.",
								"type": "boolean"
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
				"cookbook_id": {
					"type": "string"
				},
				"import_id": {
					"type": "string"
				},
				"left_out": {
					"description": "What the importer read and deliberately did not bring in, recipe by recipe: `site_icon` is a source site's favicon, which is not a photograph of the dish (ADR 0017); `extra_photos` are pictures past the first, since a recipe keeps one Main Photo and Kamosu does not guess which Step another shows; `unreadable_photos` could not be decoded.",
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"count": {
								"type": "integer"
							},
							"foreign_id": {
								"type": "string"
							},
							"icon": {
								"description": "For a `site_icon`: the icon itself as a `data:` URI, so the Report can show what was left out. Carried only here, never stored as a Photograph.",
								"type": "string"
							},
							"title": {
								"type": "string"
							},
							"what": {
								"enum": [
									"site_icon",
									"extra_photos",
									"unreadable_photos"
								]
							}
						},
						"required": [
							"foreign_id",
							"branch_id",
							"title",
							"what",
							"count"
						],
						"type": "object"
					},
					"type": "array"
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
				"related_candidates": {
					"description": "Pairs of recipes this Import landed that share a name or a web page, offered as Related Recipes to tick with `set_related_recipe` — never joined into one Lineage (ADR 0025). A pair already related is not offered.",
					"items": {
						"additionalProperties": false,
						"properties": {
							"recipes": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"ingredients": {
											"type": "integer"
										},
										"lineage_id": {
											"type": "string"
										},
										"main_photo": {
											"type": [
												"string",
												"null"
											]
										},
										"title": {
											"type": "string"
										}
									},
									"required": [
										"lineage_id",
										"branch_id",
										"title",
										"main_photo",
										"ingredients"
									],
									"type": "object"
								},
								"type": "array"
							},
							"shared": {
								"items": {
									"enum": [
										"name",
										"page"
									]
								},
								"type": "array"
							}
						},
						"required": [
							"recipes",
							"shared"
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
							"kept_as": {
								"additionalProperties": false,
								"description": "From a damaged Bundle: the new recipe of your own its words were kept as, with no history and no link to the recipe it came from.",
								"properties": {
									"branch_id": {
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
									"lineage_id",
									"branch_id",
									"title"
								],
								"type": "object"
							},
							"name": {
								"description": "What the source called it, where even its own id could not be read — a Crouton file's name.",
								"type": "string"
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
				"cookbook_id",
				"source_kind",
				"arrived",
				"offered",
				"unreadable",
				"left_out",
				"related_candidates"
			],
			"type": "object"
		}
	},
	{
		"name": "rename_version",
		"summary": "Rename a Version — the one thing about it that can change later. An absent or empty name clears it. Targeted by the Branch's own sequence number, since the same content can recur more than once on one Branch, each occurrence named on its own. Only the Person who saved that Version may rename it.",
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
		"name": "search_recipes",
		"summary": "The shelf, and searching it. With no query: everything the Kitchens this Person cooks in hold, merged, alphabetical, one entry per Lineage, each titled in the reader's Reading Language with a marked fallback. With a query: the same shelf narrowed to what matched, an exact title first, every entry quoting the line that matched. One Operation either way — Meaning Search arrives here rather than beside it (ADR 0027, ADR 0029).",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"kitchen_id": {
					"type": [
						"string",
						"null"
					]
				},
				"mine": {
					"type": "boolean"
				},
				"query": {
					"type": [
						"string",
						"null"
					]
				},
				"tag_id": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"closest": {
					"type": "boolean"
				},
				"query": {
					"type": [
						"string",
						"null"
					]
				},
				"recipes": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"language": {
								"type": "string"
							},
							"language_fallback": {
								"type": "boolean"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"type": [
									"string",
									"null"
								]
							},
							"matched": {
								"additionalProperties": false,
								"properties": {
									"by": {
										"enum": [
											"words",
											"meaning"
										]
									},
									"line": {
										"type": "string"
									},
									"step_number": {
										"type": [
											"integer",
											"null"
										]
									},
									"where": {
										"enum": [
											"title",
											"tag",
											"ingredient",
											"step",
											"section",
											"note",
											"attempt"
										],
										"type": "string"
									}
								},
								"required": [
									"where",
									"line",
									"step_number",
									"by"
								],
								"type": [
									"object",
									"null"
								]
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
							"lineage_id",
							"branch_id",
							"title",
							"language",
							"language_fallback",
							"main_photo",
							"yield",
							"matched"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"query",
				"closest",
				"recipes"
			],
			"type": "object"
		}
	},
	{
		"name": "home_shelves",
		"summary": "Home: the computed shelves that answer *show me something* rather than handing back a search box — cooked most, quick tonight, never cooked, recently opened. Each is one card per Lineage in the reader's Reading Language, in the same shape the library's shelf answers in. A shelf with nothing on it is left out rather than sent empty, so an instance holding no recipes answers with no shelves at all. All four are counted from recipes and Attempts that already exist, except *recently opened*, which reads what `note_recipe_opened` remembered (ADR 0011, ADR 0027).",
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
				"quick_tonight_minutes": {
					"type": "integer"
				},
				"shelves": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"name": {
								"enum": [
									"cooked_most",
									"quick_tonight",
									"never_cooked",
									"recently_opened"
								]
							},
							"recipes": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"language": {
											"type": "string"
										},
										"language_fallback": {
											"type": "boolean"
										},
										"lineage_id": {
											"type": "string"
										},
										"main_photo": {
											"type": [
												"string",
												"null"
											]
										},
										"matched": {
											"additionalProperties": false,
											"properties": {
												"by": {
													"enum": [
														"words",
														"meaning"
													]
												},
												"line": {
													"type": "string"
												},
												"step_number": {
													"type": [
														"integer",
														"null"
													]
												},
												"where": {
													"enum": [
														"title",
														"tag",
														"ingredient",
														"step",
														"section",
														"note",
														"attempt"
													],
													"type": "string"
												}
											},
											"required": [
												"where",
												"line",
												"step_number",
												"by"
											],
											"type": [
												"object",
												"null"
											]
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
										"lineage_id",
										"branch_id",
										"title",
										"language",
										"language_fallback",
										"main_photo",
										"yield",
										"matched"
									],
									"type": "object"
								},
								"type": "array"
							}
						},
						"required": [
							"name",
							"recipes"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"quick_tonight_minutes",
				"shelves"
			],
			"type": "object"
		}
	},
	{
		"name": "note_recipe_opened",
		"summary": "Remember that the caller opened this recipe, for Home's *recently opened* shelf. One fact per Person per Lineage — opening a recipe's French Branch and its English one is opening the same recipe — and opening it again moves the time rather than adding a row. It is private to the Person, never travels, and is in no fingerprint, Vault or Bundle: an instance that lost it would lose the order of one shelf and nothing else (ADR 0027).",
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
				"lineage_id": {
					"type": "string"
				},
				"opened_at": {
					"type": "string"
				}
			},
			"required": [
				"lineage_id",
				"opened_at"
			],
			"type": "object"
		}
	},
	{
		"name": "meaning_search_status",
		"summary": "Whether Meaning Search is on here, what model it would use, who accepted that model's terms — and whether this caller should be offered it. Answers on every instance, including the many that will never turn it on.",
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
				"accepted_at": {
					"type": [
						"string",
						"null"
					]
				},
				"accepted_by": {
					"type": [
						"string",
						"null"
					]
				},
				"accepted_via_access_key": {
					"type": [
						"boolean",
						"null"
					]
				},
				"declined_at": {
					"type": [
						"string",
						"null"
					]
				},
				"indexed_at": {
					"type": [
						"string",
						"null"
					]
				},
				"may_change": {
					"type": "boolean"
				},
				"model": {
					"type": "string"
				},
				"model_present": {
					"type": "boolean"
				},
				"offer": {
					"type": "boolean"
				},
				"on": {
					"type": "boolean"
				},
				"prohibited_use_policy_url": {
					"type": "string"
				},
				"recipes_not_yet_indexed": {
					"type": "integer"
				},
				"state": {
					"enum": [
						"unasked",
						"declined",
						"accepted",
						"on"
					]
				},
				"terms_url": {
					"type": "string"
				},
				"terms_version": {
					"type": "string"
				}
			},
			"required": [
				"state",
				"on",
				"offer",
				"may_change",
				"model",
				"terms_url",
				"prohibited_use_policy_url",
				"terms_version",
				"accepted_by",
				"accepted_via_access_key",
				"accepted_at",
				"declined_at",
				"model_present",
				"indexed_at",
				"recipes_not_yet_indexed"
			],
			"type": "object"
		}
	},
	{
		"name": "accept_meaning_search_terms",
		"summary": "Accept the terms of the model Meaning Search needs. Kamosu ships no weights (ADR 0029): the person who accepts the terms is the person the terms are about, and the acceptance keeps the Hand that made it and whether it arrived by login or by Access Key. Available at both Doors — a web-only carve-out would be the first hole in Parity, and would stop nothing anyway.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"state": {
					"enum": [
						"unasked",
						"declined",
						"accepted",
						"on"
					]
				}
			},
			"required": [
				"state"
			],
			"type": "object"
		}
	},
	{
		"name": "decline_meaning_search",
		"summary": "Decline the model's terms. Meaning Search stays off and the offer is never made again on this instance — a question already answered, asked twice, is a nag.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"state": {
					"enum": [
						"unasked",
						"declined",
						"accepted",
						"on"
					]
				}
			},
			"required": [
				"state"
			],
			"type": "object"
		}
	},
	{
		"name": "download_meaning_model",
		"summary": "Fetch the Meaning Search model into /data, as a Job. No weights ship in the image; this is the only way any arrive, and only after the terms have been accepted. The download is pinned to one revision and verified against a manifest, so a half-finished one is never mistaken for a model.",
		"permission": "operator",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"model": {
					"type": "string"
				},
				"repository": {
					"type": "string"
				},
				"revision": {
					"type": "string"
				}
			},
			"required": [
				"model",
				"repository",
				"revision"
			],
			"type": "object"
		}
	},
	{
		"name": "build_meaning_index",
		"summary": "Read the library into the Meaning Search index, as a Job, and turn Meaning Search on. Incremental: what is read is what the index does not already hold, so the first run is the whole library and every later one is whatever changed. The index is derived from the recipes and can be rebuilt at any time. Kamosu also does this by itself, within the minute, whenever a recipe changes.",
		"permission": "operator",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"indexed": {
					"type": "integer"
				}
			},
			"required": [
				"indexed"
			],
			"type": "object"
		}
	},
	{
		"name": "turn_off_meaning_search",
		"summary": "Stop matching on meaning and throw the index away. Discards nothing that cannot be rebuilt — the index is derived from the recipes — and keeps both the acceptance, which is history, and the downloaded weights, so turning it back on is a rebuild rather than another download.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"state": {
					"enum": [
						"unasked",
						"declined",
						"accepted",
						"on"
					]
				}
			},
			"required": [
				"state"
			],
			"type": "object"
		}
	},
	{
		"name": "get_recipe",
		"summary": "Read a Recipe: the Branch as it stands and its whole chain of Versions, oldest first. Each Version's `measured` lines are scaled to `wanted_yield` where one is given (null for the recipe as written), and otherwise to the Yield the caller's own In Progress Attempt is cooking to; `scaled_to` says which, or is null where the amounts are as written. Nothing is stored.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"wanted_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"cookbook": {
					"additionalProperties": false,
					"properties": {
						"authors": {
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
						"id": {
							"type": "string"
						},
						"name": {
							"type": [
								"string",
								"null"
							]
						}
					},
					"required": [
						"id",
						"name",
						"authors"
					],
					"type": "object"
				},
				"cooked": {
					"additionalProperties": false,
					"properties": {
						"count": {
							"minimum": 0,
							"type": "integer"
						},
						"last_cooked_at": {
							"type": [
								"string",
								"null"
							]
						},
						"ratings": {
							"items": {
								"additionalProperties": false,
								"properties": {
									"at": {
										"type": "string"
									},
									"name": {
										"type": "string"
									},
									"person_id": {
										"type": "string"
									},
									"rating": {
										"enum": [
											"again",
											"tweak",
											"no"
										],
										"type": "string"
									}
								},
								"required": [
									"person_id",
									"name",
									"rating",
									"at"
								],
								"type": "object"
							},
							"type": "array"
						}
					},
					"required": [
						"count",
						"last_cooked_at",
						"ratings"
					],
					"type": "object"
				},
				"hand_id": {
					"type": "string"
				},
				"head_version_id": {
					"type": "string"
				},
				"language": {
					"type": "string"
				},
				"lineage_id": {
					"type": "string"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
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
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"language_fallback": {
								"type": "boolean"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"type": [
									"string",
									"null"
								]
							},
							"title": {
								"type": "string"
							}
						},
						"required": [
							"lineage_id",
							"branch_id",
							"title",
							"main_photo",
							"language",
							"language_fallback"
						],
						"type": "object"
					},
					"type": "array"
				},
				"tags": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"cookbook_id": {
								"type": "string"
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
							"language_fallback": {
								"type": "boolean"
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
							"recipes": {
								"type": "integer"
							}
						},
						"required": [
							"id",
							"cookbook_id",
							"name",
							"language",
							"names",
							"recipes",
							"language_fallback"
						],
						"type": "object"
					},
					"type": "array"
				},
				"translation": {
					"additionalProperties": false,
					"properties": {
						"source_branch_id": {
							"description": "The recipe this one translates. Null where that recipe is not on this instance — a Translation may arrive on its own, and how far behind it has fallen is then unanswerable rather than zero.",
							"type": [
								"string",
								"null"
							]
						},
						"translates_version_id": {
							"description": "The Version of the source this recipe's newest Version renders.",
							"type": "string"
						},
						"versions_behind": {
							"description": "How many Versions the source has moved on since the one this translates. Zero means up to date.",
							"type": [
								"integer",
								"null"
							]
						}
					},
					"required": [
						"translates_version_id",
						"source_branch_id",
						"versions_behind"
					],
					"type": [
						"object",
						"null"
					]
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
							"components": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
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
												"nutrition": {
													"additionalProperties": false,
													"properties": {
														"basis": {
															"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
															"enum": [
																"per_serving",
																"per_100g"
															]
														},
														"calories": {
															"description": "Calories, zero or more.",
															"minimum": 0,
															"type": "number"
														}
													},
													"required": [
														"calories",
														"basis"
													],
													"type": [
														"object",
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
												"nutrition",
												"ingredients",
												"steps"
											],
											"type": [
												"object",
												"null"
											]
										},
										"held": {
											"type": "boolean"
										},
										"lineage_id": {
											"type": "string"
										},
										"measured": {
											"additionalProperties": false,
											"properties": {
												"ingredients": {
													"items": {
														"type": [
															"string",
															"null"
														]
													},
													"type": "array"
												},
												"steps": {
													"items": {
														"type": [
															"string",
															"null"
														]
													},
													"type": "array"
												}
											},
											"required": [
												"ingredients",
												"steps"
											],
											"type": [
												"object",
												"null"
											]
										},
										"path": {
											"items": {
												"type": "integer"
											},
											"type": "array"
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
													"lineage_id": {
														"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
													"target",
													"lineage_id"
												],
												"type": [
													"object",
													"null"
												]
											},
											"type": [
												"array",
												"null"
											]
										},
										"said": {
											"type": "string"
										},
										"share": {
											"type": [
												"number",
												"null"
											]
										},
										"stopped": {
											"type": "boolean"
										},
										"title": {
											"type": [
												"string",
												"null"
											]
										}
									},
									"required": [
										"path",
										"lineage_id",
										"held",
										"stopped",
										"branch_id",
										"title",
										"share",
										"said",
										"content",
										"readings",
										"measured"
									],
									"type": "object"
								},
								"type": "array"
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
									"nutrition": {
										"additionalProperties": false,
										"properties": {
											"basis": {
												"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
												"enum": [
													"per_serving",
													"per_100g"
												]
											},
											"calories": {
												"description": "Calories, zero or more.",
												"minimum": 0,
												"type": "number"
											}
										},
										"required": [
											"calories",
											"basis"
										],
										"type": [
											"object",
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
									"nutrition",
									"ingredients",
									"steps"
								],
								"type": "object"
							},
							"cooking": {
								"additionalProperties": false,
								"properties": {
									"steps": {
										"items": {
											"additionalProperties": false,
											"properties": {
												"timer_seconds": {
													"minimum": 1,
													"type": [
														"integer",
														"null"
													]
												},
												"uses": {
													"items": {
														"minimum": 0,
														"type": "integer"
													},
													"type": "array"
												}
											},
											"required": [
												"uses",
												"timer_seconds"
											],
											"type": [
												"object",
												"null"
											]
										},
										"type": "array"
									}
								},
								"required": [
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
							"language": {
								"type": [
									"string",
									"null"
								]
							},
							"measured": {
								"additionalProperties": false,
								"properties": {
									"ingredients": {
										"items": {
											"type": [
												"string",
												"null"
											]
										},
										"type": "array"
									},
									"steps": {
										"items": {
											"type": [
												"string",
												"null"
											]
										},
										"type": "array"
									}
								},
								"required": [
									"ingredients",
									"steps"
								],
								"type": "object"
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
										"lineage_id": {
											"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
										"target",
										"lineage_id"
									],
									"type": [
										"object",
										"null"
									]
								},
								"type": "array"
							},
							"scaled_to": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"sequence": {
								"type": "integer"
							},
							"translates_version_id": {
								"type": [
									"string",
									"null"
								]
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
							"readings",
							"measured",
							"cooking",
							"components",
							"translates_version_id",
							"language",
							"scaled_to"
						],
						"type": "object"
					},
					"type": "array"
				},
				"writes": {
					"type": "boolean"
				}
			},
			"required": [
				"branch_id",
				"lineage_id",
				"cookbook",
				"name",
				"writes",
				"hand_id",
				"language",
				"origin_address",
				"head_version_id",
				"versions",
				"translation",
				"tags",
				"related_recipes",
				"cooked"
			],
			"type": "object"
		}
	},
	{
		"name": "get_thread",
		"summary": "Read the Thread: every Version of every Branch of one Lineage this Person can see, oldest first per Branch, with every Attempt hanging off it. branch_id is only the entry point — any Branch of the Lineage answers the same Thread.",
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
				"attempts": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"as_cooked": {
								"additionalProperties": false,
								"properties": {
									"against": {
										"additionalProperties": false,
										"properties": {
											"ingredients": {
												"items": {
													"additionalProperties": false,
													"properties": {
														"from_branch_point": {
															"type": "boolean"
														},
														"kind": {
															"type": "string"
														},
														"mine": {
															"additionalProperties": false,
															"properties": {
																"index": {
																	"type": "integer"
																},
																"kind": {
																	"type": "string"
																},
																"text": {
																	"type": "string"
																}
															},
															"required": [
																"kind",
																"text",
																"index"
															],
															"type": [
																"object",
																"null"
															]
														},
														"state": {
															"enum": [
																"same",
																"changed",
																"only-mine",
																"only-theirs"
															]
														},
														"theirs": {
															"additionalProperties": false,
															"properties": {
																"index": {
																	"type": "integer"
																},
																"kind": {
																	"type": "string"
																},
																"text": {
																	"type": "string"
																}
															},
															"required": [
																"kind",
																"text",
																"index"
															],
															"type": [
																"object",
																"null"
															]
														}
													},
													"required": [
														"kind",
														"state",
														"from_branch_point",
														"mine",
														"theirs"
													],
													"type": "object"
												},
												"type": "array"
											},
											"steps": {
												"items": {
													"additionalProperties": false,
													"properties": {
														"from_branch_point": {
															"type": "boolean"
														},
														"kind": {
															"type": "string"
														},
														"mine": {
															"additionalProperties": false,
															"properties": {
																"index": {
																	"type": "integer"
																},
																"kind": {
																	"type": "string"
																},
																"text": {
																	"type": "string"
																}
															},
															"required": [
																"kind",
																"text",
																"index"
															],
															"type": [
																"object",
																"null"
															]
														},
														"state": {
															"enum": [
																"same",
																"changed",
																"only-mine",
																"only-theirs"
															]
														},
														"theirs": {
															"additionalProperties": false,
															"properties": {
																"index": {
																	"type": "integer"
																},
																"kind": {
																	"type": "string"
																},
																"text": {
																	"type": "string"
																}
															},
															"required": [
																"kind",
																"text",
																"index"
															],
															"type": [
																"object",
																"null"
															]
														}
													},
													"required": [
														"kind",
														"state",
														"from_branch_point",
														"mine",
														"theirs"
													],
													"type": "object"
												},
												"type": "array"
											}
										},
										"required": [
											"ingredients",
											"steps"
										],
										"type": "object"
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
											"nutrition": {
												"additionalProperties": false,
												"properties": {
													"basis": {
														"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
														"enum": [
															"per_serving",
															"per_100g"
														]
													},
													"calories": {
														"description": "Calories, zero or more.",
														"minimum": 0,
														"type": "number"
													}
												},
												"required": [
													"calories",
													"basis"
												],
												"type": [
													"object",
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
											"nutrition",
											"ingredients",
											"steps"
										],
										"type": "object"
									},
									"promotion_declined": {
										"type": "boolean"
									},
									"version_id": {
										"type": "string"
									}
								},
								"required": [
									"version_id",
									"content",
									"against",
									"promotion_declined"
								],
								"type": [
									"object",
									"null"
								]
							},
							"cooking_yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"photographs": {
								"items": {
									"type": "string"
								},
								"type": "array"
							},
							"rating": {
								"enum": [
									"again",
									"tweak",
									"no",
									null
								],
								"type": [
									"string",
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
							"last_action_at",
							"photographs",
							"as_cooked"
						],
						"type": "object"
					},
					"type": "array"
				},
				"branches": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"arrived": {
								"type": "boolean"
							},
							"branch_id": {
								"type": "string"
							},
							"cookbook": {
								"additionalProperties": false,
								"properties": {
									"authors": {
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
									"id": {
										"type": "string"
									},
									"name": {
										"type": [
											"string",
											"null"
										]
									}
								},
								"required": [
									"id",
									"name",
									"authors"
								],
								"type": "object"
							},
							"hand_id": {
								"type": "string"
							},
							"hand_name": {
								"type": [
									"string",
									"null"
								]
							},
							"head_version_id": {
								"type": "string"
							},
							"language": {
								"type": "string"
							},
							"mine": {
								"type": "boolean"
							},
							"name": {
								"type": [
									"string",
									"null"
								]
							},
							"translation": {
								"additionalProperties": false,
								"properties": {
									"source_branch_id": {
										"description": "The recipe this one translates. Null where that recipe is not on this instance — a Translation may arrive on its own, and how far behind it has fallen is then unanswerable rather than zero.",
										"type": [
											"string",
											"null"
										]
									},
									"translates_version_id": {
										"description": "The Version of the source this recipe's newest Version renders.",
										"type": "string"
									},
									"versions_behind": {
										"description": "How many Versions the source has moved on since the one this translates. Zero means up to date.",
										"type": [
											"integer",
											"null"
										]
									}
								},
								"required": [
									"translates_version_id",
									"source_branch_id",
									"versions_behind"
								],
								"type": [
									"object",
									"null"
								]
							}
						},
						"required": [
							"branch_id",
							"cookbook",
							"name",
							"mine",
							"arrived",
							"hand_id",
							"hand_name",
							"language",
							"head_version_id",
							"translation"
						],
						"type": "object"
					},
					"type": "array"
				},
				"lineage_id": {
					"type": "string"
				},
				"versions": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"change_note": {
								"type": [
									"string",
									"null"
								]
							},
							"created_at": {
								"type": "string"
							},
							"hand_id": {
								"type": "string"
							},
							"hand_name": {
								"type": [
									"string",
									"null"
								]
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
							"parent_version_id": {
								"type": [
									"string",
									"null"
								]
							},
							"sequence": {
								"type": "integer"
							},
							"translates_version_id": {
								"type": [
									"string",
									"null"
								]
							},
							"version_id": {
								"type": "string"
							}
						},
						"required": [
							"branch_id",
							"sequence",
							"version_id",
							"parent_version_id",
							"hand_id",
							"hand_name",
							"name",
							"change_note",
							"created_at",
							"translates_version_id",
							"language"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"lineage_id",
				"branches",
				"versions",
				"attempts"
			],
			"type": "object"
		}
	},
	{
		"name": "share_recipe",
		"summary": "Turn a Recipe's Share Link on, and answer the link. One permanent, unguessable address per Recipe, never expiring, freely passed on. Asking twice for a Recipe already shared answers the link it already has rather than minting a second one. The link's Secret is answered exactly once — here, at the moment it is minted — because only its hash is stored. The instance's public address is asked for at the first Share Link and stored once; a link is kept as a token rather than a URL, so setting the address later makes every link already minted render correctly.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"public_address": {
					"description": "Where this instance is reachable from outside, e.g. https://kamosu.example.com — asked at the first Share Link and stored once. Ignored where an address is already stored; `set_public_address` is how one is changed.",
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
				"created_at": {
					"type": [
						"string",
						"null"
					]
				},
				"public_address": {
					"type": [
						"string",
						"null"
					]
				},
				"share_id": {
					"type": [
						"string",
						"null"
					]
				},
				"shared": {
					"description": "Whether a live Share Link exists. This is the whole of Visibility: there is no scale and no third audience.",
					"type": "boolean"
				},
				"shared_by": {
					"description": "The Name of the Person who turned the link on, looked up live. Never a Kitchen: a Kitchen's Nickname is private to the member who set it and could not appear on a public page.",
					"type": [
						"string",
						"null"
					]
				},
				"url": {
					"description": "The link itself, answered once, at the moment it is minted.",
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"shared",
				"share_id",
				"url",
				"shared_by",
				"created_at",
				"public_address"
			],
			"type": "object"
		}
	},
	{
		"name": "end_share_link",
		"summary": "End a Recipe's Share Link. Permanent: the link stops working and turning sharing back on mints a new one, so a withdrawn link stays dead. It reaches no copy already sent, and Kamosu says so rather than letting that be discovered.",
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
				"created_at": {
					"type": [
						"string",
						"null"
					]
				},
				"public_address": {
					"type": [
						"string",
						"null"
					]
				},
				"share_id": {
					"type": [
						"string",
						"null"
					]
				},
				"shared": {
					"description": "Whether a live Share Link exists. This is the whole of Visibility: there is no scale and no third audience.",
					"type": "boolean"
				},
				"shared_by": {
					"description": "The Name of the Person who turned the link on, looked up live. Never a Kitchen: a Kitchen's Nickname is private to the member who set it and could not appear on a public page.",
					"type": [
						"string",
						"null"
					]
				},
				"url": {
					"description": "The link itself, answered once, at the moment it is minted.",
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"shared",
				"share_id",
				"url",
				"shared_by",
				"created_at",
				"public_address"
			],
			"type": "object"
		}
	},
	{
		"name": "get_share_link",
		"summary": "Whether a Recipe is shared, and by whom. The link's URL is answered only at the moment it is minted, since only the Secret's hash is stored — so this says a link exists without being able to reprint it.",
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
				"created_at": {
					"type": [
						"string",
						"null"
					]
				},
				"public_address": {
					"type": [
						"string",
						"null"
					]
				},
				"share_id": {
					"type": [
						"string",
						"null"
					]
				},
				"shared": {
					"description": "Whether a live Share Link exists. This is the whole of Visibility: there is no scale and no third audience.",
					"type": "boolean"
				},
				"shared_by": {
					"description": "The Name of the Person who turned the link on, looked up live. Never a Kitchen: a Kitchen's Nickname is private to the member who set it and could not appear on a public page.",
					"type": [
						"string",
						"null"
					]
				},
				"url": {
					"description": "The link itself, answered once, at the moment it is minted.",
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"shared",
				"share_id",
				"url",
				"shared_by",
				"created_at",
				"public_address"
			],
			"type": "object"
		}
	},
	{
		"name": "get_public_address",
		"summary": "Where this instance currently says it is reachable from outside, or nothing if it has never been asked. The Operator's half of `set_public_address`: changing an address you cannot see is a guess.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"public_address": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"public_address"
			],
			"type": "object"
		}
	},
	{
		"name": "set_public_address",
		"summary": "Change where this instance says it is reachable from outside. Kept in the database and never in an environment variable, so moving an instance is one act rather than a redeployment. It fixes the future, not the past: Share Links minted after it carry the new address, while a link already sent stays the text it was sent as and cannot be reissued — only the secret's hash is kept, so Kamosu can no longer print that link at all.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"public_address": {
					"type": "string"
				}
			},
			"required": [
				"public_address"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"public_address": {
					"type": "string"
				}
			},
			"required": [
				"public_address"
			],
			"type": "object"
		}
	},
	{
		"name": "export_bundle",
		"summary": "Write a Bundle of one recipe: a plain zip holding a readable Markdown note per recipe with its Thread beneath it, its Photographs, and a hidden .kamosu/ sidecar carrying every Version complete back to the first, the Readings and the ids. It carries the Branch named, its Translations, and every Component it needs as a Passenger. This answers what the Bundle holds; fetch its bytes at GET /api/bundles/<branch_id> under the same Credential. Nothing is sent anywhere and nothing is changed.",
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
				"fetch_at": {
					"type": "string"
				},
				"file_name": {
					"type": "string"
				},
				"missing_photographs": {
					"items": {
						"type": "string"
					},
					"type": "array"
				},
				"notes": {
					"items": {
						"type": "string"
					},
					"type": "array"
				},
				"passengers": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"lineage_id": {
								"type": "string"
							},
							"title": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"lineage_id",
							"title"
						],
						"type": "object"
					},
					"type": "array"
				},
				"photographs": {
					"type": "integer"
				},
				"subjects": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"lineage_id": {
								"type": "string"
							},
							"title": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"lineage_id",
							"title"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"file_name",
				"fetch_at",
				"subjects",
				"passengers",
				"notes",
				"photographs",
				"missing_photographs"
			],
			"type": "object"
		}
	},
	{
		"name": "make_sheet",
		"summary": "Set a Sheet of one recipe: the Branch as it stands on this Person's screen, set for paper as a PDF. It carries the recipe and not the library — no Tags, Attempts, Thread or past Versions. Written Ingredient Lines are printed and Readings are not, except the amount beneath a line when a cooking has scaled the recipe; Components unfold after it, parent first, each already scaled. Letter for US Reading Measures, A4 otherwise. `wanted_yield` is the Yield the screen is scaled to, as `get_recipe` takes it. When the Job completes, fetch the PDF at GET /api/sheets/<job_id> under the same Credential. Nothing is changed.",
		"permission": "person",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"wanted_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"branch_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"fetch_at": {
					"type": "string"
				},
				"file_name": {
					"type": "string"
				},
				"kept_as": {
					"type": "string"
				},
				"pages": {
					"minimum": 1,
					"type": "integer"
				},
				"paper": {
					"enum": [
						"a4",
						"us-letter"
					]
				}
			},
			"required": [
				"file_name",
				"fetch_at",
				"paper",
				"pages",
				"kept_as"
			],
			"type": "object"
		}
	},
	{
		"name": "make_shared_sheet",
		"summary": "Set a Sheet of the recipe a Share Link shows, for anyone holding the link — no account needed. The recipe is printed as written, with its Components unfolded after it at the amount each line asks for. `language` picks one of the link's Translations; `locale` is the reader's locale (a US or Canadian one prints Letter, anything else A4) and decides nothing but the paper. When the Job completes, fetch the PDF at GET /api/sheets/<job_id>.",
		"permission": "public",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"language": {
					"type": "string"
				},
				"locale": {
					"type": "string"
				},
				"token": {
					"type": "string"
				}
			},
			"required": [
				"token"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"fetch_at": {
					"type": "string"
				},
				"file_name": {
					"type": "string"
				},
				"kept_as": {
					"type": "string"
				},
				"pages": {
					"minimum": 1,
					"type": "integer"
				},
				"paper": {
					"enum": [
						"a4",
						"us-letter"
					]
				}
			},
			"required": [
				"file_name",
				"fetch_at",
				"paper",
				"pages",
				"kept_as"
			],
			"type": "object"
		}
	},
	{
		"name": "import_bundle",
		"summary": "Receive a Bundle into your own Cookbook, as a Job. Every recipe it carries is placed under the sender's Hands and travels on under the sender's ids, its Versions, Readings and Photographs exactly as they were sent, while your Cookbook holds it under an id of this instance's own; one your Cookbook already holds is extended by whatever the Bundle carries past it, so the same friend's next Bundle continues their recipe. Another Cookbook here holding it is no part of the question: each Cookbook receives its own copy. Receiving makes nothing of your own — changing what arrived does. A recipe whose history is damaged arrives as a new recipe of your own with no history, and the Import Report says so. Send the file to POST /api/uploads and pass the `upload_id` it answers, or pass it base64-encoded as `data`.",
		"permission": "person",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"data": {
					"description": "The Bundle's zip, base64-encoded — for a Door that can send only JSON.",
					"type": "string"
				},
				"upload_id": {
					"description": "The id POST /api/uploads answered for the Bundle. Used once, then deleted.",
					"type": "string"
				}
			},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"arrived": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"bare": {
								"description": "It came as a bare name (and usually a link): no Ingredient Line and no Step. A real recipe that arrived correctly, not a failure.",
								"type": "boolean"
							},
							"branch_id": {
								"description": "The Branch as this instance holds it — the id every other Operation and every URL takes. On a recipe received from elsewhere it is not the id it travelled under.",
								"type": "string"
							},
							"foreign_id": {
								"description": "What the source called this recipe: from a Bundle, the Branch id it travels under, which is the sender's.",
								"type": "string"
							},
							"lineage_id": {
								"type": "string"
							},
							"main_photo": {
								"description": "The Photograph the recipe arrived with as its Main Photo, or null.",
								"type": [
									"string",
									"null"
								]
							},
							"status": {
								"enum": [
									"created",
									"extended",
									"unchanged"
								]
							},
							"subject": {
								"description": "From a Bundle: whether this recipe is what the Bundle is about, rather than a Passenger that travelled because something needed it.",
								"type": "boolean"
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
				"cookbook_id": {
					"type": "string"
				},
				"import_id": {
					"type": "string"
				},
				"left_out": {
					"description": "What the importer read and deliberately did not bring in, recipe by recipe: `site_icon` is a source site's favicon, which is not a photograph of the dish (ADR 0017); `extra_photos` are pictures past the first, since a recipe keeps one Main Photo and Kamosu does not guess which Step another shows; `unreadable_photos` could not be decoded.",
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"count": {
								"type": "integer"
							},
							"foreign_id": {
								"type": "string"
							},
							"icon": {
								"description": "For a `site_icon`: the icon itself as a `data:` URI, so the Report can show what was left out. Carried only here, never stored as a Photograph.",
								"type": "string"
							},
							"title": {
								"type": "string"
							},
							"what": {
								"enum": [
									"site_icon",
									"extra_photos",
									"unreadable_photos"
								]
							}
						},
						"required": [
							"foreign_id",
							"branch_id",
							"title",
							"what",
							"count"
						],
						"type": "object"
					},
					"type": "array"
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
				"related_candidates": {
					"description": "Pairs of recipes this Import landed that share a name or a web page, offered as Related Recipes to tick with `set_related_recipe` — never joined into one Lineage (ADR 0025). A pair already related is not offered.",
					"items": {
						"additionalProperties": false,
						"properties": {
							"recipes": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"ingredients": {
											"type": "integer"
										},
										"lineage_id": {
											"type": "string"
										},
										"main_photo": {
											"type": [
												"string",
												"null"
											]
										},
										"title": {
											"type": "string"
										}
									},
									"required": [
										"lineage_id",
										"branch_id",
										"title",
										"main_photo",
										"ingredients"
									],
									"type": "object"
								},
								"type": "array"
							},
							"shared": {
								"items": {
									"enum": [
										"name",
										"page"
									]
								},
								"type": "array"
							}
						},
						"required": [
							"recipes",
							"shared"
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
							"kept_as": {
								"additionalProperties": false,
								"description": "From a damaged Bundle: the new recipe of your own its words were kept as, with no history and no link to the recipe it came from.",
								"properties": {
									"branch_id": {
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
									"lineage_id",
									"branch_id",
									"title"
								],
								"type": "object"
							},
							"name": {
								"description": "What the source called it, where even its own id could not be read — a Crouton file's name.",
								"type": "string"
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
				"cookbook_id",
				"source_kind",
				"arrived",
				"offered",
				"unreadable",
				"left_out",
				"related_candidates"
			],
			"type": "object"
		}
	},
	{
		"name": "read_shared_recipe",
		"summary": "Read a Recipe through its Share Link token: the Recipe as it stands, its Translations, and its Thread complete back to the first Version with every name and *what changed* line. Never an Attempt, a rating or an Attempt photograph. Public, because holding the token is the whole of the permission — this is what the Share Link page consumes, and the page is not an Operation, so Parity is untouched.",
		"permission": "public",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"token": {
					"type": "string"
				}
			},
			"required": [
				"token"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"ended": {
					"description": "Whether this link has been ended. A token nobody minted is not found; a real token whose link was withdrawn answers here, because 'no such page' reads as a mistake to retry.",
					"type": "boolean"
				},
				"public_address": {
					"type": [
						"string",
						"null"
					]
				},
				"recipe": {
					"additionalProperties": false,
					"properties": {
						"branch_id": {
							"type": "string"
						},
						"components": {
							"items": {
								"additionalProperties": false,
								"properties": {
									"branch_id": {
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
											"nutrition": {
												"additionalProperties": false,
												"properties": {
													"basis": {
														"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
														"enum": [
															"per_serving",
															"per_100g"
														]
													},
													"calories": {
														"description": "Calories, zero or more.",
														"minimum": 0,
														"type": "number"
													}
												},
												"required": [
													"calories",
													"basis"
												],
												"type": [
													"object",
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
											"nutrition",
											"ingredients",
											"steps"
										],
										"type": [
											"object",
											"null"
										]
									},
									"held": {
										"type": "boolean"
									},
									"lineage_id": {
										"type": "string"
									},
									"measured": {
										"type": "null"
									},
									"path": {
										"items": {
											"type": "integer"
										},
										"type": "array"
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
												"lineage_id": {
													"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
												"target",
												"lineage_id"
											],
											"type": [
												"object",
												"null"
											]
										},
										"type": [
											"array",
											"null"
										]
									},
									"said": {
										"type": "string"
									},
									"share": {
										"type": [
											"number",
											"null"
										]
									},
									"stopped": {
										"type": "boolean"
									},
									"title": {
										"type": [
											"string",
											"null"
										]
									}
								},
								"required": [
									"path",
									"lineage_id",
									"held",
									"stopped",
									"branch_id",
									"title",
									"share",
									"said",
									"content",
									"readings",
									"measured"
								],
								"type": "object"
							},
							"type": "array"
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
								"nutrition": {
									"additionalProperties": false,
									"properties": {
										"basis": {
											"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
											"enum": [
												"per_serving",
												"per_100g"
											]
										},
										"calories": {
											"description": "Calories, zero or more.",
											"minimum": 0,
											"type": "number"
										}
									},
									"required": [
										"calories",
										"basis"
									],
									"type": [
										"object",
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
								"nutrition",
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"language": {
							"type": "string"
						},
						"lineage_id": {
							"description": "What a Cover is drawn from, for a recipe with no photograph — the Lineage id and nothing else (#46).",
							"type": "string"
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
									"lineage_id": {
										"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
									"target",
									"lineage_id"
								],
								"type": [
									"object",
									"null"
								]
							},
							"type": "array"
						},
						"version_id": {
							"type": "string"
						}
					},
					"required": [
						"branch_id",
						"lineage_id",
						"version_id",
						"language",
						"content",
						"readings",
						"components"
					],
					"type": [
						"object",
						"null"
					]
				},
				"share_id": {
					"type": "string"
				},
				"shared_by": {
					"type": [
						"string",
						"null"
					]
				},
				"thread": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"change_note": {
								"type": [
									"string",
									"null"
								]
							},
							"created_at": {
								"type": "string"
							},
							"hand": {
								"description": "The Name of the Person who wrote this Version. What makes credit travel with a recipe.",
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
							"sequence",
							"name",
							"change_note",
							"hand",
							"created_at"
						],
						"type": "object"
					},
					"type": "array"
				},
				"translations": {
					"description": "The Branches of this Lineage in another Language that translate the Branch shared (ADR 0006). Carried whole rather than as links: each is a Branch of its own, and a token per Translation would be a second link to end.",
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"components": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
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
												"nutrition": {
													"additionalProperties": false,
													"properties": {
														"basis": {
															"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
															"enum": [
																"per_serving",
																"per_100g"
															]
														},
														"calories": {
															"description": "Calories, zero or more.",
															"minimum": 0,
															"type": "number"
														}
													},
													"required": [
														"calories",
														"basis"
													],
													"type": [
														"object",
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
												"nutrition",
												"ingredients",
												"steps"
											],
											"type": [
												"object",
												"null"
											]
										},
										"held": {
											"type": "boolean"
										},
										"lineage_id": {
											"type": "string"
										},
										"measured": {
											"type": "null"
										},
										"path": {
											"items": {
												"type": "integer"
											},
											"type": "array"
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
													"lineage_id": {
														"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
													"target",
													"lineage_id"
												],
												"type": [
													"object",
													"null"
												]
											},
											"type": [
												"array",
												"null"
											]
										},
										"said": {
											"type": "string"
										},
										"share": {
											"type": [
												"number",
												"null"
											]
										},
										"stopped": {
											"type": "boolean"
										},
										"title": {
											"type": [
												"string",
												"null"
											]
										}
									},
									"required": [
										"path",
										"lineage_id",
										"held",
										"stopped",
										"branch_id",
										"title",
										"share",
										"said",
										"content",
										"readings",
										"measured"
									],
									"type": "object"
								},
								"type": "array"
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
									"nutrition": {
										"additionalProperties": false,
										"properties": {
											"basis": {
												"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
												"enum": [
													"per_serving",
													"per_100g"
												]
											},
											"calories": {
												"description": "Calories, zero or more.",
												"minimum": 0,
												"type": "number"
											}
										},
										"required": [
											"calories",
											"basis"
										],
										"type": [
											"object",
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
									"nutrition",
									"ingredients",
									"steps"
								],
								"type": "object"
							},
							"language": {
								"type": "string"
							},
							"lineage_id": {
								"description": "What a Cover is drawn from, for a recipe with no photograph — the Lineage id and nothing else (#46).",
								"type": "string"
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
										"lineage_id": {
											"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
										"target",
										"lineage_id"
									],
									"type": [
										"object",
										"null"
									]
								},
								"type": "array"
							},
							"version_id": {
								"type": "string"
							}
						},
						"required": [
							"branch_id",
							"lineage_id",
							"version_id",
							"language",
							"content",
							"readings",
							"components"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"ended",
				"share_id",
				"shared_by",
				"public_address",
				"recipe",
				"translations",
				"thread"
			],
			"type": "object"
		}
	},
	{
		"name": "branch_point",
		"summary": "The last Version two Branches share, found by walking both chains back until they meet — never declared, always computed. A chain that does not converge on a shared first Version answers a damaged-Bundle error rather than a guess.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_a_id": {
					"type": "string"
				},
				"branch_b_id": {
					"type": "string"
				}
			},
			"required": [
				"branch_a_id",
				"branch_b_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"version_id": {
					"type": "string"
				}
			},
			"required": [
				"version_id"
			],
			"type": "object"
		}
	},
	{
		"name": "divergence",
		"summary": "Two Branches of one Lineage laid over each other, so a screen can show two whole recipes with a switch between them rather than a difference (ADR 0014). Every row carries both sides' own words; a line only one side has is a Ghost. Which line is which is read against the Branch Point, never by an id stapled to a line (ADR 0019), and an uncertain reading declines to pair rather than claiming a connection.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"other_branch_id": {
					"type": "string"
				}
			},
			"required": [
				"branch_id",
				"other_branch_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_point_version_id": {
					"type": "string"
				},
				"fields": {
					"additionalProperties": false,
					"properties": {
						"cook_time_minutes": {
							"additionalProperties": false,
							"properties": {
								"mine": {},
								"same": {
									"type": "boolean"
								},
								"theirs": {}
							},
							"required": [
								"same",
								"mine",
								"theirs"
							],
							"type": "object"
						},
						"main_photo": {
							"additionalProperties": false,
							"properties": {
								"mine": {},
								"same": {
									"type": "boolean"
								},
								"theirs": {}
							},
							"required": [
								"same",
								"mine",
								"theirs"
							],
							"type": "object"
						},
						"note": {
							"additionalProperties": false,
							"properties": {
								"mine": {},
								"same": {
									"type": "boolean"
								},
								"theirs": {}
							},
							"required": [
								"same",
								"mine",
								"theirs"
							],
							"type": "object"
						},
						"nutrition": {
							"additionalProperties": false,
							"properties": {
								"mine": {},
								"same": {
									"type": "boolean"
								},
								"theirs": {}
							},
							"required": [
								"same",
								"mine",
								"theirs"
							],
							"type": "object"
						},
						"prep_time_minutes": {
							"additionalProperties": false,
							"properties": {
								"mine": {},
								"same": {
									"type": "boolean"
								},
								"theirs": {}
							},
							"required": [
								"same",
								"mine",
								"theirs"
							],
							"type": "object"
						},
						"source": {
							"additionalProperties": false,
							"properties": {
								"mine": {},
								"same": {
									"type": "boolean"
								},
								"theirs": {}
							},
							"required": [
								"same",
								"mine",
								"theirs"
							],
							"type": "object"
						},
						"title": {
							"additionalProperties": false,
							"properties": {
								"mine": {},
								"same": {
									"type": "boolean"
								},
								"theirs": {}
							},
							"required": [
								"same",
								"mine",
								"theirs"
							],
							"type": "object"
						},
						"yield": {
							"additionalProperties": false,
							"properties": {
								"mine": {},
								"same": {
									"type": "boolean"
								},
								"theirs": {}
							},
							"required": [
								"same",
								"mine",
								"theirs"
							],
							"type": "object"
						}
					},
					"required": [
						"title",
						"yield",
						"prep_time_minutes",
						"cook_time_minutes",
						"source",
						"note",
						"nutrition",
						"main_photo"
					],
					"type": "object"
				},
				"ingredients": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"from_branch_point": {
								"type": "boolean"
							},
							"kind": {
								"type": "string"
							},
							"mine": {
								"additionalProperties": false,
								"properties": {
									"index": {
										"type": "integer"
									},
									"kind": {
										"type": "string"
									},
									"text": {
										"type": "string"
									}
								},
								"required": [
									"kind",
									"text",
									"index"
								],
								"type": [
									"object",
									"null"
								]
							},
							"state": {
								"enum": [
									"same",
									"changed",
									"only-mine",
									"only-theirs"
								]
							},
							"theirs": {
								"additionalProperties": false,
								"properties": {
									"index": {
										"type": "integer"
									},
									"kind": {
										"type": "string"
									},
									"text": {
										"type": "string"
									}
								},
								"required": [
									"kind",
									"text",
									"index"
								],
								"type": [
									"object",
									"null"
								]
							}
						},
						"required": [
							"kind",
							"state",
							"from_branch_point",
							"mine",
							"theirs"
						],
						"type": "object"
					},
					"type": "array"
				},
				"lineage_id": {
					"type": "string"
				},
				"mine": {
					"additionalProperties": false,
					"properties": {
						"arrived": {
							"type": "boolean"
						},
						"branch_id": {
							"type": "string"
						},
						"components": {
							"items": {
								"additionalProperties": false,
								"properties": {
									"branch_id": {
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
											"nutrition": {
												"additionalProperties": false,
												"properties": {
													"basis": {
														"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
														"enum": [
															"per_serving",
															"per_100g"
														]
													},
													"calories": {
														"description": "Calories, zero or more.",
														"minimum": 0,
														"type": "number"
													}
												},
												"required": [
													"calories",
													"basis"
												],
												"type": [
													"object",
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
											"nutrition",
											"ingredients",
											"steps"
										],
										"type": [
											"object",
											"null"
										]
									},
									"held": {
										"type": "boolean"
									},
									"lineage_id": {
										"type": "string"
									},
									"measured": {
										"additionalProperties": false,
										"properties": {
											"ingredients": {
												"items": {
													"type": [
														"string",
														"null"
													]
												},
												"type": "array"
											},
											"steps": {
												"items": {
													"type": [
														"string",
														"null"
													]
												},
												"type": "array"
											}
										},
										"required": [
											"ingredients",
											"steps"
										],
										"type": [
											"object",
											"null"
										]
									},
									"path": {
										"items": {
											"type": "integer"
										},
										"type": "array"
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
												"lineage_id": {
													"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
												"target",
												"lineage_id"
											],
											"type": [
												"object",
												"null"
											]
										},
										"type": [
											"array",
											"null"
										]
									},
									"said": {
										"type": "string"
									},
									"share": {
										"type": [
											"number",
											"null"
										]
									},
									"stopped": {
										"type": "boolean"
									},
									"title": {
										"type": [
											"string",
											"null"
										]
									}
								},
								"required": [
									"path",
									"lineage_id",
									"held",
									"stopped",
									"branch_id",
									"title",
									"share",
									"said",
									"content",
									"readings",
									"measured"
								],
								"type": "object"
							},
							"type": "array"
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
								"nutrition": {
									"additionalProperties": false,
									"properties": {
										"basis": {
											"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
											"enum": [
												"per_serving",
												"per_100g"
											]
										},
										"calories": {
											"description": "Calories, zero or more.",
											"minimum": 0,
											"type": "number"
										}
									},
									"required": [
										"calories",
										"basis"
									],
									"type": [
										"object",
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
								"nutrition",
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"cookbook": {
							"additionalProperties": false,
							"properties": {
								"authors": {
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
								"id": {
									"type": "string"
								},
								"name": {
									"type": [
										"string",
										"null"
									]
								}
							},
							"required": [
								"id",
								"name",
								"authors"
							],
							"type": "object"
						},
						"hand_id": {
							"type": "string"
						},
						"hand_name": {
							"type": [
								"string",
								"null"
							]
						},
						"head_version_id": {
							"type": "string"
						},
						"language": {
							"type": "string"
						},
						"measured": {
							"additionalProperties": false,
							"properties": {
								"ingredients": {
									"items": {
										"type": [
											"string",
											"null"
										]
									},
									"type": "array"
								},
								"steps": {
									"items": {
										"type": [
											"string",
											"null"
										]
									},
									"type": "array"
								}
							},
							"required": [
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"mine": {
							"type": "boolean"
						},
						"name": {
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
									"lineage_id": {
										"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
									"target",
									"lineage_id"
								],
								"type": [
									"object",
									"null"
								]
							},
							"type": "array"
						}
					},
					"required": [
						"branch_id",
						"cookbook",
						"name",
						"mine",
						"arrived",
						"hand_id",
						"hand_name",
						"language",
						"head_version_id",
						"content",
						"readings",
						"measured",
						"components"
					],
					"type": "object"
				},
				"steps": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"from_branch_point": {
								"type": "boolean"
							},
							"kind": {
								"type": "string"
							},
							"mine": {
								"additionalProperties": false,
								"properties": {
									"index": {
										"type": "integer"
									},
									"kind": {
										"type": "string"
									},
									"text": {
										"type": "string"
									}
								},
								"required": [
									"kind",
									"text",
									"index"
								],
								"type": [
									"object",
									"null"
								]
							},
							"state": {
								"enum": [
									"same",
									"changed",
									"only-mine",
									"only-theirs"
								]
							},
							"theirs": {
								"additionalProperties": false,
								"properties": {
									"index": {
										"type": "integer"
									},
									"kind": {
										"type": "string"
									},
									"text": {
										"type": "string"
									}
								},
								"required": [
									"kind",
									"text",
									"index"
								],
								"type": [
									"object",
									"null"
								]
							}
						},
						"required": [
							"kind",
							"state",
							"from_branch_point",
							"mine",
							"theirs"
						],
						"type": "object"
					},
					"type": "array"
				},
				"theirs": {
					"additionalProperties": false,
					"properties": {
						"arrived": {
							"type": "boolean"
						},
						"branch_id": {
							"type": "string"
						},
						"components": {
							"items": {
								"additionalProperties": false,
								"properties": {
									"branch_id": {
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
											"nutrition": {
												"additionalProperties": false,
												"properties": {
													"basis": {
														"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
														"enum": [
															"per_serving",
															"per_100g"
														]
													},
													"calories": {
														"description": "Calories, zero or more.",
														"minimum": 0,
														"type": "number"
													}
												},
												"required": [
													"calories",
													"basis"
												],
												"type": [
													"object",
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
											"nutrition",
											"ingredients",
											"steps"
										],
										"type": [
											"object",
											"null"
										]
									},
									"held": {
										"type": "boolean"
									},
									"lineage_id": {
										"type": "string"
									},
									"measured": {
										"additionalProperties": false,
										"properties": {
											"ingredients": {
												"items": {
													"type": [
														"string",
														"null"
													]
												},
												"type": "array"
											},
											"steps": {
												"items": {
													"type": [
														"string",
														"null"
													]
												},
												"type": "array"
											}
										},
										"required": [
											"ingredients",
											"steps"
										],
										"type": [
											"object",
											"null"
										]
									},
									"path": {
										"items": {
											"type": "integer"
										},
										"type": "array"
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
												"lineage_id": {
													"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
												"target",
												"lineage_id"
											],
											"type": [
												"object",
												"null"
											]
										},
										"type": [
											"array",
											"null"
										]
									},
									"said": {
										"type": "string"
									},
									"share": {
										"type": [
											"number",
											"null"
										]
									},
									"stopped": {
										"type": "boolean"
									},
									"title": {
										"type": [
											"string",
											"null"
										]
									}
								},
								"required": [
									"path",
									"lineage_id",
									"held",
									"stopped",
									"branch_id",
									"title",
									"share",
									"said",
									"content",
									"readings",
									"measured"
								],
								"type": "object"
							},
							"type": "array"
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
								"nutrition": {
									"additionalProperties": false,
									"properties": {
										"basis": {
											"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
											"enum": [
												"per_serving",
												"per_100g"
											]
										},
										"calories": {
											"description": "Calories, zero or more.",
											"minimum": 0,
											"type": "number"
										}
									},
									"required": [
										"calories",
										"basis"
									],
									"type": [
										"object",
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
								"nutrition",
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"cookbook": {
							"additionalProperties": false,
							"properties": {
								"authors": {
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
								"id": {
									"type": "string"
								},
								"name": {
									"type": [
										"string",
										"null"
									]
								}
							},
							"required": [
								"id",
								"name",
								"authors"
							],
							"type": "object"
						},
						"hand_id": {
							"type": "string"
						},
						"hand_name": {
							"type": [
								"string",
								"null"
							]
						},
						"head_version_id": {
							"type": "string"
						},
						"language": {
							"type": "string"
						},
						"measured": {
							"additionalProperties": false,
							"properties": {
								"ingredients": {
									"items": {
										"type": [
											"string",
											"null"
										]
									},
									"type": "array"
								},
								"steps": {
									"items": {
										"type": [
											"string",
											"null"
										]
									},
									"type": "array"
								}
							},
							"required": [
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"mine": {
							"type": "boolean"
						},
						"name": {
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
									"lineage_id": {
										"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
									"target",
									"lineage_id"
								],
								"type": [
									"object",
									"null"
								]
							},
							"type": "array"
						}
					},
					"required": [
						"branch_id",
						"cookbook",
						"name",
						"mine",
						"arrived",
						"hand_id",
						"hand_name",
						"language",
						"head_version_id",
						"content",
						"readings",
						"measured",
						"components"
					],
					"type": "object"
				}
			},
			"required": [
				"lineage_id",
				"branch_point_version_id",
				"mine",
				"theirs",
				"ingredients",
				"steps",
				"fields"
			],
			"type": "object"
		}
	},
	{
		"name": "set_reading",
		"summary": "Correct the Reading on one Ingredient Line of a Recipe's current state — an amount, a Unit and a target, sent together as the whole new Reading (never a per-field patch, the same convention save_recipe_version uses for the whole recipe). Mints no Version and appears in no history (ADR 0021). All of them left out together clears the Reading, taking the line back to fully unread. The target is either a Food's written word or — as `lineage_id` — the Recipe this line names, which makes the Ingredient a Component (ADR 0008); never both, and a Lineage this instance does not hold is accepted, because a Component goes on naming its recipe when the recipe is gone.",
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
				"lineage_id": {
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
				"measured": {
					"type": [
						"string",
						"null"
					]
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
						"lineage_id": {
							"description": "The Recipe this line names, which makes the Ingredient a Component. It may name a Lineage this instance does not hold — deleted, never received, or held by nobody here — and the line still reads correctly, because the written line was always the truth.",
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
						"target",
						"lineage_id"
					],
					"type": [
						"object",
						"null"
					]
				}
			},
			"required": [
				"line_index",
				"reading",
				"measured"
			],
			"type": "object"
		}
	},
	{
		"name": "read_ingredient_lines",
		"summary": "Read every Ingredient Line in the library that nothing has read yet, as a Job, laying a Reading over each one Kamosu can make sense of. Touches no written line and makes no Version. A line already carrying a Reading is left alone, so a correction is never overwritten, and a line Kamosu cannot read is left unread, which is an ordinary state for a line rather than a failure. Kamosu also reads the lines of every recipe as it is written or imported, so this is for a library that predates it.",
		"permission": "operator",
		"kind": "job",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"read": {
					"type": "integer"
				}
			},
			"required": [
				"read"
			],
			"type": "object"
		}
	},
	{
		"name": "start_attempt",
		"summary": "Start cooking a Recipe: creates the Attempt, or hands back the one already In Progress for this Lineage — the cooking screen is that Attempt, never a second thing beside it. Pinned by fingerprint to the Branch's head Version at this moment, or to version_id — an older Version read back from the Thread — when one is given. Anyone who can see the recipe may.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"attempt_id": {
					"description": "The id to give this cooking, for one started with no network (#77): at_ and sixteen lower-case hex digits. Sending the same start twice answers the same Attempt; where the Lineage already has one In Progress, that one is answered instead.",
					"type": "string"
				},
				"branch_id": {
					"type": "string"
				},
				"started_at": {
					"description": "When cooking really started, for a start that waited on a phone with no network (#77). The same form as written_at.",
					"type": "string"
				},
				"version_id": {
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
				"as_cooked": {
					"additionalProperties": false,
					"properties": {
						"against": {
							"additionalProperties": false,
							"properties": {
								"ingredients": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								},
								"steps": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								}
							},
							"required": [
								"ingredients",
								"steps"
							],
							"type": "object"
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
								"nutrition": {
									"additionalProperties": false,
									"properties": {
										"basis": {
											"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
											"enum": [
												"per_serving",
												"per_100g"
											]
										},
										"calories": {
											"description": "Calories, zero or more.",
											"minimum": 0,
											"type": "number"
										}
									},
									"required": [
										"calories",
										"basis"
									],
									"type": [
										"object",
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
								"nutrition",
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"promotion_declined": {
							"type": "boolean"
						},
						"version_id": {
							"type": "string"
						}
					},
					"required": [
						"version_id",
						"content",
						"against",
						"promotion_declined"
					],
					"type": [
						"object",
						"null"
					]
				},
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"photographs": {
					"items": {
						"type": "string"
					},
					"type": "array"
				},
				"rating": {
					"enum": [
						"again",
						"tweak",
						"no",
						null
					],
					"type": [
						"string",
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
				"last_action_at",
				"photographs",
				"as_cooked"
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
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				},
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
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
				"as_cooked": {
					"additionalProperties": false,
					"properties": {
						"against": {
							"additionalProperties": false,
							"properties": {
								"ingredients": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								},
								"steps": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								}
							},
							"required": [
								"ingredients",
								"steps"
							],
							"type": "object"
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
								"nutrition": {
									"additionalProperties": false,
									"properties": {
										"basis": {
											"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
											"enum": [
												"per_serving",
												"per_100g"
											]
										},
										"calories": {
											"description": "Calories, zero or more.",
											"minimum": 0,
											"type": "number"
										}
									},
									"required": [
										"calories",
										"basis"
									],
									"type": [
										"object",
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
								"nutrition",
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"promotion_declined": {
							"type": "boolean"
						},
						"version_id": {
							"type": "string"
						}
					},
					"required": [
						"version_id",
						"content",
						"against",
						"promotion_declined"
					],
					"type": [
						"object",
						"null"
					]
				},
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"photographs": {
					"items": {
						"type": "string"
					},
					"type": "array"
				},
				"rating": {
					"enum": [
						"again",
						"tweak",
						"no",
						null
					],
					"type": [
						"string",
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
				"last_action_at",
				"photographs",
				"as_cooked"
			],
			"type": "object"
		}
	},
	{
		"name": "finish_attempt",
		"summary": "End an In Progress Attempt, taking the judgement that lands with it: a rating, a note and Photographs, all optional. Ending is not what makes the cooking real — starting already did — only what stops it being In Progress, so a cook who says nothing still cooked.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"add_photographs": {
					"description": "Photographs to put beside those the cooking already holds, rather than replacing them: what a device sends when it takes a picture, so two devices never erase each other's (#77).",
					"items": {
						"type": "string"
					},
					"type": [
						"array",
						"null"
					]
				},
				"attempt_id": {
					"type": "string"
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"photographs": {
					"items": {
						"type": "string"
					},
					"type": [
						"array",
						"null"
					]
				},
				"rating": {
					"enum": [
						"again",
						"tweak",
						"no",
						null
					],
					"type": [
						"string",
						"null"
					]
				},
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
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
				"as_cooked": {
					"additionalProperties": false,
					"properties": {
						"against": {
							"additionalProperties": false,
							"properties": {
								"ingredients": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								},
								"steps": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								}
							},
							"required": [
								"ingredients",
								"steps"
							],
							"type": "object"
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
								"nutrition": {
									"additionalProperties": false,
									"properties": {
										"basis": {
											"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
											"enum": [
												"per_serving",
												"per_100g"
											]
										},
										"calories": {
											"description": "Calories, zero or more.",
											"minimum": 0,
											"type": "number"
										}
									},
									"required": [
										"calories",
										"basis"
									],
									"type": [
										"object",
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
								"nutrition",
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"promotion_declined": {
							"type": "boolean"
						},
						"version_id": {
							"type": "string"
						}
					},
					"required": [
						"version_id",
						"content",
						"against",
						"promotion_declined"
					],
					"type": [
						"object",
						"null"
					]
				},
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"photographs": {
					"items": {
						"type": "string"
					},
					"type": "array"
				},
				"rating": {
					"enum": [
						"again",
						"tweak",
						"no",
						null
					],
					"type": [
						"string",
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
				"last_action_at",
				"photographs",
				"as_cooked"
			],
			"type": "object"
		}
	},
	{
		"name": "edit_attempt",
		"summary": "Change an Attempt's free text, its rating or its Photographs, whether it is still In Progress or long finished — an Attempt is freely editable by its cook, unlike the recipe it was cooked from.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"add_photographs": {
					"description": "Photographs to put beside those the cooking already holds, rather than replacing them: what a device sends when it takes a picture, so two devices never erase each other's (#77).",
					"items": {
						"type": "string"
					},
					"type": [
						"array",
						"null"
					]
				},
				"attempt_id": {
					"type": "string"
				},
				"note": {
					"type": [
						"string",
						"null"
					]
				},
				"photographs": {
					"items": {
						"type": "string"
					},
					"type": [
						"array",
						"null"
					]
				},
				"rating": {
					"enum": [
						"again",
						"tweak",
						"no",
						null
					],
					"type": [
						"string",
						"null"
					]
				},
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
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
				"as_cooked": {
					"additionalProperties": false,
					"properties": {
						"against": {
							"additionalProperties": false,
							"properties": {
								"ingredients": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								},
								"steps": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								}
							},
							"required": [
								"ingredients",
								"steps"
							],
							"type": "object"
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
								"nutrition": {
									"additionalProperties": false,
									"properties": {
										"basis": {
											"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
											"enum": [
												"per_serving",
												"per_100g"
											]
										},
										"calories": {
											"description": "Calories, zero or more.",
											"minimum": 0,
											"type": "number"
										}
									},
									"required": [
										"calories",
										"basis"
									],
									"type": [
										"object",
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
								"nutrition",
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"promotion_declined": {
							"type": "boolean"
						},
						"version_id": {
							"type": "string"
						}
					},
					"required": [
						"version_id",
						"content",
						"against",
						"promotion_declined"
					],
					"type": [
						"object",
						"null"
					]
				},
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"photographs": {
					"items": {
						"type": "string"
					},
					"type": "array"
				},
				"rating": {
					"enum": [
						"again",
						"tweak",
						"no",
						null
					],
					"type": [
						"string",
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
				"last_action_at",
				"photographs",
				"as_cooked"
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
		"name": "promote_attempt_photograph",
		"summary": "Make a picture taken while cooking the recipe's Main Photo, or a Step's photo — so the picture you actually took becomes the recipe's picture. This is an ordinary edit making a Version, with everything that follows from it: a rapid re-save folding into the Version already being shaped, and a Copy in your own Cookbook where you do not write the Branch's. The Branch must be one you may see. The Attempt keeps the picture too; promoting is not moving.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"attempt_id": {
					"type": "string"
				},
				"branch_id": {
					"description": "Which Branch of the cooked Lineage to promote into. An Attempt belongs to a Lineage rather than a Branch, so this says where the picture lands.",
					"type": "string"
				},
				"change_note": {
					"type": [
						"string",
						"null"
					]
				},
				"kitchen_id": {
					"description": "Ignored. Everything you write lands in your own Cookbook (ADR 0041), so there is no Kitchen to name. Accepted so that a client which still sends it is not refused.",
					"type": "string"
				},
				"photograph_id": {
					"description": "One of this Attempt's own Photographs. Any other Photograph is refused: this is not a second way to set the Main Photo.",
					"type": "string"
				},
				"step_index": {
					"description": "The Step whose photo this becomes. Left out or null, the picture becomes the Main Photo.",
					"minimum": 0,
					"type": [
						"integer",
						"null"
					]
				}
			},
			"required": [
				"attempt_id",
				"photograph_id",
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
				"collapsed": {
					"type": "boolean"
				},
				"copied": {
					"description": "True when this save was a Copy: branch_id names the new Branch it started, never the one asked for.",
					"type": "boolean"
				},
				"language": {
					"description": "The Language this recipe still carries. A save never changes it.",
					"type": "string"
				},
				"language_offer": {
					"description": "The Language this text reads as, when that disagrees with the one the recipe carries — an offer to put to the cook, never a change. Null when they agree, when there is too little text to tell, and always when the Language is unknown.",
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
				"sequence": {
					"type": "integer"
				},
				"translates_version_id": {
					"description": "The Version of the source this Version renders, for a Translation. Carried forward from the Version replaced unless this save named a new one; null on a recipe that translates nothing.",
					"type": [
						"string",
						"null"
					]
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
				"copied",
				"language",
				"language_offer",
				"translates_version_id"
			],
			"type": "object"
		}
	},
	{
		"name": "set_as_cooked",
		"summary": "Write down what you actually cooked, where it differed from the recipe: the whole recipe as you cooked it, in ordinary Ingredient Lines and ordinary Step text — a line reworded, one added, one dropped, a step grown. Not a record of differences; the same shape a Version takes. Sending back exactly what the recipe says, or null, stores nothing at all, because cooking a recipe as it is written changes nothing. Changes no recipe and makes no Version: that is Promotion, and it is a separate act.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"as_cooked": {
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
						"nutrition": {
							"additionalProperties": false,
							"properties": {
								"basis": {
									"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
									"enum": [
										"per_serving",
										"per_100g"
									]
								},
								"calories": {
									"description": "Calories, zero or more.",
									"minimum": 0,
									"type": "number"
								}
							},
							"required": [
								"calories",
								"basis"
							],
							"type": [
								"object",
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
									"text"
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
						"title"
					],
					"type": [
						"object",
						"null"
					]
				},
				"attempt_id": {
					"type": "string"
				},
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
					"type": "string"
				}
			},
			"required": [
				"attempt_id",
				"as_cooked"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"as_cooked": {
					"additionalProperties": false,
					"properties": {
						"against": {
							"additionalProperties": false,
							"properties": {
								"ingredients": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								},
								"steps": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								}
							},
							"required": [
								"ingredients",
								"steps"
							],
							"type": "object"
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
								"nutrition": {
									"additionalProperties": false,
									"properties": {
										"basis": {
											"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
											"enum": [
												"per_serving",
												"per_100g"
											]
										},
										"calories": {
											"description": "Calories, zero or more.",
											"minimum": 0,
											"type": "number"
										}
									},
									"required": [
										"calories",
										"basis"
									],
									"type": [
										"object",
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
								"nutrition",
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"promotion_declined": {
							"type": "boolean"
						},
						"version_id": {
							"type": "string"
						}
					},
					"required": [
						"version_id",
						"content",
						"against",
						"promotion_declined"
					],
					"type": [
						"object",
						"null"
					]
				},
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"photographs": {
					"items": {
						"type": "string"
					},
					"type": "array"
				},
				"rating": {
					"enum": [
						"again",
						"tweak",
						"no",
						null
					],
					"type": [
						"string",
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
				"last_action_at",
				"photographs",
				"as_cooked"
			],
			"type": "object"
		}
	},
	{
		"name": "decline_promotion",
		"summary": "Say that the words a cooking used belong in the diary and not in the recipe — or take that back. It answers the offer and nothing else: what was cooked stays on the cooking, whole. Remembered, because a question already answered, asked twice, is a nag.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"attempt_id": {
					"type": "string"
				},
				"declined": {
					"type": "boolean"
				}
			},
			"required": [
				"attempt_id",
				"declined"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"as_cooked": {
					"additionalProperties": false,
					"properties": {
						"against": {
							"additionalProperties": false,
							"properties": {
								"ingredients": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								},
								"steps": {
									"items": {
										"additionalProperties": false,
										"properties": {
											"from_branch_point": {
												"type": "boolean"
											},
											"kind": {
												"type": "string"
											},
											"mine": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											},
											"state": {
												"enum": [
													"same",
													"changed",
													"only-mine",
													"only-theirs"
												]
											},
											"theirs": {
												"additionalProperties": false,
												"properties": {
													"index": {
														"type": "integer"
													},
													"kind": {
														"type": "string"
													},
													"text": {
														"type": "string"
													}
												},
												"required": [
													"kind",
													"text",
													"index"
												],
												"type": [
													"object",
													"null"
												]
											}
										},
										"required": [
											"kind",
											"state",
											"from_branch_point",
											"mine",
											"theirs"
										],
										"type": "object"
									},
									"type": "array"
								}
							},
							"required": [
								"ingredients",
								"steps"
							],
							"type": "object"
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
								"nutrition": {
									"additionalProperties": false,
									"properties": {
										"basis": {
											"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
											"enum": [
												"per_serving",
												"per_100g"
											]
										},
										"calories": {
											"description": "Calories, zero or more.",
											"minimum": 0,
											"type": "number"
										}
									},
									"required": [
										"calories",
										"basis"
									],
									"type": [
										"object",
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
								"nutrition",
								"ingredients",
								"steps"
							],
							"type": "object"
						},
						"promotion_declined": {
							"type": "boolean"
						},
						"version_id": {
							"type": "string"
						}
					},
					"required": [
						"version_id",
						"content",
						"against",
						"promotion_declined"
					],
					"type": [
						"object",
						"null"
					]
				},
				"cooking_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"photographs": {
					"items": {
						"type": "string"
					},
					"type": "array"
				},
				"rating": {
					"enum": [
						"again",
						"tweak",
						"no",
						null
					],
					"type": [
						"string",
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
				"last_action_at",
				"photographs",
				"as_cooked"
			],
			"type": "object"
		}
	},
	{
		"name": "promote_as_cooked",
		"summary": "Promotion: turn what you cooked into a real Version of the recipe. Mechanical — the As Cooked is already a whole recipe, so nothing is retyped and nothing is reconciled. It is an ordinary edit and inherits all of one: a rapid re-save folds into the Version being shaped, and a Branch whose Cookbook you do not write becomes a Copy in your own. The Branch must be one you may see. Promoting a cooking of an older Version appends onto wherever the Branch stands now — a Version, never a merge. The Attempt is left exactly as it was, still saying which Version it cooked.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"attempt_id": {
					"type": "string"
				},
				"branch_id": {
					"description": "Which Branch of the cooked Lineage to promote into. An Attempt belongs to a Lineage rather than a Branch, so this says where the words land.",
					"type": "string"
				},
				"change_note": {
					"type": [
						"string",
						"null"
					]
				},
				"kitchen_id": {
					"description": "Ignored. Everything you write lands in your own Cookbook (ADR 0041), so there is no Kitchen to name. Accepted so that a client which still sends it is not refused.",
					"type": "string"
				},
				"name": {
					"type": [
						"string",
						"null"
					]
				}
			},
			"required": [
				"attempt_id",
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
				"collapsed": {
					"type": "boolean"
				},
				"copied": {
					"description": "True when this save was a Copy: branch_id names the new Branch it started, never the one asked for.",
					"type": "boolean"
				},
				"language": {
					"description": "The Language this recipe still carries. A save never changes it.",
					"type": "string"
				},
				"language_offer": {
					"description": "The Language this text reads as, when that disagrees with the one the recipe carries — an offer to put to the cook, never a change. Null when they agree, when there is too little text to tell, and always when the Language is unknown.",
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
				"sequence": {
					"type": "integer"
				},
				"translates_version_id": {
					"description": "The Version of the source this Version renders, for a Translation. Carried forward from the Version replaced unless this save named a new one; null on a recipe that translates nothing.",
					"type": [
						"string",
						"null"
					]
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
				"copied",
				"language",
				"language_offer",
				"translates_version_id"
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
						"as_cooked": {
							"additionalProperties": false,
							"properties": {
								"against": {
									"additionalProperties": false,
									"properties": {
										"ingredients": {
											"items": {
												"additionalProperties": false,
												"properties": {
													"from_branch_point": {
														"type": "boolean"
													},
													"kind": {
														"type": "string"
													},
													"mine": {
														"additionalProperties": false,
														"properties": {
															"index": {
																"type": "integer"
															},
															"kind": {
																"type": "string"
															},
															"text": {
																"type": "string"
															}
														},
														"required": [
															"kind",
															"text",
															"index"
														],
														"type": [
															"object",
															"null"
														]
													},
													"state": {
														"enum": [
															"same",
															"changed",
															"only-mine",
															"only-theirs"
														]
													},
													"theirs": {
														"additionalProperties": false,
														"properties": {
															"index": {
																"type": "integer"
															},
															"kind": {
																"type": "string"
															},
															"text": {
																"type": "string"
															}
														},
														"required": [
															"kind",
															"text",
															"index"
														],
														"type": [
															"object",
															"null"
														]
													}
												},
												"required": [
													"kind",
													"state",
													"from_branch_point",
													"mine",
													"theirs"
												],
												"type": "object"
											},
											"type": "array"
										},
										"steps": {
											"items": {
												"additionalProperties": false,
												"properties": {
													"from_branch_point": {
														"type": "boolean"
													},
													"kind": {
														"type": "string"
													},
													"mine": {
														"additionalProperties": false,
														"properties": {
															"index": {
																"type": "integer"
															},
															"kind": {
																"type": "string"
															},
															"text": {
																"type": "string"
															}
														},
														"required": [
															"kind",
															"text",
															"index"
														],
														"type": [
															"object",
															"null"
														]
													},
													"state": {
														"enum": [
															"same",
															"changed",
															"only-mine",
															"only-theirs"
														]
													},
													"theirs": {
														"additionalProperties": false,
														"properties": {
															"index": {
																"type": "integer"
															},
															"kind": {
																"type": "string"
															},
															"text": {
																"type": "string"
															}
														},
														"required": [
															"kind",
															"text",
															"index"
														],
														"type": [
															"object",
															"null"
														]
													}
												},
												"required": [
													"kind",
													"state",
													"from_branch_point",
													"mine",
													"theirs"
												],
												"type": "object"
											},
											"type": "array"
										}
									},
									"required": [
										"ingredients",
										"steps"
									],
									"type": "object"
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
										"nutrition": {
											"additionalProperties": false,
											"properties": {
												"basis": {
													"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
													"enum": [
														"per_serving",
														"per_100g"
													]
												},
												"calories": {
													"description": "Calories, zero or more.",
													"minimum": 0,
													"type": "number"
												}
											},
											"required": [
												"calories",
												"basis"
											],
											"type": [
												"object",
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
										"nutrition",
										"ingredients",
										"steps"
									],
									"type": "object"
								},
								"promotion_declined": {
									"type": "boolean"
								},
								"version_id": {
									"type": "string"
								}
							},
							"required": [
								"version_id",
								"content",
								"against",
								"promotion_declined"
							],
							"type": [
								"object",
								"null"
							]
						},
						"cooking_yield": {
							"additionalProperties": false,
							"properties": {
								"amount": {
									"type": "string"
								},
								"noun": {
									"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
						"photographs": {
							"items": {
								"type": "string"
							},
							"type": "array"
						},
						"rating": {
							"enum": [
								"again",
								"tweak",
								"no",
								null
							],
							"type": [
								"string",
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
						"last_action_at",
						"photographs",
						"as_cooked"
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
		"name": "list_attempts",
		"summary": "The cooking diary: every Attempt the caller has made, newest first, across every recipe — sorted by date rather than by recipe, which is what makes *what did I cook that week* answerable. Unfinished and In Progress cookings are in it too, because starting is what makes a cooking real. Each entry names the recipe it was cooked from, and still names it after that recipe has left the caller's shelf.",
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
				"attempts": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"as_cooked": {
								"additionalProperties": false,
								"properties": {
									"against": {
										"additionalProperties": false,
										"properties": {
											"ingredients": {
												"items": {
													"additionalProperties": false,
													"properties": {
														"from_branch_point": {
															"type": "boolean"
														},
														"kind": {
															"type": "string"
														},
														"mine": {
															"additionalProperties": false,
															"properties": {
																"index": {
																	"type": "integer"
																},
																"kind": {
																	"type": "string"
																},
																"text": {
																	"type": "string"
																}
															},
															"required": [
																"kind",
																"text",
																"index"
															],
															"type": [
																"object",
																"null"
															]
														},
														"state": {
															"enum": [
																"same",
																"changed",
																"only-mine",
																"only-theirs"
															]
														},
														"theirs": {
															"additionalProperties": false,
															"properties": {
																"index": {
																	"type": "integer"
																},
																"kind": {
																	"type": "string"
																},
																"text": {
																	"type": "string"
																}
															},
															"required": [
																"kind",
																"text",
																"index"
															],
															"type": [
																"object",
																"null"
															]
														}
													},
													"required": [
														"kind",
														"state",
														"from_branch_point",
														"mine",
														"theirs"
													],
													"type": "object"
												},
												"type": "array"
											},
											"steps": {
												"items": {
													"additionalProperties": false,
													"properties": {
														"from_branch_point": {
															"type": "boolean"
														},
														"kind": {
															"type": "string"
														},
														"mine": {
															"additionalProperties": false,
															"properties": {
																"index": {
																	"type": "integer"
																},
																"kind": {
																	"type": "string"
																},
																"text": {
																	"type": "string"
																}
															},
															"required": [
																"kind",
																"text",
																"index"
															],
															"type": [
																"object",
																"null"
															]
														},
														"state": {
															"enum": [
																"same",
																"changed",
																"only-mine",
																"only-theirs"
															]
														},
														"theirs": {
															"additionalProperties": false,
															"properties": {
																"index": {
																	"type": "integer"
																},
																"kind": {
																	"type": "string"
																},
																"text": {
																	"type": "string"
																}
															},
															"required": [
																"kind",
																"text",
																"index"
															],
															"type": [
																"object",
																"null"
															]
														}
													},
													"required": [
														"kind",
														"state",
														"from_branch_point",
														"mine",
														"theirs"
													],
													"type": "object"
												},
												"type": "array"
											}
										},
										"required": [
											"ingredients",
											"steps"
										],
										"type": "object"
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
											"nutrition": {
												"additionalProperties": false,
												"properties": {
													"basis": {
														"description": "What the figure counts: one serving of the Yield as written, or 100 g.",
														"enum": [
															"per_serving",
															"per_100g"
														]
													},
													"calories": {
														"description": "Calories, zero or more.",
														"minimum": 0,
														"type": "number"
													}
												},
												"required": [
													"calories",
													"basis"
												],
												"type": [
													"object",
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
											"nutrition",
											"ingredients",
											"steps"
										],
										"type": "object"
									},
									"promotion_declined": {
										"type": "boolean"
									},
									"version_id": {
										"type": "string"
									}
								},
								"required": [
									"version_id",
									"content",
									"against",
									"promotion_declined"
								],
								"type": [
									"object",
									"null"
								]
							},
							"cooking_yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"photographs": {
								"items": {
									"type": "string"
								},
								"type": "array"
							},
							"rating": {
								"enum": [
									"again",
									"tweak",
									"no",
									null
								],
								"type": [
									"string",
									"null"
								]
							},
							"recipe": {
								"additionalProperties": false,
								"properties": {
									"branch_id": {
										"type": [
											"string",
											"null"
										]
									},
									"title": {
										"type": "string"
									},
									"written_yield": {
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
									"title",
									"written_yield"
								],
								"type": "object"
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
							"last_action_at",
							"photographs",
							"as_cooked",
							"recipe"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"attempts"
			],
			"type": "object"
		}
	},
	{
		"name": "get_shopping_list",
		"summary": "Your Shopping List: the recipes you chose, and the rows worked out from them. Everyone has exactly one; it has no name and is never archived. The rows are computed on every read and stored nowhere, so editing a chosen recipe or correcting a Reading changes the list at once. A row names a Food in your Reading Language and merges every mention of it; amounts add where the Units honestly convert, saying about, and ride side by side where they do not. Nothing here is ticked off.",
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
				"chosen": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"gone": {
								"type": "boolean"
							},
							"shopping_yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"title": {
								"type": "string"
							},
							"written_yield": {
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
							"title",
							"gone",
							"shopping_yield",
							"written_yield"
						],
						"type": "object"
					},
					"type": "array"
				},
				"rows": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kind": {
								"enum": [
									"food",
									"line",
									"loose"
								]
							},
							"lines": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"recipe": {
											"type": "string"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"branch_id",
										"recipe",
										"text"
									],
									"type": "object"
								},
								"type": "array"
							},
							"name": {
								"type": "string"
							},
							"name_language": {
								"type": [
									"string",
									"null"
								]
							},
							"parts": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"kind": {
											"enum": [
												"about",
												"count",
												"as_written",
												"no_amount"
											]
										},
										"sources": {
											"items": {
												"type": "string"
											},
											"type": "array"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"kind",
										"text",
										"sources"
									],
									"type": "object"
								},
								"type": "array"
							},
							"said": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"kind",
							"name",
							"name_language",
							"parts",
							"lines",
							"said"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"chosen",
				"rows"
			],
			"type": "object"
		}
	},
	{
		"name": "shopping_basis",
		"summary": "What one recipe puts on a Shopping List before anything is added up: each Ingredient Line, the Food it was read as and the name that Food goes by for you, how much it said, its Unit, and what a cup of the Food weighs. Every recipe this one includes is unfolded to the bottom and its lines are here too, already carrying their share, so a pizza's flour and its dough's flour add up to one thing to buy. Always the Branch's latest Version. It is how a phone with no network works out the list's rows itself for the recipes it holds (#77); get_shopping_list is the list itself.",
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
				"lines": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"food": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": [
											"number",
											"null"
										]
									},
									"cup_weight_grams": {
										"type": [
											"number",
											"null"
										]
									},
									"id": {
										"type": "string"
									},
									"name": {
										"type": [
											"string",
											"null"
										]
									},
									"name_language": {
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
									},
									"unit_id": {
										"type": [
											"string",
											"null"
										]
									},
									"unit_key": {
										"type": [
											"string",
											"null"
										]
									}
								},
								"required": [
									"id",
									"name",
									"name_language",
									"amount",
									"unit",
									"unit_id",
									"unit_key",
									"cup_weight_grams"
								],
								"type": [
									"object",
									"null"
								]
							},
							"from": {
								"additionalProperties": false,
								"properties": {
									"branch_id": {
										"type": "string"
									},
									"title": {
										"type": "string"
									}
								},
								"required": [
									"branch_id",
									"title"
								],
								"type": [
									"object",
									"null"
								]
							},
							"path": {
								"items": {
									"minimum": 0,
									"type": "integer"
								},
								"type": "array"
							},
							"said": {
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
							"path",
							"from",
							"said",
							"text",
							"food"
						],
						"type": "object"
					},
					"type": "array"
				},
				"title": {
					"type": "string"
				},
				"written_yield": {
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
				"title",
				"written_yield",
				"lines"
			],
			"type": "object"
		}
	},
	{
		"name": "add_to_shopping_list",
		"summary": "Choose a recipe to shop for, at a Yield, a multiplier (a Yield with an empty noun) or as it is written. It holds the Branch at its latest Version, never a Lineage and never pinned, so a recipe edited between the planning and the shopping is right in the shop. Choosing one already on the list is not an error and makes no second entry: it moves that entry to the Yield given here, or back to the recipe as written when none is. Answers the whole list.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"shopping_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
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
				"chosen": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"gone": {
								"type": "boolean"
							},
							"shopping_yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"title": {
								"type": "string"
							},
							"written_yield": {
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
							"title",
							"gone",
							"shopping_yield",
							"written_yield"
						],
						"type": "object"
					},
					"type": "array"
				},
				"rows": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kind": {
								"enum": [
									"food",
									"line",
									"loose"
								]
							},
							"lines": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"recipe": {
											"type": "string"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"branch_id",
										"recipe",
										"text"
									],
									"type": "object"
								},
								"type": "array"
							},
							"name": {
								"type": "string"
							},
							"name_language": {
								"type": [
									"string",
									"null"
								]
							},
							"parts": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"kind": {
											"enum": [
												"about",
												"count",
												"as_written",
												"no_amount"
											]
										},
										"sources": {
											"items": {
												"type": "string"
											},
											"type": "array"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"kind",
										"text",
										"sources"
									],
									"type": "object"
								},
								"type": "array"
							},
							"said": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"kind",
							"name",
							"name_language",
							"parts",
							"lines",
							"said"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"chosen",
				"rows"
			],
			"type": "object"
		}
	},
	{
		"name": "remove_from_shopping_list",
		"summary": "Take a recipe off your Shopping List. Works whether or not it can still be read, which is exactly the entry somebody most wants gone. Answers the whole list.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
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
				"chosen": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"gone": {
								"type": "boolean"
							},
							"shopping_yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"title": {
								"type": "string"
							},
							"written_yield": {
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
							"title",
							"gone",
							"shopping_yield",
							"written_yield"
						],
						"type": "object"
					},
					"type": "array"
				},
				"rows": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kind": {
								"enum": [
									"food",
									"line",
									"loose"
								]
							},
							"lines": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"recipe": {
											"type": "string"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"branch_id",
										"recipe",
										"text"
									],
									"type": "object"
								},
								"type": "array"
							},
							"name": {
								"type": "string"
							},
							"name_language": {
								"type": [
									"string",
									"null"
								]
							},
							"parts": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"kind": {
											"enum": [
												"about",
												"count",
												"as_written",
												"no_amount"
											]
										},
										"sources": {
											"items": {
												"type": "string"
											},
											"type": "array"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"kind",
										"text",
										"sources"
									],
									"type": "object"
								},
								"type": "array"
							},
							"said": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"kind",
							"name",
							"name_language",
							"parts",
							"lines",
							"said"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"chosen",
				"rows"
			],
			"type": "object"
		}
	},
	{
		"name": "set_shopping_yield",
		"summary": "Say how much of a chosen recipe you are shopping for — an amount and its noun, a multiplier (an amount with an empty noun: twice the recipe is `2`), or null for the recipe as written. Every amount it contributes moves with it. Answers the whole list.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"branch_id": {
					"type": "string"
				},
				"shopping_yield": {
					"additionalProperties": false,
					"properties": {
						"amount": {
							"type": "string"
						},
						"noun": {
							"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
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
				"chosen": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"gone": {
								"type": "boolean"
							},
							"shopping_yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"title": {
								"type": "string"
							},
							"written_yield": {
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
							"title",
							"gone",
							"shopping_yield",
							"written_yield"
						],
						"type": "object"
					},
					"type": "array"
				},
				"rows": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kind": {
								"enum": [
									"food",
									"line",
									"loose"
								]
							},
							"lines": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"recipe": {
											"type": "string"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"branch_id",
										"recipe",
										"text"
									],
									"type": "object"
								},
								"type": "array"
							},
							"name": {
								"type": "string"
							},
							"name_language": {
								"type": [
									"string",
									"null"
								]
							},
							"parts": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"kind": {
											"enum": [
												"about",
												"count",
												"as_written",
												"no_amount"
											]
										},
										"sources": {
											"items": {
												"type": "string"
											},
											"type": "array"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"kind",
										"text",
										"sources"
									],
									"type": "object"
								},
								"type": "array"
							},
							"said": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"kind",
							"name",
							"name_language",
							"parts",
							"lines",
							"said"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"chosen",
				"rows"
			],
			"type": "object"
		}
	},
	{
		"name": "shopping_list_as_text",
		"summary": "Your Shopping List as plain text, ready to be carried out of Kamosu. Nothing is ticked off here, because the list leaves and something else holds the ticks — Apple Notes, through a Shortcut. The text opens with a header line, the date and the recipes it was built from (and any that can no longer be read), because a note accumulates and three trips appended with no divider are a wall. Under it, one flat alphabetical list with one Markdown checklist line (`- [ ] `) per thing to buy, so each line becomes one checkbox; a row whose amounts could not be added stays on its one line, naming the dish behind each amount. This only reads: emptying the list afterwards is a separate Operation, offered and never done on the way out.",
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
				"text": {
					"type": "string"
				}
			},
			"required": [
				"text"
			],
			"type": "object"
		}
	},
	{
		"name": "empty_shopping_list",
		"summary": "Empty your Shopping List — every recipe chosen and every typed line at once. Offered after the list has left as text and never done on the way out: a list that emptied itself when it was sent would be silent and unrecoverable. Answers the whole list.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
					"type": "string"
				}
			},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"chosen": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"gone": {
								"type": "boolean"
							},
							"shopping_yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"title": {
								"type": "string"
							},
							"written_yield": {
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
							"title",
							"gone",
							"shopping_yield",
							"written_yield"
						],
						"type": "object"
					},
					"type": "array"
				},
				"rows": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kind": {
								"enum": [
									"food",
									"line",
									"loose"
								]
							},
							"lines": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"recipe": {
											"type": "string"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"branch_id",
										"recipe",
										"text"
									],
									"type": "object"
								},
								"type": "array"
							},
							"name": {
								"type": "string"
							},
							"name_language": {
								"type": [
									"string",
									"null"
								]
							},
							"parts": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"kind": {
											"enum": [
												"about",
												"count",
												"as_written",
												"no_amount"
											]
										},
										"sources": {
											"items": {
												"type": "string"
											},
											"type": "array"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"kind",
										"text",
										"sources"
									],
									"type": "object"
								},
								"type": "array"
							},
							"said": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"kind",
							"name",
							"name_language",
							"parts",
							"lines",
							"said"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"chosen",
				"rows"
			],
			"type": "object"
		}
	},
	{
		"name": "add_loose_item",
		"summary": "Type a line straight onto your Shopping List — bin bags, coffee. Kept exactly as typed and never read, so it carries no amount and merges with nothing: typing flour beside a recipe that wants flour gives two lines. Answers the whole list.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"item_id": {
					"description": "The id to give this line, for one typed with no network (#77): i_ and sixteen lower-case hex digits. Sending the same line twice adds it once.",
					"type": "string"
				},
				"text": {
					"type": "string"
				},
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
					"type": "string"
				}
			},
			"required": [
				"text"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"chosen": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"gone": {
								"type": "boolean"
							},
							"shopping_yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"title": {
								"type": "string"
							},
							"written_yield": {
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
							"title",
							"gone",
							"shopping_yield",
							"written_yield"
						],
						"type": "object"
					},
					"type": "array"
				},
				"rows": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kind": {
								"enum": [
									"food",
									"line",
									"loose"
								]
							},
							"lines": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"recipe": {
											"type": "string"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"branch_id",
										"recipe",
										"text"
									],
									"type": "object"
								},
								"type": "array"
							},
							"name": {
								"type": "string"
							},
							"name_language": {
								"type": [
									"string",
									"null"
								]
							},
							"parts": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"kind": {
											"enum": [
												"about",
												"count",
												"as_written",
												"no_amount"
											]
										},
										"sources": {
											"items": {
												"type": "string"
											},
											"type": "array"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"kind",
										"text",
										"sources"
									],
									"type": "object"
								},
								"type": "array"
							},
							"said": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"kind",
							"name",
							"name_language",
							"parts",
							"lines",
							"said"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"chosen",
				"rows"
			],
			"type": "object"
		}
	},
	{
		"name": "remove_loose_item",
		"summary": "Take one typed line off your Shopping List. Answers the whole list.",
		"permission": "person",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"item_id": {
					"type": "string"
				},
				"written_at": {
					"description": "When this was really written, for a write a phone held while it had no network and sent later (#77): an ISO 8601 time such as 2026-09-19T14:05:00.000Z. Absent means now. Where the cook has since moved on, or the Shopping List has since been written, on another device, a write older than that changes nothing and the answer is how things stand. On a cooking already finished, a move changes nothing and a finish keeps the first finish's time (its rating, note and Photographs still land), whenever either was written: a finished cooking is final.",
					"type": "string"
				}
			},
			"required": [
				"item_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"chosen": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"branch_id": {
								"type": "string"
							},
							"gone": {
								"type": "boolean"
							},
							"shopping_yield": {
								"additionalProperties": false,
								"properties": {
									"amount": {
										"type": "string"
									},
									"noun": {
										"description": "What the amount counts, as the recipe's own Yield names it. Empty makes the amount a multiplier of the recipe as written.",
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
							"title": {
								"type": "string"
							},
							"written_yield": {
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
							"title",
							"gone",
							"shopping_yield",
							"written_yield"
						],
						"type": "object"
					},
					"type": "array"
				},
				"rows": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"id": {
								"type": "string"
							},
							"kind": {
								"enum": [
									"food",
									"line",
									"loose"
								]
							},
							"lines": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"branch_id": {
											"type": "string"
										},
										"recipe": {
											"type": "string"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"branch_id",
										"recipe",
										"text"
									],
									"type": "object"
								},
								"type": "array"
							},
							"name": {
								"type": "string"
							},
							"name_language": {
								"type": [
									"string",
									"null"
								]
							},
							"parts": {
								"items": {
									"additionalProperties": false,
									"properties": {
										"kind": {
											"enum": [
												"about",
												"count",
												"as_written",
												"no_amount"
											]
										},
										"sources": {
											"items": {
												"type": "string"
											},
											"type": "array"
										},
										"text": {
											"type": "string"
										}
									},
									"required": [
										"kind",
										"text",
										"sources"
									],
									"type": "object"
								},
								"type": "array"
							},
							"said": {
								"type": [
									"string",
									"null"
								]
							}
						},
						"required": [
							"id",
							"kind",
							"name",
							"name_language",
							"parts",
							"lines",
							"said"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"chosen",
				"rows"
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
		"name": "list_merge_suggestions",
		"summary": "The Operator's worklist: every note that two Foods are probably one thing, with the words that said so. Evidence, never an instruction — nothing merges itself.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {},
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"suggestions": {
					"items": {
						"additionalProperties": false,
						"properties": {
							"created_at": {
								"type": "string"
							},
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
								"maxItems": 2,
								"minItems": 2,
								"type": "array"
							},
							"reason": {
								"enum": [
									"arrived_as_one",
									"name_typed_onto_another"
								]
							},
							"words": {
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
							"foods",
							"reason",
							"words",
							"created_at"
						],
						"type": "object"
					},
					"type": "array"
				}
			},
			"required": [
				"suggestions"
			],
			"type": "object"
		}
	},
	{
		"name": "preview_food_merge",
		"summary": "Say how many Ingredient Lines a Merge would move, and how many Reading rows, without moving any of them. A Merge cannot be undone and refuses to run until this figure is said back to it, so this saying is its safety net rather than a courtesy.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"absorbed_food_id": {
					"type": "string"
				},
				"survivor_food_id": {
					"type": "string"
				}
			},
			"required": [
				"survivor_food_id",
				"absorbed_food_id"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"absorbed": {
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
				"cup_weight_conflict": {
					"type": "boolean"
				},
				"ingredient_lines": {
					"minimum": 0,
					"type": "integer"
				},
				"readings": {
					"minimum": 0,
					"type": "integer"
				},
				"survivor": {
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
			"required": [
				"survivor",
				"absorbed",
				"ingredient_lines",
				"readings",
				"cup_weight_conflict"
			],
			"type": "object"
		}
	},
	{
		"name": "merge_food",
		"summary": "Join two Foods into one: the survivor takes every name both had, every Reading pointing at the other points at it instead, and every Merge Suggestion naming either is cleared. ingredient_lines is the figure preview_food_merge announced, said back — a Merge that does not match it is refused. Where the two disagree about Cup Weight, cup_weight_grams says which of the two figures survives. There is no un-merge in v1.",
		"permission": "operator",
		"kind": "immediate",
		"input_schema": {
			"additionalProperties": false,
			"properties": {
				"absorbed_food_id": {
					"type": "string"
				},
				"cup_weight_grams": {
					"exclusiveMinimum": 0,
					"type": [
						"number",
						"null"
					]
				},
				"ingredient_lines": {
					"minimum": 0,
					"type": "integer"
				},
				"survivor_food_id": {
					"type": "string"
				}
			},
			"required": [
				"survivor_food_id",
				"absorbed_food_id",
				"ingredient_lines"
			],
			"type": "object"
		},
		"output_schema": {
			"additionalProperties": false,
			"properties": {
				"food": {
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
				"ingredient_lines": {
					"minimum": 0,
					"type": "integer"
				},
				"readings": {
					"minimum": 0,
					"type": "integer"
				}
			},
			"required": [
				"food",
				"ingredient_lines",
				"readings"
			],
			"type": "object"
		}
	},
	{
		"name": "delete_food",
		"summary": "Delete a Food nothing points at. One a Reading still points at is refused: what a Food knows was expensive to learn and is never discarded by an unrelated act.",
		"permission": "operator",
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

/**
 * The Immediate Operations that change nothing. The service worker answers
 * these from the phone and refreshes behind, and replays nothing else (#76,
 * ADR 0013). A list of names alone, so the worker never carries the schemas.
 */
export const READS: readonly OperationName[] = [
	'instance_status',
	'get_reading_preferences',
	'get_person',
	'list_accounts',
	'list_backups',
	'list_sessions',
	'list_access_keys',
	'list_kitchens',
	'preview_leaving_kitchen',
	'get_cookbook',
	'read_cookbook_invite',
	'list_tags',
	'read_pasted_recipe',
	'list_imports',
	'search_recipes',
	'home_shelves',
	'meaning_search_status',
	'get_recipe',
	'get_thread',
	'get_share_link',
	'get_public_address',
	'export_bundle',
	'read_shared_recipe',
	'branch_point',
	'divergence',
	'get_current_attempt',
	'list_attempts',
	'get_shopping_list',
	'shopping_basis',
	'shopping_list_as_text',
	'list_foods',
	'get_food',
	'list_merge_suggestions',
	'preview_food_merge',
	'get_job',
	'list_jobs',
];

/** The camelCase method the client exposes for each Operation. */
export const METHOD_NAMES = {
	instance_status: 'instanceStatus',
	set_reading_preferences: 'setReadingPreferences',
	get_reading_preferences: 'getReadingPreferences',
	get_person: 'getPerson',
	rename_person: 'renamePerson',
	list_accounts: 'listAccounts',
	mint_invite: 'mintInvite',
	disable_account: 'disableAccount',
	delete_account: 'deleteAccount',
	mint_recovery_link: 'mintRecoveryLink',
	set_operator: 'setOperator',
	sweep_photographs: 'sweepPhotographs',
	take_backup: 'takeBackup',
	list_backups: 'listBackups',
	list_sessions: 'listSessions',
	rename_session: 'renameSession',
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
	preview_leaving_kitchen: 'previewLeavingKitchen',
	get_cookbook: 'getCookbook',
	rename_cookbook: 'renameCookbook',
	invite_to_cookbook: 'inviteToCookbook',
	cancel_cookbook_invite: 'cancelCookbookInvite',
	read_cookbook_invite: 'readCookbookInvite',
	accept_cookbook_invite: 'acceptCookbookInvite',
	leave_cookbook: 'leaveCookbook',
	remove_cookbook_author: 'removeCookbookAuthor',
	create_tag: 'createTag',
	list_tags: 'listTags',
	rename_tag: 'renameTag',
	merge_tags: 'mergeTags',
	delete_tag: 'deleteTag',
	set_recipe_tag: 'setRecipeTag',
	set_related_recipe: 'setRelatedRecipe',
	create_recipe: 'createRecipe',
	save_recipe_version: 'saveRecipeVersion',
	start_variation: 'startVariation',
	rename_branch: 'renameBranch',
	delete_recipe: 'deleteRecipe',
	read_pasted_recipe: 'readPastedRecipe',
	start_translation: 'startTranslation',
	set_recipe_language: 'setRecipeLanguage',
	import: 'import',
	import_crouton: 'importCrouton',
	list_imports: 'listImports',
	forget_import: 'forgetImport',
	import_web_link: 'importWebLink',
	rename_version: 'renameVersion',
	upload_photograph: 'uploadPhotograph',
	search_recipes: 'searchRecipes',
	home_shelves: 'homeShelves',
	note_recipe_opened: 'noteRecipeOpened',
	meaning_search_status: 'meaningSearchStatus',
	accept_meaning_search_terms: 'acceptMeaningSearchTerms',
	decline_meaning_search: 'declineMeaningSearch',
	download_meaning_model: 'downloadMeaningModel',
	build_meaning_index: 'buildMeaningIndex',
	turn_off_meaning_search: 'turnOffMeaningSearch',
	get_recipe: 'getRecipe',
	get_thread: 'getThread',
	share_recipe: 'shareRecipe',
	end_share_link: 'endShareLink',
	get_share_link: 'getShareLink',
	get_public_address: 'getPublicAddress',
	set_public_address: 'setPublicAddress',
	export_bundle: 'exportBundle',
	make_sheet: 'makeSheet',
	make_shared_sheet: 'makeSharedSheet',
	import_bundle: 'importBundle',
	read_shared_recipe: 'readSharedRecipe',
	branch_point: 'branchPoint',
	divergence: 'divergence',
	set_reading: 'setReading',
	read_ingredient_lines: 'readIngredientLines',
	start_attempt: 'startAttempt',
	advance_attempt: 'advanceAttempt',
	finish_attempt: 'finishAttempt',
	edit_attempt: 'editAttempt',
	delete_attempt: 'deleteAttempt',
	promote_attempt_photograph: 'promoteAttemptPhotograph',
	set_as_cooked: 'setAsCooked',
	decline_promotion: 'declinePromotion',
	promote_as_cooked: 'promoteAsCooked',
	get_current_attempt: 'getCurrentAttempt',
	list_attempts: 'listAttempts',
	get_shopping_list: 'getShoppingList',
	shopping_basis: 'shoppingBasis',
	add_to_shopping_list: 'addToShoppingList',
	remove_from_shopping_list: 'removeFromShoppingList',
	set_shopping_yield: 'setShoppingYield',
	shopping_list_as_text: 'shoppingListAsText',
	empty_shopping_list: 'emptyShoppingList',
	add_loose_item: 'addLooseItem',
	remove_loose_item: 'removeLooseItem',
	list_foods: 'listFoods',
	get_food: 'getFood',
	set_food_name: 'setFoodName',
	remove_food_name: 'removeFoodName',
	set_food_cup_weight: 'setFoodCupWeight',
	list_merge_suggestions: 'listMergeSuggestions',
	preview_food_merge: 'previewFoodMerge',
	merge_food: 'mergeFood',
	delete_food: 'deleteFood',
	get_job: 'getJob',
	cancel_job: 'cancelJob',
	list_jobs: 'listJobs',
} as const;

/** The typed client: one method per Operation, named as the Catalogue names it. */
export interface KamosuClient {
	/** The version of this Kamosu, whether setup has happened, and the shortest password it accepts. */
	instanceStatus(input?: InstanceStatusInput): Promise<Answer<'instance_status'>>;
	/** Set the Language and measures this Person reads in. */
	setReadingPreferences(input: SetReadingPreferencesInput): Promise<Answer<'set_reading_preferences'>>;
	/** The Language and measures this Person reads in. Reading Measures live on the account rather than in a browser, so every Door and every device reads the same recipe the same way; the default is American, a stated convention rather than a guess about anybody. */
	getReadingPreferences(input?: GetReadingPreferencesInput): Promise<Answer<'get_reading_preferences'>>;
	/** Who this Credential names: the Person's permanent id, which is also their Hand, and the name they currently go by — the name every Version they wrote shows here, and the one they sign in with. */
	getPerson(input?: GetPersonInput): Promise<Answer<'get_person'>>;
	/** Change this Person's current reminder name. Every Version they ever wrote shows the new one on this instance, since a Hand is named live and nothing is keyed on the name; no id or fingerprint moves. It is also the name they sign in with, so a name somebody else here signs in with is refused. A Bundle already sent keeps the name it left with. */
	renamePerson(input: RenamePersonInput): Promise<Answer<'rename_person'>>;
	/** Who holds an account on this instance: their name, whether they administer it, and whether the account is disabled. Nothing about what they cook — the Operator administers and does not read (ADR 0007), so no recipe, Attempt, Cookbook or Kitchen of theirs is reachable from here. */
	listAccounts(input?: ListAccountsInput): Promise<Answer<'list_accounts'>>;
	/** Mint a one-use Invite link for a new Person. */
	mintInvite(input: MintInviteInput): Promise<Answer<'mint_invite'>>;
	/** Disable an account so it can no longer obtain a Credential. */
	disableAccount(input: DisableAccountInput): Promise<Answer<'disable_account'>>;
	/** Delete an account while preserving its Hand in history. The Person's name is freed for somebody new to sign in with; what they wrote keeps their Hand and the name they had. Disabling an account keeps the name. */
	deleteAccount(input: DeleteAccountInput): Promise<Answer<'delete_account'>>;
	/** Mint a one-use recovery link for a Person who forgot their password. */
	mintRecoveryLink(input: MintRecoveryLinkInput): Promise<Answer<'mint_recovery_link'>>;
	/** Make a Person an Operator, or stop them being one. The last Operator cannot be demoted (ADR 0007): an instance with nobody to administer it can never get one back, so the refusal is the point rather than a nicety. */
	setOperator(input: SetOperatorInput): Promise<Answer<'set_operator'>>;
	/** Take away Photographs nothing has pointed at for a week, and their Display Copies with them. Runs daily on its own; this asks for it now. */
	sweepPhotographs(input?: SweepPhotographsInput): Promise<Answer<'sweep_photographs'>>;
	/** Take a Backup now, as a Job: one archive holding a consistent copy of the database and every Photograph, written beside the database under /data. Kamosu keeps three — one taken daily, one weekly, one monthly — and takes them on its own; this asks for one now. A Job because an archive is the size of the library. It is never sent anywhere: fetch the bytes at GET /api/backups/<name>. */
	takeBackup(input?: TakeBackupInput): Promise<Answer<'take_backup'>>;
	/** The Backups this instance holds, newest first. Each can be fetched at GET /api/backups/<name>, under the same Credential as any Operation. */
	listBackups(input?: ListBackupsInput): Promise<Answer<'list_backups'>>;
	/** List this Person's browser Sessions by device and last use. `current` marks the Session asking, so it is never set when an Access Key asks. */
	listSessions(input?: ListSessionsInput): Promise<Answer<'list_sessions'>>;
	/** Rename one of your browser Sessions. A Session is named for its device when it signs in; this corrects the guess or names an older one. An ended Session is not renamed. */
	renameSession(input: RenameSessionInput): Promise<Answer<'rename_session'>>;
	/** End one of your browser Sessions. */
	revokeSession(input: RevokeSessionInput): Promise<Answer<'revoke_session'>>;
	/** Mint an Access Key for an agent to act as you, optionally read-only. */
	mintAccessKey(input: MintAccessKeyInput): Promise<Answer<'mint_access_key'>>;
	/** List this Person's Access Keys by name and last use. */
	listAccessKeys(input?: ListAccessKeysInput): Promise<Answer<'list_access_keys'>>;
	/** End one of your Access Keys. */
	revokeAccessKey(input: RevokeAccessKeyInput): Promise<Answer<'revoke_access_key'>>;
	/** Create a Kitchen: a group of People who see and cook from each other's Cookbooks. Its creator is its first member. */
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
	/** Remove a Person from a Kitchen — including yourself, to leave. Their Cookbook leaves with them; each member who stays keeps a Branch of every recipe of theirs they cooked, and they keep one of every recipe they cooked from the others. */
	removeKitchenMember(input: RemoveKitchenMemberInput): Promise<Answer<'remove_kitchen_member'>>;
	/** An Operator's power over a Kitchen: delete one nobody is left in. Nothing else about a Kitchen. */
	deleteKitchen(input: DeleteKitchenInput): Promise<Answer<'delete_kitchen'>>;
	/** What removing a Person from a Kitchen would leave each side, before anybody does it: how many recipes the members who stay keep, and how many the one leaving keeps. `person_id` defaults to you. */
	previewLeavingKitchen(input: PreviewLeavingKitchenInput): Promise<Answer<'preview_leaving_kitchen'>>;
	/** Your own Cookbook: its name, who writes it, how many recipes it holds, the Kitchens that see it and the Invites still waiting. */
	getCookbook(input?: GetCookbookInput): Promise<Answer<'get_cookbook'>>;
	/** Give your Cookbook a name of its own, or clear it back to its Co-authors' names with an empty or null one. Any Co-author may. */
	renameCookbook(input: RenameCookbookInput): Promise<Answer<'rename_cookbook'>>;
	/** Mint a one-use Invite for somebody to write your Cookbook with you. When they accept, their recipes and yours become one Cookbook either of you changes. */
	inviteToCookbook(input?: InviteToCookbookInput): Promise<Answer<'invite_to_cookbook'>>;
	/** End a Cookbook Invite nobody has used yet. */
	cancelCookbookInvite(input: CancelCookbookInviteInput): Promise<Answer<'cancel_cookbook_invite'>>;
	/** What accepting a Cookbook Invite would do, before you say yes: whose Cookbook it is, and how many recipes on each side become one. */
	readCookbookInvite(input: ReadCookbookInviteInput): Promise<Answer<'read_cookbook_invite'>>;
	/** Open a Cookbook Invite: your Cookbook joins the one it names, and every recipe in either becomes one Cookbook you both change. Spent on use. */
	acceptCookbookInvite(input: AcceptCookbookInviteInput): Promise<Answer<'accept_cookbook_invite'>>;
	/** Leave the Cookbook you write with others, taking your own Branch of every recipe in it with its whole history. Whoever started a recipe keeps the original; everyone else a copy. */
	leaveCookbook(input?: LeaveCookbookInput): Promise<Answer<'leave_cookbook'>>;
	/** Separate another Co-author from your Cookbook. They leave with a Branch of every recipe in it, as though they had left. */
	removeCookbookAuthor(input: RemoveCookbookAuthorInput): Promise<Answer<'remove_cookbook_author'>>;
	/** Create a Tag in your own Cookbook, named in one Language. A word the Cookbook already files under returns the Tag it already has rather than making a second. */
	createTag(input: CreateTagInput): Promise<Answer<'create_tag'>>;
	/** List every Tag your own Cookbook files by, each shown in the reader's Reading Language where it has a name there. With `everywhere`, every word any Cookbook you may see files by, one entry per word — what a shelf filters by. With a `kitchen_id`, the same for the Cookbooks seen in that one Kitchen of yours. */
	listTags(input: ListTagsInput): Promise<Answer<'list_tags'>>;
	/** Name a Tag in one Language, or change the name it has there. Reaches every recipe carrying it at once, and mints no Version. */
	renameTag(input: RenameTagInput): Promise<Answer<'rename_tag'>>;
	/** Merge two of a Kitchen's Tags into one: every recipe filed under the merged Tag is filed under the kept one instead. Mints no Version. */
	mergeTags(input: MergeTagsInput): Promise<Answer<'merge_tags'>>;
	/** Take a Tag out of a Kitchen's list and off every recipe carrying it. No recipe changes. */
	deleteTag(input: DeleteTagInput): Promise<Answer<'delete_tag'>>;
	/** File a recipe under one of its Kitchen's Tags, or take it back out. Mints no Version: filing is not what a recipe is. */
	setRecipeTag(input: SetRecipeTagInput): Promise<Answer<'set_recipe_tag'>>;
	/** Relate one of your Cookbook's Recipes to any Recipe you may see, or take that single two-way, untyped link back off. Your Cookbook keeps the link. It never changes either Recipe or travels in a Bundle or Share. Name the far end with `related_branch_id`, or with `related_lineage_id` where the Recipe there has since been deleted — exactly one of the two. */
	setRelatedRecipe(input: SetRelatedRecipeInput): Promise<Answer<'set_related_recipe'>>;
	/** Create a Recipe: a Lineage, a Branch in this Kitchen, and a first Version. A title is all it needs. */
	createRecipe(input: CreateRecipeInput): Promise<Answer<'create_recipe'>>;
	/** Save a new state of a Recipe onto a Branch — the whole recipe as written, replacing what was there. A rapid re-save by the same Hand collapses into the Version already being shaped rather than starting a new one. Changing a recipe your Cookbook did not write — a Kitchen-mate's, or one that arrived — is a Copy: it starts a new Branch of the same Lineage in your own Cookbook, starting at the Version you changed and carrying the whole chain behind it — the Branch you changed is left untouched. The Branch must be one you may see. */
	saveRecipeVersion(input: SaveRecipeVersionInput): Promise<Answer<'save_recipe_version'>>;
	/** Start a variation of a recipe: a Branch of it, unchanged, in your own Cookbook, under a name you give it ("Vegetarian"). Changing one never changes the other. */
	startVariation(input: StartVariationInput): Promise<Answer<'start_variation'>>;
	/** Name one of your Cookbook's Branches of a recipe, or clear its name. A Cookbook keeps one unnamed Branch of a recipe in each Language, so a second one needs a name. */
	renameBranch(input: RenameBranchInput): Promise<Answer<'rename_branch'>>;
	/** Take one recipe off the shelf for good. It is gone from the shelf, from search and from every member of its Kitchen, and nothing brings it back. **One Branch**: a translation is an ordinary Branch, so deleting the English one leaves the French one whole, and another Kitchen's copy of the same recipe is untouched. **The cooking history stays.** Every Attempt ever made from this recipe keeps its rating, its note and its Photographs, and the Cooked diary keeps each entry under the name the recipe was known by. So does a Shopping List holding it, which says it can no longer be read rather than quietly dropping it. No Version is ever deleted, by this or by anything else. A live Share Link stops working. */
	deleteRecipe(input: DeleteRecipeInput): Promise<Answer<'delete_recipe'>>;
	/** Read a whole recipe pasted as text into a title, an ingredient list and a method. Decides only what each line IS — an Ingredient Line, a Step, a Section — and never what it says: every line comes back exactly as pasted, with no amount extracted, no rewording and no reordering (ADR 0002). Nothing is guessed beyond the split and the title: no Yield, no times, no Source, and no Component (ADR 0008). It writes nothing anywhere — what comes back is shown to whoever pasted it, who moves the boundary if it landed wrong, and only then is a recipe saved by an ordinary create_recipe or save_recipe_version. The boundary is the index in `lines` where the method starts, so moving it re-splits the same answer without asking again. */
	readPastedRecipe(input: ReadPastedRecipeInput): Promise<Answer<'read_pasted_recipe'>>;
	/** Translate a recipe: start an ordinary Branch of the same Lineage in another Language, whose first Version records which Version of the source it renders. There is no Translation object — what this makes is a Branch, and every Operation from here on is the ordinary one. Its chain starts fresh rather than carrying the source's, which is what separates it from a Copy: different words rendering the same dish, with a history of their own. An agent translating calls this under the Person's own Credential and is a scribe, not an author. */
	startTranslation(input: StartTranslationInput): Promise<Answer<'start_translation'>>;
	/** Say what Language a recipe is written in. The only thing that acts on a save's language offer — Kamosu detects and offers, and never changes a Language without the cook saying so. Changing it makes a Version, so the change leaves a trace in the recipe's own history. Setting it to `unknown` says the recipe is honestly more than one Language: from then on it is offered nothing, marked nothing, and shown to every reader whatever they read in. */
	setRecipeLanguage(input: SetRecipeLanguageInput): Promise<Answer<'set_recipe_language'>>;
	/** Bring a batch of already-read recipes into your own Cookbook, as a Job. Matched by foreign id against this Cookbook's ledger for the source kind, so re-running finds what it already made instead of doubling it; a recipe found changed is offered for review, never written over. Reading the outside source itself — a file, a page, a Bundle — is each importer's own job. */
	import(input: ImportInput): Promise<Answer<'import'>>;
	/** Bring in a Crouton library, as a Job: the whole export (a zip of .crumb files) or one .crumb. Each recipe lands in your own Cookbook through the same ledger `import` uses, keyed by its Crouton id, so running it again matches instead of doubling the library. Ingredient Lines are rebuilt from Crouton's split fields; the site's favicon and Crouton's nutrition text are left out. Send the file to POST /api/uploads and pass the `upload_id` it answers, or pass it base64-encoded as `data`. */
	importCrouton(input: ImportCroutonInput): Promise<Answer<'import_crouton'>>;
	/** What has been brought into your Kitchens from outside, and what happened each time. One entry per source — a Crouton library, recipe files, web pages — each holding how many recipes its ledger remembers and every arrival you asked for, newest first. An arrival names the Job whose Report `get_job` serves, so what happened is read back long after the screen that started it closed. Listed is an event, never a mark on a recipe: an imported recipe is an ordinary recipe and says nothing about where it came from (ADR 0025). */
	listImports(input?: ListImportsInput): Promise<Answer<'list_imports'>>;
	/** Throw an Import's ledger away whole — the memory of which outside recipe became which of yours. Every recipe it made stays exactly as it is. Once forgotten, importing the same file again brings everything in as new, so do this when the place it came from is gone. */
	forgetImport(input: ForgetImportInput): Promise<Answer<'forget_import'>>;
	/** Bring in a recipe straight from a URL, as a Job. Reads the page's schema.org JSON-LD (#70) — no per-site scraping, no LLM fallback — and lands it in your own Cookbook through the same ledger `import` uses, keyed by the page's own address. Fetching is bound to public addresses at the dialled address and at every redirect (ADR 0033), and — because a page's own text can tell an agent to fetch another URL — always takes the single depth-one lane, never more than one fetch in flight regardless of who is signed in. */
	importWebLink(input: ImportWebLinkInput): Promise<Answer<'import_web_link'>>;
	/** Rename a Version — the one thing about it that can change later. An absent or empty name clears it. Targeted by the Branch's own sequence number, since the same content can recur more than once on one Branch, each occurrence named on its own. Only the Person who saved that Version may rename it. */
	renameVersion(input: RenameVersionInput): Promise<Answer<'rename_version'>>;
	/** Upload a Photograph, base64-encoded — the fallback for a Door that cannot carry raw bytes (ADR 0001). A browser uses the out-of-band `POST /api/photographs` instead. Two uploads of the same picture answer the same id. */
	uploadPhotograph(input: UploadPhotographInput): Promise<Answer<'upload_photograph'>>;
	/** The shelf, and searching it. With no query: everything the Kitchens this Person cooks in hold, merged, alphabetical, one entry per Lineage, each titled in the reader's Reading Language with a marked fallback. With a query: the same shelf narrowed to what matched, an exact title first, every entry quoting the line that matched. One Operation either way — Meaning Search arrives here rather than beside it (ADR 0027, ADR 0029). */
	searchRecipes(input: SearchRecipesInput): Promise<Answer<'search_recipes'>>;
	/** Home: the computed shelves that answer *show me something* rather than handing back a search box — cooked most, quick tonight, never cooked, recently opened. Each is one card per Lineage in the reader's Reading Language, in the same shape the library's shelf answers in. A shelf with nothing on it is left out rather than sent empty, so an instance holding no recipes answers with no shelves at all. All four are counted from recipes and Attempts that already exist, except *recently opened*, which reads what `note_recipe_opened` remembered (ADR 0011, ADR 0027). */
	homeShelves(input?: HomeShelvesInput): Promise<Answer<'home_shelves'>>;
	/** Remember that the caller opened this recipe, for Home's *recently opened* shelf. One fact per Person per Lineage — opening a recipe's French Branch and its English one is opening the same recipe — and opening it again moves the time rather than adding a row. It is private to the Person, never travels, and is in no fingerprint, Vault or Bundle: an instance that lost it would lose the order of one shelf and nothing else (ADR 0027). */
	noteRecipeOpened(input: NoteRecipeOpenedInput): Promise<Answer<'note_recipe_opened'>>;
	/** Whether Meaning Search is on here, what model it would use, who accepted that model's terms — and whether this caller should be offered it. Answers on every instance, including the many that will never turn it on. */
	meaningSearchStatus(input?: MeaningSearchStatusInput): Promise<Answer<'meaning_search_status'>>;
	/** Accept the terms of the model Meaning Search needs. Kamosu ships no weights (ADR 0029): the person who accepts the terms is the person the terms are about, and the acceptance keeps the Hand that made it and whether it arrived by login or by Access Key. Available at both Doors — a web-only carve-out would be the first hole in Parity, and would stop nothing anyway. */
	acceptMeaningSearchTerms(input?: AcceptMeaningSearchTermsInput): Promise<Answer<'accept_meaning_search_terms'>>;
	/** Decline the model's terms. Meaning Search stays off and the offer is never made again on this instance — a question already answered, asked twice, is a nag. */
	declineMeaningSearch(input?: DeclineMeaningSearchInput): Promise<Answer<'decline_meaning_search'>>;
	/** Fetch the Meaning Search model into /data, as a Job. No weights ship in the image; this is the only way any arrive, and only after the terms have been accepted. The download is pinned to one revision and verified against a manifest, so a half-finished one is never mistaken for a model. */
	downloadMeaningModel(input?: DownloadMeaningModelInput): Promise<Answer<'download_meaning_model'>>;
	/** Read the library into the Meaning Search index, as a Job, and turn Meaning Search on. Incremental: what is read is what the index does not already hold, so the first run is the whole library and every later one is whatever changed. The index is derived from the recipes and can be rebuilt at any time. Kamosu also does this by itself, within the minute, whenever a recipe changes. */
	buildMeaningIndex(input?: BuildMeaningIndexInput): Promise<Answer<'build_meaning_index'>>;
	/** Stop matching on meaning and throw the index away. Discards nothing that cannot be rebuilt — the index is derived from the recipes — and keeps both the acceptance, which is history, and the downloaded weights, so turning it back on is a rebuild rather than another download. */
	turnOffMeaningSearch(input?: TurnOffMeaningSearchInput): Promise<Answer<'turn_off_meaning_search'>>;
	/** Read a Recipe: the Branch as it stands and its whole chain of Versions, oldest first. Each Version's `measured` lines are scaled to `wanted_yield` where one is given (null for the recipe as written), and otherwise to the Yield the caller's own In Progress Attempt is cooking to; `scaled_to` says which, or is null where the amounts are as written. Nothing is stored. */
	getRecipe(input: GetRecipeInput): Promise<Answer<'get_recipe'>>;
	/** Read the Thread: every Version of every Branch of one Lineage this Person can see, oldest first per Branch, with every Attempt hanging off it. branch_id is only the entry point — any Branch of the Lineage answers the same Thread. */
	getThread(input: GetThreadInput): Promise<Answer<'get_thread'>>;
	/** Turn a Recipe's Share Link on, and answer the link. One permanent, unguessable address per Recipe, never expiring, freely passed on. Asking twice for a Recipe already shared answers the link it already has rather than minting a second one. The link's Secret is answered exactly once — here, at the moment it is minted — because only its hash is stored. The instance's public address is asked for at the first Share Link and stored once; a link is kept as a token rather than a URL, so setting the address later makes every link already minted render correctly. */
	shareRecipe(input: ShareRecipeInput): Promise<Answer<'share_recipe'>>;
	/** End a Recipe's Share Link. Permanent: the link stops working and turning sharing back on mints a new one, so a withdrawn link stays dead. It reaches no copy already sent, and Kamosu says so rather than letting that be discovered. */
	endShareLink(input: EndShareLinkInput): Promise<Answer<'end_share_link'>>;
	/** Whether a Recipe is shared, and by whom. The link's URL is answered only at the moment it is minted, since only the Secret's hash is stored — so this says a link exists without being able to reprint it. */
	getShareLink(input: GetShareLinkInput): Promise<Answer<'get_share_link'>>;
	/** Where this instance currently says it is reachable from outside, or nothing if it has never been asked. The Operator's half of `set_public_address`: changing an address you cannot see is a guess. */
	getPublicAddress(input?: GetPublicAddressInput): Promise<Answer<'get_public_address'>>;
	/** Change where this instance says it is reachable from outside. Kept in the database and never in an environment variable, so moving an instance is one act rather than a redeployment. It fixes the future, not the past: Share Links minted after it carry the new address, while a link already sent stays the text it was sent as and cannot be reissued — only the secret's hash is kept, so Kamosu can no longer print that link at all. */
	setPublicAddress(input: SetPublicAddressInput): Promise<Answer<'set_public_address'>>;
	/** Write a Bundle of one recipe: a plain zip holding a readable Markdown note per recipe with its Thread beneath it, its Photographs, and a hidden .kamosu/ sidecar carrying every Version complete back to the first, the Readings and the ids. It carries the Branch named, its Translations, and every Component it needs as a Passenger. This answers what the Bundle holds; fetch its bytes at GET /api/bundles/<branch_id> under the same Credential. Nothing is sent anywhere and nothing is changed. */
	exportBundle(input: ExportBundleInput): Promise<Answer<'export_bundle'>>;
	/** Set a Sheet of one recipe: the Branch as it stands on this Person's screen, set for paper as a PDF. It carries the recipe and not the library — no Tags, Attempts, Thread or past Versions. Written Ingredient Lines are printed and Readings are not, except the amount beneath a line when a cooking has scaled the recipe; Components unfold after it, parent first, each already scaled. Letter for US Reading Measures, A4 otherwise. `wanted_yield` is the Yield the screen is scaled to, as `get_recipe` takes it. When the Job completes, fetch the PDF at GET /api/sheets/<job_id> under the same Credential. Nothing is changed. */
	makeSheet(input: MakeSheetInput): Promise<Answer<'make_sheet'>>;
	/** Set a Sheet of the recipe a Share Link shows, for anyone holding the link — no account needed. The recipe is printed as written, with its Components unfolded after it at the amount each line asks for. `language` picks one of the link's Translations; `locale` is the reader's locale (a US or Canadian one prints Letter, anything else A4) and decides nothing but the paper. When the Job completes, fetch the PDF at GET /api/sheets/<job_id>. */
	makeSharedSheet(input: MakeSharedSheetInput): Promise<Answer<'make_shared_sheet'>>;
	/** Receive a Bundle into your own Cookbook, as a Job. Every recipe it carries is placed under the sender's Hands and travels on under the sender's ids, its Versions, Readings and Photographs exactly as they were sent, while your Cookbook holds it under an id of this instance's own; one your Cookbook already holds is extended by whatever the Bundle carries past it, so the same friend's next Bundle continues their recipe. Another Cookbook here holding it is no part of the question: each Cookbook receives its own copy. Receiving makes nothing of your own — changing what arrived does. A recipe whose history is damaged arrives as a new recipe of your own with no history, and the Import Report says so. Send the file to POST /api/uploads and pass the `upload_id` it answers, or pass it base64-encoded as `data`. */
	importBundle(input: ImportBundleInput): Promise<Answer<'import_bundle'>>;
	/** Read a Recipe through its Share Link token: the Recipe as it stands, its Translations, and its Thread complete back to the first Version with every name and *what changed* line. Never an Attempt, a rating or an Attempt photograph. Public, because holding the token is the whole of the permission — this is what the Share Link page consumes, and the page is not an Operation, so Parity is untouched. */
	readSharedRecipe(input: ReadSharedRecipeInput): Promise<Answer<'read_shared_recipe'>>;
	/** The last Version two Branches share, found by walking both chains back until they meet — never declared, always computed. A chain that does not converge on a shared first Version answers a damaged-Bundle error rather than a guess. */
	branchPoint(input: BranchPointInput): Promise<Answer<'branch_point'>>;
	/** Two Branches of one Lineage laid over each other, so a screen can show two whole recipes with a switch between them rather than a difference (ADR 0014). Every row carries both sides' own words; a line only one side has is a Ghost. Which line is which is read against the Branch Point, never by an id stapled to a line (ADR 0019), and an uncertain reading declines to pair rather than claiming a connection. */
	divergence(input: DivergenceInput): Promise<Answer<'divergence'>>;
	/** Correct the Reading on one Ingredient Line of a Recipe's current state — an amount, a Unit and a target, sent together as the whole new Reading (never a per-field patch, the same convention save_recipe_version uses for the whole recipe). Mints no Version and appears in no history (ADR 0021). All of them left out together clears the Reading, taking the line back to fully unread. The target is either a Food's written word or — as `lineage_id` — the Recipe this line names, which makes the Ingredient a Component (ADR 0008); never both, and a Lineage this instance does not hold is accepted, because a Component goes on naming its recipe when the recipe is gone. */
	setReading(input: SetReadingInput): Promise<Answer<'set_reading'>>;
	/** Read every Ingredient Line in the library that nothing has read yet, as a Job, laying a Reading over each one Kamosu can make sense of. Touches no written line and makes no Version. A line already carrying a Reading is left alone, so a correction is never overwritten, and a line Kamosu cannot read is left unread, which is an ordinary state for a line rather than a failure. Kamosu also reads the lines of every recipe as it is written or imported, so this is for a library that predates it. */
	readIngredientLines(input?: ReadIngredientLinesInput): Promise<Answer<'read_ingredient_lines'>>;
	/** Start cooking a Recipe: creates the Attempt, or hands back the one already In Progress for this Lineage — the cooking screen is that Attempt, never a second thing beside it. Pinned by fingerprint to the Branch's head Version at this moment, or to version_id — an older Version read back from the Thread — when one is given. Anyone who can see the recipe may. */
	startAttempt(input: StartAttemptInput): Promise<Answer<'start_attempt'>>;
	/** Move an In Progress Attempt forward: which Step, which Ingredients are ticked, and the Yield being cooked to — a fact about this cooking, never a deviation. Any of the three, each sent whole rather than patched. */
	advanceAttempt(input: AdvanceAttemptInput): Promise<Answer<'advance_attempt'>>;
	/** End an In Progress Attempt, taking the judgement that lands with it: a rating, a note and Photographs, all optional. Ending is not what makes the cooking real — starting already did — only what stops it being In Progress, so a cook who says nothing still cooked. */
	finishAttempt(input: FinishAttemptInput): Promise<Answer<'finish_attempt'>>;
	/** Change an Attempt's free text, its rating or its Photographs, whether it is still In Progress or long finished — an Attempt is freely editable by its cook, unlike the recipe it was cooked from. */
	editAttempt(input: EditAttemptInput): Promise<Answer<'edit_attempt'>>;
	/** Delete an Attempt outright — the explicit way a false start is undone, or any cooking record put away. Never soft-deleted: this is the whole of how an Attempt leaves. */
	deleteAttempt(input: DeleteAttemptInput): Promise<Answer<'delete_attempt'>>;
	/** Make a picture taken while cooking the recipe's Main Photo, or a Step's photo — so the picture you actually took becomes the recipe's picture. This is an ordinary edit making a Version, with everything that follows from it: a rapid re-save folding into the Version already being shaped, and a Copy in your own Cookbook where you do not write the Branch's. The Branch must be one you may see. The Attempt keeps the picture too; promoting is not moving. */
	promoteAttemptPhotograph(input: PromoteAttemptPhotographInput): Promise<Answer<'promote_attempt_photograph'>>;
	/** Write down what you actually cooked, where it differed from the recipe: the whole recipe as you cooked it, in ordinary Ingredient Lines and ordinary Step text — a line reworded, one added, one dropped, a step grown. Not a record of differences; the same shape a Version takes. Sending back exactly what the recipe says, or null, stores nothing at all, because cooking a recipe as it is written changes nothing. Changes no recipe and makes no Version: that is Promotion, and it is a separate act. */
	setAsCooked(input: SetAsCookedInput): Promise<Answer<'set_as_cooked'>>;
	/** Say that the words a cooking used belong in the diary and not in the recipe — or take that back. It answers the offer and nothing else: what was cooked stays on the cooking, whole. Remembered, because a question already answered, asked twice, is a nag. */
	declinePromotion(input: DeclinePromotionInput): Promise<Answer<'decline_promotion'>>;
	/** Promotion: turn what you cooked into a real Version of the recipe. Mechanical — the As Cooked is already a whole recipe, so nothing is retyped and nothing is reconciled. It is an ordinary edit and inherits all of one: a rapid re-save folds into the Version being shaped, and a Branch whose Cookbook you do not write becomes a Copy in your own. The Branch must be one you may see. Promoting a cooking of an older Version appends onto wherever the Branch stands now — a Version, never a merge. The Attempt is left exactly as it was, still saying which Version it cooked. */
	promoteAsCooked(input: PromoteAsCookedInput): Promise<Answer<'promote_as_cooked'>>;
	/** Read the caller's own In Progress Attempt for a Lineage, if any — how two devices cooking the same dish stay in step, and whether resuming should still be offered. */
	getCurrentAttempt(input: GetCurrentAttemptInput): Promise<Answer<'get_current_attempt'>>;
	/** The cooking diary: every Attempt the caller has made, newest first, across every recipe — sorted by date rather than by recipe, which is what makes *what did I cook that week* answerable. Unfinished and In Progress cookings are in it too, because starting is what makes a cooking real. Each entry names the recipe it was cooked from, and still names it after that recipe has left the caller's shelf. */
	listAttempts(input?: ListAttemptsInput): Promise<Answer<'list_attempts'>>;
	/** Your Shopping List: the recipes you chose, and the rows worked out from them. Everyone has exactly one; it has no name and is never archived. The rows are computed on every read and stored nowhere, so editing a chosen recipe or correcting a Reading changes the list at once. A row names a Food in your Reading Language and merges every mention of it; amounts add where the Units honestly convert, saying about, and ride side by side where they do not. Nothing here is ticked off. */
	getShoppingList(input?: GetShoppingListInput): Promise<Answer<'get_shopping_list'>>;
	/** What one recipe puts on a Shopping List before anything is added up: each Ingredient Line, the Food it was read as and the name that Food goes by for you, how much it said, its Unit, and what a cup of the Food weighs. Every recipe this one includes is unfolded to the bottom and its lines are here too, already carrying their share, so a pizza's flour and its dough's flour add up to one thing to buy. Always the Branch's latest Version. It is how a phone with no network works out the list's rows itself for the recipes it holds (#77); get_shopping_list is the list itself. */
	shoppingBasis(input: ShoppingBasisInput): Promise<Answer<'shopping_basis'>>;
	/** Choose a recipe to shop for, at a Yield, a multiplier (a Yield with an empty noun) or as it is written. It holds the Branch at its latest Version, never a Lineage and never pinned, so a recipe edited between the planning and the shopping is right in the shop. Choosing one already on the list is not an error and makes no second entry: it moves that entry to the Yield given here, or back to the recipe as written when none is. Answers the whole list. */
	addToShoppingList(input: AddToShoppingListInput): Promise<Answer<'add_to_shopping_list'>>;
	/** Take a recipe off your Shopping List. Works whether or not it can still be read, which is exactly the entry somebody most wants gone. Answers the whole list. */
	removeFromShoppingList(input: RemoveFromShoppingListInput): Promise<Answer<'remove_from_shopping_list'>>;
	/** Say how much of a chosen recipe you are shopping for — an amount and its noun, a multiplier (an amount with an empty noun: twice the recipe is `2`), or null for the recipe as written. Every amount it contributes moves with it. Answers the whole list. */
	setShoppingYield(input: SetShoppingYieldInput): Promise<Answer<'set_shopping_yield'>>;
	/** Your Shopping List as plain text, ready to be carried out of Kamosu. Nothing is ticked off here, because the list leaves and something else holds the ticks — Apple Notes, through a Shortcut. The text opens with a header line, the date and the recipes it was built from (and any that can no longer be read), because a note accumulates and three trips appended with no divider are a wall. Under it, one flat alphabetical list with one Markdown checklist line (`- [ ] `) per thing to buy, so each line becomes one checkbox; a row whose amounts could not be added stays on its one line, naming the dish behind each amount. This only reads: emptying the list afterwards is a separate Operation, offered and never done on the way out. */
	shoppingListAsText(input?: ShoppingListAsTextInput): Promise<Answer<'shopping_list_as_text'>>;
	/** Empty your Shopping List — every recipe chosen and every typed line at once. Offered after the list has left as text and never done on the way out: a list that emptied itself when it was sent would be silent and unrecoverable. Answers the whole list. */
	emptyShoppingList(input: EmptyShoppingListInput): Promise<Answer<'empty_shopping_list'>>;
	/** Type a line straight onto your Shopping List — bin bags, coffee. Kept exactly as typed and never read, so it carries no amount and merges with nothing: typing flour beside a recipe that wants flour gives two lines. Answers the whole list. */
	addLooseItem(input: AddLooseItemInput): Promise<Answer<'add_loose_item'>>;
	/** Take one typed line off your Shopping List. Answers the whole list. */
	removeLooseItem(input: RemoveLooseItemInput): Promise<Answer<'remove_loose_item'>>;
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
	/** The Operator's worklist: every note that two Foods are probably one thing, with the words that said so. Evidence, never an instruction — nothing merges itself. */
	listMergeSuggestions(input?: ListMergeSuggestionsInput): Promise<Answer<'list_merge_suggestions'>>;
	/** Say how many Ingredient Lines a Merge would move, and how many Reading rows, without moving any of them. A Merge cannot be undone and refuses to run until this figure is said back to it, so this saying is its safety net rather than a courtesy. */
	previewFoodMerge(input: PreviewFoodMergeInput): Promise<Answer<'preview_food_merge'>>;
	/** Join two Foods into one: the survivor takes every name both had, every Reading pointing at the other points at it instead, and every Merge Suggestion naming either is cleared. ingredient_lines is the figure preview_food_merge announced, said back — a Merge that does not match it is refused. Where the two disagree about Cup Weight, cup_weight_grams says which of the two figures survives. There is no un-merge in v1. */
	mergeFood(input: MergeFoodInput): Promise<Answer<'merge_food'>>;
	/** Delete a Food nothing points at. One a Reading still points at is refused: what a Food knows was expensive to learn and is never discarded by an unrelated act. */
	deleteFood(input: DeleteFoodInput): Promise<Answer<'delete_food'>>;
	/** Read one Job: its state, its progress, and its result or the reason it failed. Readable by the Person who asked, or by anyone when no Person did. */
	getJob(input: GetJobInput): Promise<Answer<'get_job'>>;
	/** Cancel a Job you asked for: acknowledged always, honoured while it still waits in line. */
	cancelJob(input: CancelJobInput): Promise<Answer<'cancel_job'>>;
	/** List the Jobs this Person has asked for, newest first. */
	listJobs(input?: ListJobsInput): Promise<Answer<'list_jobs'>>;
}
