import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import Story from './Story.svelte';
import { story } from './showing.svelte';
import { m } from '$lib/paraglide/messages';
import { getLocale, locales } from '$lib/paraglide/runtime';

/** Which stage is on screen, as the story says it to a screen reader. */
function stageShown(): string {
	return screen.getByText(/^· \d+ \S+ 11$/).textContent ?? '';
}

describe('the story (#158)', () => {
	const user = userEvent.setup();

	afterEach(() => {
		localStorage.clear();
		document.documentElement.removeAttribute('lang');
		vi.useRealTimers();
		vi.unstubAllGlobals();
	});

	it('goes on one stage for a tap, and back one for a tap on the left edge', async () => {
		render(Story, { props: { ending: 'invite' } });

		expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
			'Every recipe you cook, in one place.',
		);
		expect(stageShown()).toBe('· 1 of 11');

		await user.click(screen.getByRole('button', { name: 'Next' }));
		expect(stageShown()).toBe('· 2 of 11');
		expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('sound familiar?');

		await user.click(screen.getByRole('button', { name: 'Next' }));
		await user.click(screen.getByRole('button', { name: 'Back' }));
		expect(stageShown()).toBe('· 2 of 11');

		// Past either end, a tap does nothing.
		await user.click(screen.getByRole('button', { name: 'Back' }));
		await user.click(screen.getByRole('button', { name: 'Back' }));
		expect(stageShown()).toBe('· 1 of 11');
	});

	it('skips to the last stage, where the Invite ends on the way to the form', async () => {
		const onCreate = vi.fn();
		render(Story, { props: { ending: 'invite', onCreate } });

		await user.click(screen.getByRole('button', { name: 'Skip' }));

		expect(stageShown()).toBe('· 11 of 11');
		expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
			'Your Cookbook is waiting.',
		);
		expect(screen.queryByRole('button', { name: 'Skip' })).not.toBeInTheDocument();
		await user.click(screen.getByRole('button', { name: 'Create my account' }));
		expect(onCreate).toHaveBeenCalledOnce();
	});

	it('ends at /about without the button, asking for an Invite, with a way out', async () => {
		const onClose = vi.fn();
		render(Story, { props: { ending: 'about', onClose } });

		await user.click(screen.getByRole('button', { name: 'Skip' }));

		expect(screen.queryByRole('button', { name: 'Create my account' })).not.toBeInTheDocument();
		expect(
			screen.getByText('Want in? Ask whoever sent you this link for an invite.'),
		).toBeInTheDocument();
		expect(screen.queryByText(/This invite creates your account/)).not.toBeInTheDocument();
		await user.click(screen.getByRole('button', { name: 'Close' }));
		expect(onClose).toHaveBeenCalledOnce();
	});

	it('never goes on by itself, and only hints after a while', async () => {
		vi.useFakeTimers();
		render(Story, { props: { ending: 'invite' } });
		const hint = screen.getByText('Tap to continue');
		expect(hint).not.toHaveClass('show');

		await vi.advanceTimersByTimeAsync(60_000);

		expect(stageShown()).toBe('· 1 of 11');
		expect(hint).toHaveClass('show');
	});

	it('changes its words in place, on the same stage, and keeps the choice', async () => {
		render(Story, { props: { ending: 'invite' } });
		await user.click(screen.getByRole('button', { name: 'Next' }));
		await user.click(screen.getByRole('button', { name: 'Next' }));

		await user.click(screen.getByRole('button', { name: 'FR' }));

		expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
			'Maintenant, elles sont toutes dans votre Carnet.',
		);
		expect(stageShown()).toBe('· 3 sur 11');
		expect(screen.getByRole('button', { name: 'FR' })).toHaveAttribute('aria-pressed', 'true');
		// Kept the way the app keeps it, so the form and the app open in French.
		expect(localStorage.getItem('PARAGLIDE_LOCALE')).toBe('fr');
		expect(getLocale()).toBe('fr');
		expect(document.documentElement.lang).toBe('fr');
	});

	// Each stamp is an idea's own word, and the app already has a message
	// naming that idea. Reading the stamp against that message in every
	// language means renaming the idea in the app cannot leave the story
	// saying the old word. Contained in it, not equal to it, on purpose: no
	// message in the app holds the bare word, and the stamp is that word.
	it.each(locales)('stamps each idea with the word the app uses for it (%s)', async (locale) => {
		render(Story, { props: { ending: 'invite' } });
		if (locale !== 'en')
			await user.click(screen.getByRole('button', { name: locale.toUpperCase() }));
		const L = { locale };
		const stamps: [number, string][] = [
			[3, m.settings_your_cookbook({}, L)],
			[4, m.recipe_leave_in_diary({}, L)],
			[6, m.recipe_keep_as_version({}, L)],
			[7, m.kitchen_create_title({}, L)],
		];

		let at = 1;
		for (const [stage, appSays] of stamps) {
			for (; at < stage; at++)
				await user.click(screen.getByRole('button', { name: m.story_next({}, L) }));
			const stamp = document.querySelector('.stamp')?.textContent?.trim() ?? '';
			expect(stamp).not.toBe('');
			expect(appSays).toContain(stamp);
		}
	});

	it('draws every stage finished and still when reduced motion is asked for', () => {
		vi.stubGlobal('matchMedia', (query: string) => ({
			matches: query === '(prefers-reduced-motion: reduce)',
			media: query,
		}));
		const { container } = render(Story, { props: { ending: 'invite' } });

		expect(container.querySelector('.story')).toHaveClass('still');
	});

	it('says it is showing, so the layout leaves its header and tab bar out', () => {
		const { unmount } = render(Story, { props: { ending: 'about' } });
		expect(story.showing).toBe(true);
		unmount();
		expect(story.showing).toBe(false);
	});
});
