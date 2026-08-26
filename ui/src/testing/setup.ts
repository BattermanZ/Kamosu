/**
 * What every test runs inside.
 *
 * Screen tests are the only thing standing between a bad-day component and the
 * kitchen (ADR 0012), so the environment they run in is part of the guarantee:
 * a real DOM, the browser build of Svelte, and nothing stubbed that a screen
 * could rely on by accident.
 */

import '@testing-library/jest-dom/vitest';
import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/svelte';

afterEach(() => cleanup());
