<!--
	Bringing a Crouton library in (#69), from Settings.

	The export is a zip of 86 files and 114 MB, so it is sent as its own bytes
	to `POST /api/uploads` first, and `import_crouton` is then asked with the id
	that answers (ADR 0001). Asking starts a Job and the screen moves straight
	to its Report, which reads the Job while it runs and keeps its result once
	it ends. A Crouton import already made is one link away, so the Report is
	never lost with the screen that started it.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { useUpload } from '$lib/api/upload';
	import Section from '$lib/shell/Section.svelte';

	const kamosu = useKamosu();
	const upload = useUpload();

	let sending = $state<string | undefined>(undefined);
	let failed = $state<string | undefined>(undefined);
	let lastJob = $state<string | undefined>(undefined);

	$effect(() => {
		let current = true;
		kamosu
			.listJobs()
			.then((answer) => {
				const last = answer.jobs.find((job) => job.operation === 'import_crouton');
				if (current) lastJob = last?.id;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	const megabytes = (bytes: number) => `${Math.max(1, Math.round(bytes / 1_000_000))} MB`;

	async function choose(event: Event & { currentTarget: HTMLInputElement }) {
		const file = event.currentTarget.files?.[0];
		event.currentTarget.value = '';
		if (!file) return;
		failed = undefined;
		sending = megabytes(file.size);
		try {
			const uploadId = await upload(file);
			const asked = await kamosu.importCrouton({ upload_id: uploadId });
			await goto(`/imports/${asked.job_id}`);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			sending = undefined;
		}
	}
</script>

<Section heading={m.settings_import()}>
	<p class="mb-3 text-read text-ink-2">{m.import_crouton_blurb()}</p>
	<label
		class="inline-flex min-h-12 cursor-pointer items-center rounded-sm bg-accent px-4 font-semibold text-on-accent has-disabled:opacity-60"
	>
		{sending ? m.import_crouton_sending({ size: sending }) : m.import_crouton_choose()}
		<input
			type="file"
			accept=".zip,.crumb,application/zip"
			class="sr-only"
			disabled={sending !== undefined}
			onchange={choose}
		/>
	</label>
	{#if failed}
		<p class="mt-2 border-l-3 border-support bg-card px-3 py-2 text-read" role="alert">{failed}</p>
	{/if}
	{#if lastJob}
		<p class="mt-3 text-read">
			<a href="/imports/{lastJob}" class="text-accent underline">{m.import_crouton_last()}</a>
		</p>
	{/if}
</Section>
