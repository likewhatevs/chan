# Upgrade And Troubleshooting

Use the install page for fresh desktop downloads. Use the CLI upgrade path
for the standalone `chan` binary.

## Upgrade the CLI

The standalone binary reads CLI release metadata from
`https://chan.app/dl/cli/latest.json`, selects the asset for the current
OS and architecture, and verifies the download against the SHA256 value in
that metadata.

```sh
chan upgrade
chan upgrade --version 0.14.0
```

`--version` takes a bare version and reads
`https://chan.app/dl/cli/v<version>.json`. Public release tags use
`v<version>`.

## Server URL has expired

Each standalone server launch prints a fresh bearer-token URL. Restart the
server or copy the latest URL from the terminal that launched it.

## Drive does not update

Check that the file is inside the drive root and is a regular file. Chan
refuses special files and keeps drive access sandboxed under the drive root.

## Install script fails

The shell installer supports only the active standalone CLI release targets.
For desktop installs, use the DMG, AppImage, or deb links on the install
page.
