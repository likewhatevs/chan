import { api } from "../api/client";
import type { ScopeGrant, SurveyOption, SurveyQuestion, WatcherEvent } from "./tabs.svelte";

const REPLY_FROM = "@@Alex";
const STANDING_COMMENT_OPTION: SurveyOption = {
  key: "C",
  label: "Check my comments first",
};

export function normalizeStandingOptions(options: SurveyOption[] | undefined): SurveyOption[] {
  const out = [...(options ?? [])];
  if (!out.some((opt) => opt.label === STANDING_COMMENT_OPTION.label)) {
    out.push(STANDING_COMMENT_OPTION);
  }
  return out.slice(0, 4);
}

export function parseWatcherEvent(path: string, content: string): WatcherEvent | null {
  let raw: unknown;
  try {
    raw = JSON.parse(content);
  } catch {
    return null;
  }
  if (!raw || typeof raw !== "object") return null;
  const obj = raw as Record<string, unknown>;
  if (
    typeof obj.id !== "string" ||
    typeof obj.type !== "string" ||
    typeof obj.from !== "string" ||
    typeof obj.to !== "string"
  ) {
    return null;
  }
  if (
    obj.type !== "survey" &&
    obj.type !== "survey-reply" &&
    obj.type !== "poke" &&
    obj.type !== "pre-flight"
  ) {
    return null;
  }
  const questions = Array.isArray(obj.questions)
    ? obj.questions.map(parseQuestion).filter((q): q is SurveyQuestion => q !== null).slice(0, 4)
    : undefined;
  const standing = Array.isArray(obj.standing_options)
    ? obj.standing_options.map(parseOption).filter((o): o is SurveyOption => o !== null).slice(0, 4)
    : undefined;
  return {
    id: obj.id,
    type: obj.type,
    from: obj.from,
    to: obj.to,
    topic: typeof obj.topic === "string" ? obj.topic : undefined,
    questions,
    standing_options: standing,
    scope: parseScope(obj.scope),
    session: typeof obj.session === "string" ? obj.session : undefined,
    tab_label: typeof obj.tab_label === "string" ? obj.tab_label : undefined,
    note: typeof obj.note === "string" ? obj.note : undefined,
    path,
  };
}

/// systacean-9: list watcher event files for `sessionId`. The
/// chan-server endpoint reads from the session's attached
/// `watcher_dir` directly, bypassing the workspace sandbox. Outside-
/// workspace absolute paths (the lane-B repro case) now succeed; the
/// in-workspace case continues to work since the server simply reads
/// whatever `Registry::watcher_dir` returns.
///
/// Replaces the prior `api.list(dir) + api.read(path)` composition,
/// which routed both calls through `/api/files` and ENOENT-ed on
/// any path outside the workspace's `validate_rel` boundary. Server
/// pre-filters event filenames and sorts deterministically.
export async function readWatcherEvents(sessionId: string): Promise<WatcherEvent[]> {
  const entries = await api.terminalWatcherEvents(sessionId);
  const out: WatcherEvent[] = [];
  for (const entry of entries) {
    const parsed = parseWatcherEvent(entry.path, entry.content);
    if (parsed) out.push(parsed);
  }
  return out;
}

export async function writeSurveyReply(
  sessionId: string,
  event: WatcherEvent,
  answers: Array<{ question_index: number; key: string }>,
  scopeGrant: ScopeGrant,
  followUp = false,
): Promise<void> {
  await api.writeTerminalEventReply(sessionId, {
    id: event.id,
    type: "survey-reply",
    from: REPLY_FROM,
    to: event.from,
    answers,
    scope_grant: scopeGrant,
    ...(followUp ? { follow_up: true } : {}),
  });
}

function parseQuestion(value: unknown): SurveyQuestion | null {
  if (!value || typeof value !== "object") return null;
  const obj = value as Record<string, unknown>;
  if (typeof obj.header !== "string" || typeof obj.text !== "string") return null;
  const options = Array.isArray(obj.options)
    ? obj.options.map(parseOption).filter((o): o is SurveyOption => o !== null).slice(0, 3)
    : [];
  return { header: obj.header.slice(0, 12), text: obj.text, options };
}

function parseOption(value: unknown): SurveyOption | null {
  if (!value || typeof value !== "object") return null;
  const obj = value as Record<string, unknown>;
  if (typeof obj.key !== "string" || typeof obj.label !== "string") return null;
  return { key: obj.key, label: obj.label };
}

function parseScope(value: unknown): ScopeGrant | undefined {
  return value === "topic-session" || value === "topic-phase" || value === "one-shot"
    ? value
    : undefined;
}
