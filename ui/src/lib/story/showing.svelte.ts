/**
 * Whether the story (#158) is on screen. It fills the whole window with its own
 * corners, so the layout leaves the header and the tab bar out while it shows,
 * the way it does for the cooking screen: drawn underneath, they would still be
 * read out and tabbed through behind a page that covers them.
 */
export const story = $state({ showing: false });
