import { api, HttpError, type Me } from "../lib/api";

type Status = "idle" | "loading" | "loaded" | "anon" | "error";

class MeStore {
  status = $state<Status>("idle");
  me = $state<Me | null>(null);
  providers = $state<string[]>([]);
  error = $state<string | null>(null);

  async load() {
    this.status = "loading";
    this.error = null;
    try {
      // /api/providers is unauth. Run it in parallel with /api/me so
      // the Login view is ready as soon as the auth status resolves.
      const [providersResult, meResult] = await Promise.allSettled([
        api.providers(),
        api.me(),
      ]);
      if (providersResult.status === "fulfilled") {
        this.providers = providersResult.value.providers;
      }
      if (meResult.status === "fulfilled") {
        this.me = meResult.value;
        this.status = "loaded";
      } else if (
        meResult.reason instanceof HttpError && meResult.reason.status === 401
      ) {
        this.me = null;
        this.status = "anon";
      } else {
        const r = meResult.reason;
        this.error = r instanceof Error ? r.message : String(r);
        this.status = "error";
      }
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      this.status = "error";
    }
  }

  /// Refetch /api/me without going through the loading state. Used
  /// by the Devservers view to refresh the list after a manual nudge
  /// (an open succeeded / failed, or the user just connected a new
  /// `chan devserver`).
  async refresh() {
    try {
      this.me = await api.me();
    } catch (e) {
      // Soft-fail: leave the current list visible and surface the
      // error on the view itself.
      if (e instanceof HttpError && e.status === 401) {
        this.me = null;
        this.status = "anon";
      }
    }
  }

  async logout() {
    await api.logout();
    this.me = null;
    this.status = "anon";
  }

  async deleteAccount() {
    await api.deleteAccount();
    this.me = null;
    this.status = "anon";
  }

  async updateUsername(username: string) {
    const res = await api.updateUsername(username);
    if (this.me) {
      this.me.user.username = res.username;
      this.me.user.username_edits = MAX_USERNAME_EDITS - res.edits_remaining;
    }
    return res;
  }
}

// Mirrors MAX_USERNAME_EDITS in identity-service. Used to keep the
// User.username_edits field in sync after a rename so callers don't
// need to refetch /api/me.
export const MAX_USERNAME_EDITS = 4;

export const meStore = new MeStore();
