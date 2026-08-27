// Generated from the Catalogue by `just client`. Do not edit.
//
// Every Operation Kamosu offers is declared once in src/catalogue.rs and both
// Doors are built by walking that list (ADR 0001). These types are the third
// thing built from it, so the interface cannot ask for a shape the Core does
// not serve. Change the Catalogue and re-run `just client`; `just check`
// fails if what is committed here has drifted.

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
export type Answer<N extends OperationName> = Operations[N]['output'];

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
	list_sessions: 'listSessions',
	revoke_session: 'revokeSession',
	mint_access_key: 'mintAccessKey',
	list_access_keys: 'listAccessKeys',
	revoke_access_key: 'revokeAccessKey',
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
	/** Read one Job: its state, its progress, and its result or the reason it failed. Readable by the Person who asked, or by anyone when no Person did. */
	getJob(input: GetJobInput): Promise<Answer<'get_job'>>;
	/** Cancel a Job you asked for: acknowledged always, honoured while it still waits in line. */
	cancelJob(input: CancelJobInput): Promise<Answer<'cancel_job'>>;
	/** List the Jobs this Person has asked for, newest first. */
	listJobs(input?: ListJobsInput): Promise<Answer<'list_jobs'>>;
}
