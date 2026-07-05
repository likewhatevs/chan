import type { Preferences } from "../../api/types";

// A single-field settings write. `mutate` returns the preferences with
// one slice changed; `persist`, when a field has a dedicated store/api
// setter (theme), runs that instead of the generic serial PATCH. The
// parent surface owns the optimistic apply and the in-flight guard, so a
// section stays purely presentational.
export type CommitFn = (
  mutate: (p: Preferences) => Preferences,
  persist?: () => Promise<unknown>,
) => void;
