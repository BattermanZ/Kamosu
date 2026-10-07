/**
 * What a browser says of a drag (#204). jsdom has no drag of its own, so a
 * test hands its events this: the files carried, or a link, or words.
 */
export function carrying(what: { files?: File[]; link?: string; text?: string }): DataTransfer {
	const files = what.files ?? [];
	const data: Record<string, string> = {};
	if (what.link) data['text/uri-list'] = what.link;
	if (what.link ?? what.text) data['text/plain'] = (what.link ?? what.text)!;
	return {
		types: [...(files.length ? ['Files'] : []), ...Object.keys(data)],
		// Before the drop a browser gives each file's type and keeps its name.
		items: files.map((file) => ({ kind: 'file', type: file.type })),
		files,
		getData: (type: string) => data[type] ?? '',
		dropEffect: 'copy',
	} as unknown as DataTransfer;
}
