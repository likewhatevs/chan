# story.html setup + seamless paragraph — DRAFT for @@Host review

Status: RESOLVED. @@Host picked Variant A with one correction (the VPS
tunnel terminates at the gateway; he reaches the workspace via the gateway)
and "a Linux VM" (no Lima). Final committed as ba73ce5a (taxes was
3e94bd5b). This file is the review record; the variants below are kept as
the audit trail of the choice.

The point to land (per #3): not "I run three setups" but "all three feel
local; the remote ones are as seamless as the laptop." It also fixes a
small inaccuracy in the current line (outbound is NOT through a tunnel).

## Current paragraph (story.html:7-16, taxes already applied)

> I have been a Linux person since the mid-90s, and I have spent my career
> as an engineer and then as an engineering manager. That means I write.
> Code and design docs at work, and cooking recipes and family trip plans
> at home. For years I kept switching tools depending on which of those I
> was doing and which machine I was on. In 2026 I wanted to stop. **I wanted
> one editor I could trust everywhere: on my laptop, and on a remote machine
> over HTTP, reachable inbound or outbound through a tunnel.** That is the
> first reason chan exists.

Only the bold sentence changes (and grows). The rest of the paragraph
stays as-is.

## Variant A (narrative — recommended; matches the reflective voice)

> ... In 2026 I wanted to stop. I wanted one editor I could trust
> everywhere, and for it to feel the same whether the files were on my
> laptop or on a machine across the network. Today I run chan across three
> at once, and all three feel local. One workspace is the laptop itself.
> The second is a Linux VM on that same Mac, which the desktop app reaches
> by dialing out to its `chan serve` over HTTP/2. The third lives on a VPS
> and comes the other way, tunneling back to me inbound through chan's own
> gateway. The two remote ones are as immediate as the one in front of me;
> I stop thinking about where the workspace actually runs. That is the
> first reason chan exists.

## Variant B (more technical — names Lima / Chan Desktop / self-hosted gateway)

> ... In 2026 I wanted to stop. I wanted one editor I could trust
> everywhere, and for the remote machines to feel no different from the
> laptop. Today I run chan across three environments at once and all three
> feel local. The first is the laptop, a plain `chan serve`. The second is
> a Linux VM (Lima) on the same Mac, which Chan Desktop attaches to
> outbound, dialing its `chan serve` over HTTP/2. The third is a VPS that
> reaches me inbound, tunneling back through the self-hosted gateway that
> fronts it. The VM and the VPS are as seamless as the local workspace; I
> never think about which one I am in. That is the first reason chan exists.

## Accuracy mapping (D1 terminology, kept exact)

| @@Host machine        | what happens                          | D1 term          |
|-----------------------|---------------------------------------|------------------|
| local macOS laptop    | plain `chan serve` on loopback        | local            |
| Lima VM on same Mac   | desktop dials OUT to the VM's         | outbound (direct |
|                       | `chan serve` over HTTP/2              | HTTP/2, no tunnel|
| VPS                   | VM's... VPS's `chan serve` tunnels    | inbound (tunnel  |
|                       | IN via gateway / workspace-proxy      | + gateway stack) |

Note: the current line says "inbound or outbound through a tunnel", which
lumps outbound in with the tunnel. Outbound is a direct HTTP/2 dial to a
remote `chan serve` (no tunnel); only inbound uses the tunnel + gateway.
Both variants fix that.

In the HTML, `chan serve` renders as `<code>chan serve</code>` (the page
already uses `<code>` for `cs`).

## Questions for @@Host

1. Variant A or B (or a blend)?
2. Name "Lima" explicitly (B does, A says "a Linux VM")?
3. Is "the two remote ones are as immediate as the one in front of me" the
   seamless framing you want, or sharper?
