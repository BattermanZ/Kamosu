/**
 * The screen-seam harness: a stand-in Kamosu, derived from the Catalogue.
 *
 * A screen test needs answers without a database behind them. The danger in
 * that is always the same — the stubbed answer drifts from what the real
 * Operation returns, the tests keep passing, and the kitchen breaks. Here it
 * cannot: the stand-in reads the *same* generated declarations the client is
 * built from, and every answer a test programs is checked against the output
 * schema the Catalogue declares before any screen sees it.
 *
 * So a test can lie about the values. It cannot lie about the shape.
 *
 * ```ts
 * const kamosu = standIn({ instance_status: { version: '0.1.0', setup_complete: true, password_minimum: 15 } });
 * render(Home, { props: { kamosu: kamosu.client } });
 * ```
 */

import { CATALOGUE, type Operations, type OperationName } from './catalogue';
import { createClient, OperationError, type ErrorKind, type Transport } from './client';
import type { KamosuClient } from './catalogue';

/**
 * One Operation's declaration, widened from the generated literal types. The
 * generated `CATALOGUE` is `as const`, which is exactly right for the types the
 * client is built from and exactly wrong here: this code must keep working for
 * a Catalogue that declares Jobs, and one that happens not to.
 */
interface Declaration {
	name: string;
	summary: string;
	permission: 'public' | 'person';
	kind: 'immediate' | 'job';
	input_schema: SchemaNode;
	output_schema: SchemaNode;
}

const DECLARATIONS = new Map<string, Declaration>(
	CATALOGUE.map((declaration) => [declaration.name, declaration as unknown as Declaration]),
);

/** What a test says an Operation should do. A value, or a refusal. */
export type Answered<N extends OperationName> =
	Operations[N]['output'] | { refuse: ErrorKind; message?: string };

/** What a Job answers when asked for: an id at once, nothing more (ADR 0032). */
export type AnsweredJob = { job_id: string } | { refuse: ErrorKind; message?: string };

/**
 * A function answer is handed what the screen sent, so a test can answer a
 * search for *naan* differently from a search for *rice*.
 */
export type Answers = {
	[N in OperationName]?: Operations[N]['kind'] extends 'job'
		? AnsweredJob | ((input: Operations[N]['input']) => AnsweredJob)
		: Answered<N> | ((input: Operations[N]['input']) => Answered<N>);
};

export interface StandIn {
	/** The client to hand a screen. Same methods as the real one, by construction. */
	client: KamosuClient;
	/** Every ask that reached the stand-in, in order — what a screen actually did. */
	calls: { operation: OperationName; input: unknown }[];
	/** Change an answer mid-test: a Job that was queued and is now completed. */
	answer<N extends OperationName>(operation: N, answer: Answers[N]): void;
}

const isRefusal = (value: unknown): value is { refuse: ErrorKind; message?: string } =>
	typeof value === 'object' && value !== null && 'refuse' in value;

/**
 * Check an answer against what the Catalogue declares. Only the properties the
 * schema names are examined — this is the seam's guarantee, not a JSON Schema
 * implementation, and the shapes the Catalogue declares are deliberately plain.
 */
function checkAgainstSchema(operation: string, schema: SchemaNode, value: unknown, path: string) {
	const where = path ? `${operation} → ${path}` : operation;

	if (!schema || Object.keys(schema).length === 0) return; // `{}`: genuinely anything.

	if (schema.enum) {
		if (!schema.enum.includes(value as never)) {
			throw new Error(
				`the stand-in was given ${JSON.stringify(value)} for ${where}, which the Catalogue does not allow (${schema.enum.join(', ')})`,
			);
		}
		return;
	}

	const allowed = Array.isArray(schema.type) ? schema.type : schema.type ? [schema.type] : [];
	if (allowed.length === 0) return;

	if (!allowed.some((type) => matchesType(type, value))) {
		throw new Error(
			`the stand-in was given ${describe(value)} for ${where}, but the Catalogue declares ${allowed.join(' | ')}`,
		);
	}

	if (allowed.includes('object') && typeof value === 'object' && value !== null) {
		const record = value as Record<string, unknown>;
		for (const name of schema.required ?? []) {
			if (!(name in record)) {
				throw new Error(`the stand-in's answer for ${where} is missing ${name}`);
			}
		}
		for (const [name, child] of Object.entries(record)) {
			const childSchema = schema.properties?.[name];
			if (!childSchema) {
				if (schema.additionalProperties === false) {
					throw new Error(
						`the stand-in's answer for ${where} carries ${name}, which the Catalogue does not declare`,
					);
				}
				continue;
			}
			checkAgainstSchema(operation, childSchema, child, path ? `${path}.${name}` : name);
		}
	}

	if (allowed.includes('array') && Array.isArray(value) && schema.items) {
		value.forEach((item, index) =>
			checkAgainstSchema(operation, schema.items as SchemaNode, item, `${path}[${index}]`),
		);
	}
}

interface SchemaNode {
	type?: string | readonly string[];
	enum?: readonly unknown[];
	properties?: Record<string, SchemaNode>;
	required?: readonly string[];
	items?: SchemaNode;
	additionalProperties?: boolean;
}

/**
 * Whether a value is of one JSON Schema type. The list mirrors what
 * `generate-client.mjs` renders — the two live in different processes, so they
 * cannot share code; both therefore refuse an unrecognised type rather than
 * waving it through, and adding one to the Catalogue fails loudly in both.
 */
function matchesType(type: string, value: unknown): boolean {
	switch (type) {
		case 'null':
			return value === null;
		case 'string':
			return typeof value === 'string';
		case 'boolean':
			return typeof value === 'boolean';
		case 'integer':
			return typeof value === 'number' && Number.isInteger(value);
		case 'number':
			return typeof value === 'number';
		case 'array':
			return Array.isArray(value);
		case 'object':
			return typeof value === 'object' && value !== null && !Array.isArray(value);
		default:
			throw new Error(
				`the Catalogue declares the type ${type}, which the screen-seam harness does not know how to check — teach stand-in.ts and generate-client.mjs about it together`,
			);
	}
}

const describe = (value: unknown): string =>
	value === null ? 'null' : Array.isArray(value) ? 'an array' : typeof value;

/** The shape a Job ask always answers, whatever the Job is. */
const JOB_ASK_SCHEMA: SchemaNode = {
	type: 'object',
	properties: { job_id: { type: 'string' } },
	required: ['job_id'],
	additionalProperties: false,
};

/**
 * Build a stand-in Kamosu that answers what the tests say — and refuses to
 * answer anything the Catalogue does not declare.
 */
export function standIn(answers: Answers = {}): StandIn {
	const programmed: Answers = { ...answers };
	const calls: StandIn['calls'] = [];

	const transport: Transport = async (operation, input) => {
		const declaration = DECLARATIONS.get(operation);
		if (!declaration) {
			// Only reachable if a test reaches past the generated client.
			throw new OperationError(
				operation,
				'unknown_operation',
				`${operation} is not in the Catalogue.`,
			);
		}
		calls.push({ operation, input });

		// The input is held to the Catalogue too: a screen that sends a field no
		// Operation declares fails here rather than in a kitchen.
		checkAgainstSchema(operation, declaration.input_schema, input ?? {}, 'input');

		const programmedAnswer = programmed[operation];
		if (programmedAnswer === undefined) {
			throw new Error(
				`the stand-in was asked for ${operation}, which this test did not answer. Add it to standIn({ … }).`,
			);
		}

		const answer =
			typeof programmedAnswer === 'function'
				? (programmedAnswer as (input: unknown) => unknown)(input ?? {})
				: programmedAnswer;

		if (isRefusal(answer)) {
			throw new OperationError(
				operation,
				answer.refuse,
				answer.message ?? `${operation} was refused.`,
			);
		}

		// A Job's declared output describes its eventual result; asking for it
		// answers the id envelope, and that is what is checked here.
		const schema = declaration.kind === 'job' ? JOB_ASK_SCHEMA : declaration.output_schema;
		checkAgainstSchema(operation, schema, answer, '');
		return answer;
	};

	return {
		client: createClient(transport),
		calls,
		answer(operation, answer) {
			programmed[operation] = answer;
		},
	};
}
