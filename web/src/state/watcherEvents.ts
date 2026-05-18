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
    note: typeof obj.note === "string" ? obj.note : undefined,
    path,
  };
}

export async function readWatcherEvents(dir: string): Promise<WatcherEvent[]> {
  const entries = await api.list(dir);
  const files = entries
    .filter((entry) => !entry.is_dir && eventFilename(entry.path))
    .sort((a, b) => a.path.localeCompare(b.path));
  const out: WatcherEvent[] = [];
  for (const file of files) {
    try {
      const body = await api.read(file.path);
      const parsed = parseWatcherEvent(file.path, body.content);
      if (parsed) out.push(parsed);
    } catch {
      // Watcher files are read-once best-effort. A file may disappear
      // between list and read if another agent cleans its outbox.
    }
  }
  return out;
}

export async function writeSurveyReply(
  sessionId: string,
  event: WatcherEvent,
  answers: Array<{ question_index: number; key: string }>,
  scopeGrant: ScopeGrant,
): Promise<void> {
  await api.writeTerminalEventReply(sessionId, {
    id: event.id,
    type: "survey-reply",
    from: REPLY_FROM,
    to: event.from,
    answers,
    scope_grant: scopeGrant,
  });
}

function eventFilename(path: string): boolean {
  const name = path.split("/").pop() ?? path;
  return /^event-.+\.(md|json)$/.test(name);
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
