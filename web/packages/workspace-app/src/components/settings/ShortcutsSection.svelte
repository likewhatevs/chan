<script lang="ts">
  // Shortcut-assignment mount point. This lane owns the container and
  // its placement inside the settings surface; the assignment rows,
  // chord capture, and conflict detection are the Keymap lane's. Its
  // view mounts here in place of the placeholder below.
</script>

<div class="shortcuts">
  <p class="lead">Assign a keyboard shortcut to any command.</p>
  <p class="hint">
    Pick a command and press the keys you want. Shortcuts can be set per
    operating system, so the same configuration travels between machines.
  </p>
</div>

<style>
  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 24px 0;
  }
  .lead {
    margin: 0;
    font-size: 14px;
    color: var(--text);
  }
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.5;
    max-width: 46ch;
  }
</style>
