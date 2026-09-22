/**
 * Where a screen asked to go, for every test file alike (#122).
 *
 * `$app/navigation` is mocked once, in `setup.ts`, with this one `goto`. Each
 * spec used to mock it with its own spy, which works only while every file gets
 * a fresh module registry: with isolation off, a component is imported once per
 * worker and keeps the `goto` of whichever file imported it first, so a later
 * file's spy was never called. One spy, cleared after every test, has no first.
 */

import { vi } from 'vitest';

export const went = vi.fn();
