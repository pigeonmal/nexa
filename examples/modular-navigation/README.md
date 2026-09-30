# Modular navigation example

This project keeps the app entry point, navigable screens, and tab content in
separate `.nx` files. `App.nx` imports each file; imported declarations are
available to the normal typed screen and component calls.

From this directory, run:

```sh
nexa check
nexa dev
```

- `screens/Home.nx` defines the navigation root and connects imported tab components.
- `screens/Details.nx` defines a second screen with local state and access to app state.
- `tabs/HomeTab.nx` and `tabs/ProfileTab.nx` define reusable tab content.

During `nexa dev`, adding an imported `.nx` file and referencing it from the
entry point is picked up by hot reload; no native host rebuild is needed for
these source-only changes.
