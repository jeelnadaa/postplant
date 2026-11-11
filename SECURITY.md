# Security

Please report security problems privately through GitHub's **Report a vulnerability** button on the repository's Security tab, not in a public issue.

Especially interesting: anything that could leak a user's API key (it should only ever reach Windows Credential Manager and `api.henrikdev.xyz`), import files that escape their checks (`docs/export-format.md`), or ways a crafted backup or JSON export could write outside the app's data folder.

Only the latest release gets fixes.
