# Upgrade And Troubleshooting

Use the install page for fresh desktop downloads. Use the CLI upgrade path
for the standalone `chan` binary.

## Upgrade the CLI

The standalone binary checks the latest GitHub Release at
`github.com/fiorix/chan` and verifies downloads against the published
`SHA256SUMS` file.

```sh
chan upgrade
chan upgrade --version 0.14.0
```

`--version` takes a bare version. Release tags are named `chan-v<version>` by
the release workflow.

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
