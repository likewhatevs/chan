// Desktop-native download progress store.
//
// The browser's Download button hands the file to the browser's own
// download manager (progress + Downloads folder + reveal, all native).
// chan-desktop's webview has no such manager, so `runDesktopDownload`
// (api/desktop.ts) fetches the file over the loopback connection with
// XHR progress and saves it through the `save_file_to_downloads` Tauri
// command. This store carries the in-app indicator state the inspector
// renders to mimic the browser's progress UI.

/// `progress` is 0..1 while a content-length is known, or null for an
/// indeterminate transfer (server sent no Content-Length). `cancel`
/// aborts the in-flight fetch; null once the transfer is past the point
/// where cancelling is meaningful (writing to disk). `savedPath` is set
/// on success so the indicator can offer a "reveal in Finder" action.
export interface DownloadTransfer {
  filename: string;
  progress: number | null;
  cancel: (() => void) | null;
  savedPath: string | null;
  error: string | null;
}

export const downloadTransfer = $state<{ value: DownloadTransfer | null }>({
  value: null,
});

/// True when a desktop download is currently in flight. The inspector's
/// Download button reads this to disable itself / show a spinner.
export function downloadTransferActive(): boolean {
  return downloadTransfer.value !== null && downloadTransfer.value.savedPath === null
    && downloadTransfer.value.error === null;
}

export function beginDownloadTransfer(
  filename: string,
  cancel: (() => void) | null,
): void {
  downloadTransfer.value = {
    filename,
    progress: null,
    cancel,
    savedPath: null,
    error: null,
  };
}

export function setDownloadProgress(progress: number | null): void {
  if (!downloadTransfer.value) return;
  downloadTransfer.value.progress = progress;
}

export function finishDownloadTransfer(savedPath: string): void {
  if (!downloadTransfer.value) return;
  downloadTransfer.value.progress = 1;
  downloadTransfer.value.cancel = null;
  downloadTransfer.value.savedPath = savedPath;
}

export function failDownloadTransfer(error: string): void {
  if (!downloadTransfer.value) return;
  downloadTransfer.value.cancel = null;
  downloadTransfer.value.error = error;
}

/// Clear the indicator. The inspector calls this when the user
/// dismisses the toast or after a short success auto-dismiss.
export function clearDownloadTransfer(): void {
  downloadTransfer.value = null;
}
