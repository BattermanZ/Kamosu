/**
 * The day a cooking happened, as the diary writes it — `20 August` — and
 * everywhere a cooking is named by when it was, so a picture on the recipe
 * page is dated exactly as it is in the diary (#110).
 */
import { aDay } from '$lib/dates';

export const cookingDay = (when: string): string => aDay(when);
