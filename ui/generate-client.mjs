/**
 * Generate the typed client from the Catalogue.
 *
 * Kamosu declares every Operation exactly once, in `src/catalogue.rs`, and both
 * Doors are built by walking that list (ADR 0001). This script extends the same
 * idea to the browser: `kamosu catalogue` prints those declarations as JSON, and
 * what comes out here is TypeScript the interface calls through. The frontend
 * and the Core therefore cannot disagree about an Operation's shape — a renamed
 * field is a failed build, not a blank screen in a kitchen (ADR 0012).
 *
 * The same generated file is what the screen-seam harness reads: a stand-in
 * derived from these declarations answers exactly what the real Operations
 * answer, which is only possible because both come from here.
 *
 * Usage:  node ./generate-client.mjs <catalogue.json>   (or the JSON on stdin)
 *         OUT=path/to/catalogue.ts node ./generate-client.mjs …
 *
 * Never edit the output. Change `src/catalogue.rs` and run `just client`;
 * `just check` fails if the committed copy has drifted.
 */

import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const DEFAULT_OUT = 'src/lib/api/catalogue.ts';

// ── Reading the declarations ─────────────────────────────────────────────────

function readDeclarations() {
	const [file] = process.argv.slice(2);
	const raw = file ? readFileSync(file, 'utf8') : readFileSync(0, 'utf8');
	const declarations = JSON.parse(raw);
	if (!Array.isArray(declarations) || declarations.length === 0) {
		throw new Error('expected a non-empty array of Operation declarations');
	}
	return declarations;
}

// ── JSON Schema → TypeScript ─────────────────────────────────────────────────

/**
 * The subset of JSON Schema the Catalogue actually declares. Anything outside
 * it throws rather than degrading to `unknown`: a shape this cannot render is a
 * shape the interface would silently stop checking.
 */
function typeOf(schema, indent) {
	if (schema === undefined) throw new Error('missing schema');

	// `{}` — declared by `get_job` for a result whose shape depends on which
	// Operation ran. Genuinely unknown, and typed as such.
	if (Object.keys(schema).length === 0) return 'unknown';

	if (schema.enum) {
		return schema.enum.map((value) => JSON.stringify(value)).join(' | ');
	}

	const types = Array.isArray(schema.type) ? schema.type : [schema.type];
	const rendered = types.map((type) => {
		switch (type) {
			case 'string':
				return 'string';
			case 'boolean':
				return 'boolean';
			case 'integer':
			case 'number':
				return 'number';
			case 'null':
				return 'null';
			case 'array': {
				// A union has to be parenthesised before `[]` binds to it, or an
				// array of nullable Readings renders as `Reading | null[]` — which
				// is a union with an array of nulls, and type-checks nothing.
				const item = typeOf(schema.items, indent);
				const union =
					(Array.isArray(schema.items?.type) && schema.items.type.length > 1) ||
					(Array.isArray(schema.items?.enum) && schema.items.enum.length > 1);
				return union ? `(${item})[]` : `${item}[]`;
			}
			case 'object':
				return objectType(schema, indent);
			default:
				throw new Error(`unsupported schema type: ${JSON.stringify(schema.type)}`);
		}
	});
	return rendered.join(' | ');
}

function objectType(schema, indent) {
	const properties = schema.properties ?? {};
	const names = Object.keys(properties);
	if (names.length === 0) {
		// An Operation that takes nothing. `Record<string, never>` is the empty
		// envelope: passing a field is a type error, as the Core would refuse it.
		return 'Record<string, never>';
	}
	const required = new Set(schema.required ?? []);
	const inner = indent + '\t';
	const fields = names.map((name) => {
		const optional = required.has(name) ? '' : '?';
		return `${inner}${propertyKey(name)}${optional}: ${typeOf(properties[name], inner)};`;
	});
	return `{\n${fields.join('\n')}\n${indent}}`;
}

const SAFE_KEY = /^[A-Za-z_$][A-Za-z0-9_$]*$/;
const propertyKey = (name) => (SAFE_KEY.test(name) ? name : JSON.stringify(name));

// ── Names ────────────────────────────────────────────────────────────────────

const pascal = (name) =>
	name
		.split('_')
		.filter(Boolean)
		.map((word) => word[0].toUpperCase() + word.slice(1))
		.join('');

const camel = (name) => {
	const p = pascal(name);
	return p[0].toLowerCase() + p.slice(1);
};

// ── The generated file ───────────────────────────────────────────────────────

function generate(declarations) {
	const out = [];
	out.push('// Generated from the Catalogue by `just client`. Do not edit.');
	out.push('//');
	out.push('// Every Operation Kamosu offers is declared once in src/catalogue.rs and both');
	out.push('// Doors are built by walking that list (ADR 0001). These types are the third');
	out.push('// thing built from it, so the interface cannot ask for a shape the Core does');
	out.push('// not serve. Change the Catalogue and re-run `just client`; `just check`');
	out.push('// fails if what is committed here has drifted.');
	out.push('');
	if (declarations.some((op) => op.kind === 'job')) {
		out.push("import type { JobAsk } from './job';");
		out.push('');
	}

	for (const op of declarations) {
		const name = pascal(op.name);
		out.push(`/** ${op.summary.replace(/\s+/g, ' ')} */`);
		out.push(`export type ${name}Input = ${typeOf(op.input_schema, '')};`);
		out.push(
			op.kind === 'job'
				? `/** What ${op.name} eventually produces, read back through \`get_job\`. */`
				: `/** What ${op.name} answers. */`,
		);
		out.push(`export type ${name}Output = ${typeOf(op.output_schema, '')};`);
		out.push('');
	}

	out.push('/**');
	out.push(' * Every Operation, by the name both Doors use for it. An Immediate');
	out.push(' * Operation answers its output; a Job answers a job id at once and its');
	out.push(' * output arrives later, through `get_job` (ADR 0032).');
	out.push(' */');
	out.push('export interface Operations {');
	for (const op of declarations) {
		const name = pascal(op.name);
		out.push(`\t${propertyKey(op.name)}: {`);
		out.push(`\t\tinput: ${name}Input;`);
		out.push(`\t\toutput: ${name}Output;`);
		out.push(`\t\tkind: '${op.kind}';`);
		out.push(`\t\tpermission: '${op.permission}';`);
		out.push('\t};');
	}
	out.push('}');
	out.push('');
	out.push('export type OperationName = keyof Operations;');
	out.push('');
	out.push('/** What asking for an Operation answers: a Job answers an id, nothing else does. */');
	if (declarations.some((op) => op.kind === 'job')) {
		out.push('export type Answer<N extends OperationName> =');
		out.push("\tOperations[N]['kind'] extends 'job' ? JobAsk : Operations[N]['output'];");
	} else {
		// No Job is declared in this build, so every Operation answers its output.
		out.push("export type Answer<N extends OperationName> = Operations[N]['output'];");
	}
	out.push('');

	out.push('/**');
	out.push(' * The declarations themselves, verbatim. The screen-seam harness reads these');
	out.push(' * to answer as the real Operations do; nothing else needs them at runtime.');
	out.push(' */');
	out.push('export const CATALOGUE = [');
	for (const op of declarations) {
		const entry = JSON.stringify(
			{
				name: op.name,
				summary: op.summary,
				permission: op.permission,
				kind: op.kind,
				input_schema: op.input_schema,
				output_schema: op.output_schema,
			},
			null,
			'\t',
		);
		out.push(entry.replace(/^/gm, '\t') + ',');
	}
	out.push('] as const;');
	out.push('');

	out.push('/** The camelCase method the client exposes for each Operation. */');
	out.push('export const METHOD_NAMES = {');
	for (const op of declarations) {
		out.push(`\t${propertyKey(op.name)}: '${camel(op.name)}',`);
	}
	out.push('} as const;');
	out.push('');

	out.push('/** The typed client: one method per Operation, named as the Catalogue names it. */');
	out.push('export interface KamosuClient {');
	for (const op of declarations) {
		const name = pascal(op.name);
		const takesNothing = Object.keys(op.input_schema.properties ?? {}).length === 0;
		const argument = takesNothing ? `input?: ${name}Input` : `input: ${name}Input`;
		out.push(`\t/** ${op.summary.replace(/\s+/g, ' ')} */`);
		out.push(`\t${camel(op.name)}(${argument}): Promise<Answer<'${op.name}'>>;`);
	}
	out.push('}');
	out.push('');

	return out.join('\n');
}

// ── Run ──────────────────────────────────────────────────────────────────────

const target = resolve(process.env.OUT ?? DEFAULT_OUT);
mkdirSync(dirname(target), { recursive: true });
writeFileSync(target, generate(readDeclarations()));
