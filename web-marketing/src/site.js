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
