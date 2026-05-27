(function () {
  const key = "chan-site-mode";
  const button = document.getElementById("theme-toggle");
  const saved = window.localStorage.getItem(key);
  if (saved === "dark" || saved === "light") {
    document.body.dataset.mode = saved;
  } else if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
    document.body.dataset.mode = "dark";
  }
  if (button) {
    button.addEventListener("click", () => {
      const next = document.body.dataset.mode === "dark" ? "light" : "dark";
      document.body.dataset.mode = next;
      window.localStorage.setItem(key, next);
    });
  }
})();

(function () {
  document.querySelectorAll("[data-copy-block]").forEach((block) => {
    const button = block.querySelector("button");
    const value = block.querySelector("[data-copy-value]");
    if (!button || !value) return;
    button.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(value.textContent || "");
        const previous = button.textContent;
        button.textContent = "Copied";
        block.classList.add("copied");
        window.setTimeout(() => {
          button.textContent = previous;
          block.classList.remove("copied");
        }, 1400);
      } catch (_err) {
      }
    });
  });
})();

(function () {
  const metadataUrl = "/dl/releases.json";
  const fallbackUrl = "https://github.com/fiorix/chan/releases";
  const links = Array.from(document.querySelectorAll("[data-release-download]"));
  if (links.length === 0) return;

  const applyFallback = () => {
    links.forEach((link) => {
      link.href = fallbackUrl;
      link.dataset.releaseState = "fallback";
    });
  };

  fetch(metadataUrl, { cache: "no-store" })
    .then((response) => {
      if (!response.ok) throw new Error(`metadata HTTP ${response.status}`);
      return response.json();
    })
    .then((metadata) => {
      const release = latestRelease(metadata);
      const downloads = new Map(
        (release.downloads || [])
          .filter((download) => isSafeDownload(download))
          .map((download) => [download.id, download]),
      );
      links.forEach((link) => {
        const download = downloads.get(link.dataset.releaseDownload || "");
        if (!download) return;
        link.href = download.url;
        link.dataset.releaseState = "ready";
        if (download.asset) {
          link.dataset.releaseAsset = download.asset;
        }
      });
    })
    .catch(() => {
      applyFallback();
    });

  function latestRelease(metadata) {
    const releases = Array.isArray(metadata?.releases) ? metadata.releases : [];
    return releases.find((release) => release.version === metadata.latest) || releases[0] || {};
  }

  function isSafeDownload(download) {
    if (!download || typeof download.id !== "string" || typeof download.url !== "string") {
      return false;
    }
    if (!download.url.startsWith("https://")) return false;
    if (download.url.includes("/releases/latest/download/")) return false;
    return true;
  }
})();
