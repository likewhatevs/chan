<script lang="ts">
  // Chan's brand mark (ensō) sized like a lucide icon so it can sit
  // next to them in menus. Painted via CSS mask of /chan-mark.png
  // tinted with --assistant-accent, same recipe as AccessoryPill's
  // pill so the menu icon and the bottom-pill logo read as one
  // visual identity.
  //
  // strokeWidth is accepted (and ignored) so call sites can keep the
  // same prop shape they use for lucide icons.
  let {
    size = 16,
    strokeWidth: _strokeWidth = 1.75,
  }: { size?: number; strokeWidth?: number } = $props();
</script>

<span
  class="enso-icon"
  style:--enso-icon-size="{size}px"
  aria-hidden="true"
></span>

<style>
  .enso-icon {
    display: inline-block;
    width: var(--enso-icon-size);
    height: var(--enso-icon-size);
    flex-shrink: 0;
    background-color: var(--assistant-accent);
    -webkit-mask: url('/chan-mark.png') center / contain no-repeat;
            mask: url('/chan-mark.png') center / contain no-repeat;
    vertical-align: middle;
  }
</style>
