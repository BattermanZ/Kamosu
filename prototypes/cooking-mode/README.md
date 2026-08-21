# Kamosu cooking-mode prototype

Throwaway UI prototype for the Wayfinder ticket **What Kamosu looks and feels
like, especially while cooking**.

It compares three directions for the same recipe and live cooking flow:

- **A — Countertop:** calm recipe overview, then one large instruction at a time.
- **B — Mise en place:** dense working view with the whole method kept visible.
- **C — Recipe cards:** tactile, compact layers that keep ingredients beside the
  current step.

Run it from the repository root:

```sh
python3 -m http.server 4173 --directory prototypes/cooking-mode
```

Then open <http://localhost:4173>. Use the bottom switcher or the left and right
arrow keys to change direction. The URL keeps the choice in `?variant=A`,
`?variant=B`, or `?variant=C` so a particular direction can be shared.

This is intentionally dependency-free and disposable. It is not production
frontend code.
