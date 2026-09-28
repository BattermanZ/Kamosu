# Changelog

## v0.1.0 - unreleased

The first release. Kamosu has been cooked from in one household since August
2026. This is the first time anyone else can run it: it is now
public, licensed under the AGPL-3.0, and published as a ready-made image,
`battermanz/kamosu`, for both amd64 and arm64.

Nothing is listed here as added or fixed, since there was no earlier release to
compare with. What 0.1.0 contains:

- **Recipes that keep their history.** Every save is a new Version and none is
  ever lost. Variations and Translations are Branches of the same recipe, and
  the History screen shows where two of them parted and every line that
  differs.
- **Cooking mode.** Step by step, with the amounts each step uses, ticked
  ingredients, timers read out of the step text, and the recipe scaled to what
  you are making. It works offline, and a cooking started on the phone carries
  on on a tablet.
- **Attempts.** A diary of each cooking, with a rating, photos, notes and what
  was actually done, any of which can be promoted into the recipe.
- **Ingredient lines kept as written**, read into amount, unit and food for
  scaling, metric and US conversions, oven temperatures and the shopping list.
- **A shopping list** worked out from the recipes you picked, merging the same
  food across recipes, plus loose items you type.
- **Imports** from a web page's schema.org data, a pasted recipe, a recipe PDF,
  a Crouton export, and another Kamosu's recipe files and Share Links.
- **Sharing.** A Share Link per recipe for anyone who holds it, a printable PDF
  Sheet, and a recipe file carrying the full history.
- **People.** Invites instead of signup, a Cookbook per person that two people
  can join, Kitchens for cooking from each other's Cookbooks, and Operators who
  run the instance without being able to read anyone's recipes through the app.
- **Agents.** Every operation is offered over MCP at `/mcp` exactly as the web
  app gets it, with per-person Access Keys that can be read-only.
- **Search** by words from the start, and by meaning once an Operator turns on
  the optional EmbeddingGemma model, which Kamosu does not ship.
- **Backups** taken automatically, three kept (a day, a week and a month back),
  and a Snapshot before every upgrade migrates the database.
- **English, French and Spanish**, for the interface and for recipes.

The things Kamosu does not defend against are listed in
[SECURITY.md](SECURITY.md).
