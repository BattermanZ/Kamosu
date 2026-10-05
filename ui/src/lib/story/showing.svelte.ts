/**
 * Whether the story (#158) is on screen. It fills the whole window with its own
 * corners, so the shell draws no navigation while it shows, the way it draws
 * none for the cooking screen: a header, tab bar or sidebar drawn underneath
 * would still be read out and tabbed through behind a page that covers it.
 */
export const story = $state({ showing: false });
