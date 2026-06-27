# Release signing procedures

The detailed release-signing procedures and helper scripts that used to live here are maintained in the team's private `dev/` tree and are intentionally not distributed in the public repository:

- macOS Developer ID signing and notarization: the full per-secret table, certificate generation, and rotation steps.
- Windows Authenticode signing: the SSL.com eSigner cloud-HSM path.
- DNS cutover notes.
- The `populate-apple-secrets.sh` and `setup-notarytool-keychain.sh` helper scripts.

CI references these procedures by name when a required signing secret is missing; the material itself is held privately by the maintainers.
