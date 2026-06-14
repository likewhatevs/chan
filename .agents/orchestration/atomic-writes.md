# Atomic event writes

The chan-server watcher uses fsnotify to detect new event files in a user-chosen directory. It reads each file **once** on `Create` / rename-final events, parses it as JSON, and dispatches. There is no retry, no defensive multi-read.

This means writers MUST publish event files atomically. The only POSIX-portable way to do that is: write to a temp file in the same directory, then `rename` it to the final name. Same filesystem = atomic visibility.

If you write directly to the final filename, the watcher can read your file mid-write and get a truncated / malformed JSON. The watcher will log + drop that event; no dispatch.

## Per-language minimal examples

### bash

```bash
dir=/path/to/watcher-dir
final=$dir/event-$(uuidgen).md
tmp=$dir/.event.$$.tmp
printf '%s' "$payload" > "$tmp"
mv "$tmp" "$final"
```

`mv` between two paths on the same filesystem calls `rename(2)`, which is atomic. The leading dot on the temp name keeps the file out of any glob-based listings while it's being written.

### python

```python
import os, json, uuid
dir = "/path/to/watcher-dir"
final = f"{dir}/event-{uuid.uuid4().hex}.md"
tmp = f"{dir}/.event-{os.getpid()}.tmp"
with open(tmp, "w") as f:
    json.dump(payload, f)
os.replace(tmp, final)
```

`os.replace` is the atomic rename primitive (it works on both POSIX and Windows; `os.rename` is non-atomic on Windows if the destination exists).

### rust

```rust
use std::{fs, path::Path};
let dir = Path::new("/path/to/watcher-dir");
let final_path = dir.join(format!("event-{}.md", uuid));
let tmp = dir.join(format!(".event-{}.tmp", std::process::id()));
fs::write(&tmp, payload)?;
fs::rename(&tmp, &final_path)?;
```

### node / TypeScript

```ts
import * as fs from "node:fs/promises";
import { randomUUID } from "node:crypto";
const dir = "/path/to/watcher-dir";
const final = `${dir}/event-${randomUUID()}.md`;
const tmp = `${dir}/.event-${process.pid}.tmp`;
await fs.writeFile(tmp, payload);
await fs.rename(tmp, final);
```

## The no-self-loop rule

chan-server's reaction to a watched event is to write `poke\n` to a target agent's PTY. It NEVER writes a file into the watched directory. Watcher writers should follow the same posture: if your agent reacts to an incoming event, write any outbound response into a DIFFERENT directory, or use chan-server's reply endpoint (see [spawn-protocol.md](./spawn-protocol.md)) which bypasses the watcher entirely.

Writing into the same directory you watch creates an infinite loop. Easy to do, hard to debug.

## Event schema

The full survey / survey-reply schema is preserved in git history. Minimal version:

```json
{
  "id": "<unique-id>",
  "type": "survey",
  "from": "@@SomeAgent",
  "to": "@@Host",
  "topic": "<short-topic-tag>",
  "questions": [
    {
      "header": "<short label>",
      "text": "<question text>",
      "options": [
        {"key": "1", "label": "yes"},
        {"key": "2", "label": "no"}
      ]
    }
  ],
  "standing_options": [
    {"key": "C", "label": "Check my comments first"}
  ],
  "scope": "one-shot"
}
```

Required fields: `id`, `type`, `from`, `to`. Everything else is optional. Unknown `type` values are logged and ignored; this gives you forward-compat for adding new event categories without breaking existing watchers.
