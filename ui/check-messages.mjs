// The guard that keeps the interface's copy short (issue #141).
//
// Every phrase in messages/ was trimmed once, and approved phrase by phrase.
// This fails `just check` if the paragraphs come back: an em dash in any
// language, or an English phrase longer than LIMIT. Only English is measured,
// because French and Spanish follow the approved English and run longer by
// nature.
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const messages = path.join(path.dirname(fileURLToPath(import.meta.url)), 'messages');

// Only this file is measured for length.
const MEASURED = 'en.json';

// The longest ordinary phrase approved in #141 was 163 characters.
const LIMIT = 170;

// Real warnings that need more than LIMIT to say what a person must know,
// and story copy Aurélien approved sentence by sentence (#158). Keep this
// list short: a key belongs here only if cutting it would drop a fact
// someone needs to act, or words he chose.
const ALLOWED_LONGER = {
	// Who can slow a sign-in, how long the wait is, and the way out.
	settings_name_held_off: 250,
	// The story's everyday card (#158): four features in one line, approved
	// sentence by sentence by Aurélien on 25 September 2026.
	story_10_said: 190,
	// A scanned PDF's refusal (#176): why, and the two ways in that still
	// work. Aurélien chose this wording word for word on 27 September 2026.
	plus_pdf_no_text: 175,
};

const failures = [];

for (const file of readdirSync(messages).filter((f) => f.endsWith('.json'))) {
	const phrases = JSON.parse(readFileSync(path.join(messages, file), 'utf8'));
	for (const [key, text] of Object.entries(phrases)) {
		if (key === '$schema') continue;
		if (text.includes('—')) failures.push(`${file} ${key}: contains an em dash`);
		if (file !== MEASURED) continue;
		const limit = ALLOWED_LONGER[key] ?? LIMIT;
		if (text.length > limit) {
			failures.push(`${file} ${key}: ${text.length} characters, over the limit of ${limit}`);
		}
	}
	if (file === MEASURED) {
		for (const key of Object.keys(ALLOWED_LONGER)) {
			if (!(key in phrases))
				failures.push(`check-messages.mjs: allowed key ${key} no longer exists`);
		}
	}
}

if (failures.length) {
	console.error(
		`Interface copy is too long or uses em dashes (issue #141):\n  ${failures.join('\n  ')}`,
	);
	process.exit(1);
}
