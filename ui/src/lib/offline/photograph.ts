/**
 * Putting a picture on a cooking (#77, ADR 0005), the one way both the
 * cooking screen and the diary do it.
 *
 * The picture is kept on the phone first and named there, then added to the
 * cooking beside whatever it already holds (`add_photographs`), so a picture
 * taken with no network is as much a part of the cooking as one taken at home.
 * When it reaches the server it is remade at the door and named by its bytes
 * (ADR 0017); until then the outbox shows it from the phone.
 */

import type { KamosuClient, StartAttemptOutput } from '$lib/api/catalogue';
import type { Keeping } from './outbox';

export async function photographCooking(
	kamosu: KamosuClient,
	keeping: Keeping,
	attemptId: string,
	picture: Blob,
): Promise<StartAttemptOutput['photographs']> {
	const name = await keeping.keepPhotograph(picture);
	const edited = await kamosu.editAttempt({ attempt_id: attemptId, add_photographs: [name] });
	return edited.photographs;
}

/** The one file a picker handed over, and the picker emptied for the next. */
export function pickedFile(event: Event): File | undefined {
	const input = event.currentTarget as HTMLInputElement;
	const file = input.files?.[0];
	input.value = '';
	return file;
}
