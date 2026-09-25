import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import About from './+page.svelte';
import { renderScreen } from '../../testing/render';
import { went } from '../../testing/navigation';

describe('/about (#158)', () => {
	const user = userEvent.setup();

	// No layout sends a stranger to the sign-in form, and this page asks Kamosu
	// nothing: somebody with no account and no Invite reads the whole story.
	it('tells a signed-out visitor the story without asking Kamosu anything', async () => {
		const { kamosu } = renderScreen(About);

		expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
			'Every recipe you cook, in one place.',
		);
		expect(kamosu.calls).toHaveLength(0);

		await user.click(screen.getByRole('button', { name: 'Skip' }));
		await user.click(screen.getByRole('button', { name: 'Close' }));
		expect(went).toHaveBeenCalledWith('/');
	});
});
