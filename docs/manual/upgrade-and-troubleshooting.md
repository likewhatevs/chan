# Upgrade And Troubleshooting

Use the install page for fresh desktop downloads. Use the CLI upgrade path for the standalone `chan` binary.

## Upgrade the CLI

The standalone binary reads CLI release metadata from `https://chan.app/dl/cli/latest.json`, selects the asset for the current OS and architecture, and verifies the download against the SHA256 value in that metadata.

```sh
chan upgrade
chan upgrade --version X.Y.Z
```

`--version` takes a bare version and reads `https://chan.app/dl/cli/v<version>.json`. Public release tags use `v<version>`.

## Server URL has expired

Each standalone server launch prints a fresh bearer-token URL. Restart the server or copy the latest URL from the terminal that launched it.

## Workspace does not update

Check that the file is inside the workspace root and is a regular file. Chan refuses special files and keeps workspace access sandboxed under the workspace root.

## Install script fails

The shell installer supports the active standalone CLI release targets. For desktop installs, use the [install page](https://chan.app/install/). If release metadata cannot be fetched, the install page links to GitHub Releases instead of guessing asset URLs.
