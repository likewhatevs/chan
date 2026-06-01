# Why I built chan

Author: drafted by Claude in @@Alex's first-person voice
Date: 2026-06-01
Status: PROPOSED for review. This is a draft of a first-person founder
note for the marketing site (an About / Story page). It is written AS
@@Alex, from the six motivations in branding-story.md section 15.
@@Alex: edit freely, this is your voice, not mine. Nothing ships until
you approve it. No marketing superlatives by design; the credibility is
in it being true.

---

I have been a Linux person since the mid-90s, and I have spent my career
as an engineer and then as an engineering manager. That means I write.
Code and design docs at work, and taxes and recipes and family trip
plans at home. For years I kept switching tools depending on which of
those I was doing and which machine I was on. In 2026 I wanted to stop.
I wanted one editor I could trust everywhere: on my laptop, and on a
remote machine over HTTP, reachable inbound or outbound through a tunnel.
That is the first reason chan exists.

The second is the terminal. I grew up in the console: DOS, bare tty on
Linux, window managers like WindowMaker and AfterStep and XFCE (I
contributed to XFCE's xfsound back in 1998), and a lot of GTK code for
video applications around the turn of the century. I have always needed
a real terminal to write code and drive builds. Today I need it for
something new: running several AI agents at once. Claude, Codex, Gemini.
Different agents for different jobs. At home I watch the cost; at work I
match the agent to the task; and I like mixing them in one session, say
four Claude and two Codex. I have lived in iTerm2 and tmux -CC for a long
time, and chan's terminal is built in that spirit, with broadcast,
groups, and command-line tooling to manage every session.

The third reason is what I think of as a second brain: keeping the
project's own knowledge close, and feeding it to the agents. Early on,
projects like qmd pointed the way, and chan started life as a text editor
for exactly that. Now chan runs its own MCP server, so the agents can
read and write the workspace directly, and the terminal tooling lets me
orchestrate them. That combination does things I had not seen before.

I did not arrive at the terminal gracefully. I tried wiring in the
provider APIs first. Then I tried embedding headless agents inside the
editor, fighting their interfaces the whole way. It did not feel right.
Eventually I stopped trying to hide the agents and gave them a real
terminal instead, and the tension disappeared. The TUI is not the enemy;
it is a foundation. Because the agents run on a pty, the terminal became
my orchestration layer, where agents can even poke and write to each
other.

The file browser started as a necessity and became one of my favorite
parts. Paired with the `cs` command line and the rest of chan, it makes
the whole thing flow. The inspector was born there and now shows up in
most of the hybrid tabs.

And the hybrid itself was almost an absurd idea: one surface that mixes
tabs of completely different kinds, editor and terminal and browser and
graph, all talking to each other. It works far better than I expected.
It feels local even when it is remote, it does not feel like a web
browser, and it survives a window reload without losing its place.

That is chan. It is the tool I wanted to stop looking for. I hope it is
useful to you too.

-- @@Alex
