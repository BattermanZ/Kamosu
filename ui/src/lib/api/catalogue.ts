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

/** End one of your browser Sessions. */
export type RevokeSessionInput = {
	session_id: string;
};
/** What revoke_session answers. */
export type RevokeSessionOutput = {
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
	revoke_session: {
		input: RevokeSessionInput;
		output: RevokeSessionOutput;
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
	revoke_session: 'revokeSession',
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
	/** End one of your browser Sessions. */
	revokeSession(input: RevokeSessionInput): Promise<Answer<'revoke_session'>>;
	/** Read one Job: its state, its progress, and its result or the reason it failed. Readable by the Person who asked, or by anyone when no Person did. */
	getJob(input: GetJobInput): Promise<Answer<'get_job'>>;
	/** Cancel a Job you asked for: acknowledged always, honoured while it still waits in line. */
	cancelJob(input: CancelJobInput): Promise<Answer<'cancel_job'>>;
	/** List the Jobs this Person has asked for, newest first. */
	listJobs(input?: ListJobsInput): Promise<Answer<'list_jobs'>>;
}
