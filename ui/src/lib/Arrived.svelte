<!--
	What bringing a recipe file in just said, above the recipe it brought (#93).

	Drawn with `Notice`, which is the house voice for Kamosu saying a thing once
	at the top of the page and the person putting it away (#76). A recipe file
	does not end on the Import Report the way a Crouton library does — one file
	is one recipe, so the recipe opens and this says the line (option C,
	chosen 21 September 2026). *How it went* leads to the Report for what a
	line cannot hold.

	It shows only above the recipe it is about, and leaving that recipe is what
	forgets it: a note about something that has just happened must not still be
	sitting there three screens later.

	Four outcomes, four sentences, because they are four different facts. A
	recipe that arrived, one your Kitchen already held unchanged, and one the
	file carried further are all successes and read as such. A recipe whose
	history could not be read is the fourth, and its sentence is the Core's own
	(#67) rather than one written again here — it already says exactly what was
	kept and what was lost.

	A Share Link imported from its own page (#170) ends here too, in lines
	that name no file, and says whose newer Versions came when it brought some.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import Notice from '$lib/offline/Notice.svelte';
	import { forgetArrival, theArrival } from '$lib/arrival.svelte';

	interface Props {
		/**
		 * Where the reader is. Handed down rather than read from `$app/state`
		 * here, so this stays a component a screen test can render: the route
		 * files are what know about navigation in this app.
		 */
		pathname: string;
	}

	let { pathname }: Props = $props();

	const arrival = $derived(theArrival());
	const here = $derived(arrival !== undefined && pathname === `/recipes/${arrival.branchId}`);

	/**
	 * Said once: reaching the recipe arms the forgetting, and leaving it does
	 * the forgetting.
	 *
	 * The teardown is the whole of it, and it has to be, because the arrival is
	 * noted *before* the navigation that shows it — so "not on that page" means
	 * two opposite things a moment apart, on the way there and gone from there.
	 * An effect that only registers a teardown while `here` cannot confuse the
	 * two: nothing is armed on the way there, and the teardown runs exactly
	 * when `here` stops being true.
	 */
	$effect(() => {
		if (here) return forgetArrival;
	});

	const title = $derived.by(() => {
		if (!arrival) return '';
		if (arrival.how === 'kept') return m.arrived_kept();
		if (arrival.status === 'unchanged') return m.arrived_unchanged();
		if (arrival.status === 'extended') return m.arrived_extended();
		return arrival.from === 'link' ? m.arrived_link_created() : m.arrived_created();
	});

	/**
	 * Each ending's line, for a file somebody chose and for a Share Link
	 * imported from its page (#170), which is no file and names none.
	 */
	const LINES = {
		file: {
			created: m.arrived_created_line,
			extended: m.arrived_extended_line,
			unchanged: m.arrived_unchanged_line,
		},
		link: {
			created: m.arrived_link_created_line,
			extended: m.arrived_link_extended_line,
			unchanged: m.arrived_link_unchanged_line,
		},
	};

	const line = $derived.by(() => {
		if (!arrival) return '';
		// A damaged Bundle's own reason, which already names what was kept and
		// what was lost. Rewriting it here would say less, and differently.
		if (arrival.how === 'kept') return arrival.reason ?? '';
		const { title, writer, added } = arrival;
		// Newer Versions from a Share Link are somebody's, and counted: the
		// confirm screen said how many, and this says they came.
		if (arrival.status === 'extended' && writer && added) {
			return added === 1
				? m.arrived_link_extended_by_one({ name: writer, title })
				: m.arrived_link_extended_by({ name: writer, title, count: added });
		}
		return LINES[arrival.from ?? 'file'][arrival.status ?? 'created']({ title });
	});

	/** The Passengers, said only where there are any (ADR 0008). */
	const carried = $derived.by(() => {
		const passengers = arrival?.passengers ?? [];
		if (passengers.length === 0) return undefined;
		const recipes = passengers.join(', ');
		return passengers.length === 1
			? m.arrived_passengers_one({ recipes })
			: m.arrived_passengers({ recipes });
	});
</script>

{#if arrival && here}
	<Notice
		{title}
		tone="plain"
		actions={[
			{
				label: m.arrived_report(),
				act: () => {
					const jobId = arrival.jobId;
					forgetArrival();
					goto(`/imports/${jobId}`);
				},
			},
			{ label: m.arrived_put_away(), act: forgetArrival },
		]}
	>
		<p>{line}</p>
		{#if carried}
			<p class="mt-1 text-support-2">{carried}</p>
		{/if}
	</Notice>
{/if}
