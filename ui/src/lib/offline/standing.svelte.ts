/**
 * The recipe the page is showing, if it is showing one (#76).
 *
 * The offline card says something different on a recipe the Person's Kitchen
 * holds — fully on the phone — and on one they only opened once, which is a
 * copy from that day. The card sits in the layout and the recipe in the page,
 * so the recipe screen says here where it is standing.
 */

export const standing = $state<{ branchId?: string; keptAt?: Date }>({});
