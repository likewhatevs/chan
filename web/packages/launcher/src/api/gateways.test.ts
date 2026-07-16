// The gateway client surface: the live client's routes are a frozen wire
// contract (the server pins the same paths byte-for-byte), the mock keeps
// CRUD exercisable in-memory with no seeds, and the demo fabricates one
// sample gateway for its populated variant.

import { describe, it, expect, afterEach, vi } from "vitest";
import { liveApi, type GatewayEntry } from "./library";
import { mockApi, resetMockGateways } from "./mock";
import { createLauncherDemoApi } from "./demo";

describe("liveApi gateway routes (frozen wire)", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  function stubFetch(status = 200, body: unknown = []): { calls: [string, RequestInit][] } {
    const calls: [string, RequestInit][] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string, init?: RequestInit) => {
        calls.push([url, init ?? {}]);
        return new Response(status === 204 ? null : JSON.stringify(body), { status });
      }),
    );
    return { calls };
  }

  it("lists, adds, removes, connects, and disconnects on /api/library/gateways", async () => {
    const { calls } = stubFetch();
    await liveApi.listGateways();
    const { calls: post } = stubFetch(200, {});
    await liveApi.addGateway({ url: "https://id.chan.app", label: "" });
    const { calls: rest } = stubFetch(204);
    await liveApi.removeGateway("gw-1a2b3c4d");
    await liveApi.connectGateway("gw-1a2b3c4d");
    await liveApi.disconnectGateway("gw-1a2b3c4d");

    expect(calls[0][0]).toBe("/api/library/gateways");
    expect(calls[0][1].method).toBe("GET");
    expect(post[0][0]).toBe("/api/library/gateways");
    expect(post[0][1].method).toBe("POST");
    expect(post[0][1].body).toBe(JSON.stringify({ url: "https://id.chan.app", label: "" }));
    expect(rest.map(([url, init]) => [url, init.method])).toEqual([
      ["/api/library/gateways/gw-1a2b3c4d", "DELETE"],
      ["/api/library/gateways/gw-1a2b3c4d/connect", "POST"],
      ["/api/library/gateways/gw-1a2b3c4d/disconnect", "POST"],
    ]);
  });

  it("parses the pinned GatewayEntry wire shape", async () => {
    // The Contract C sample: every key present, no extras consumed.
    const wire = {
      id: "gw-1a2b3c4d",
      url: "https://id.chan.app",
      label: "",
      enabled: true,
      status: "disconnected",
      pending_signin: false,
      devserver_count: 3,
      last_error: null,
    };
    stubFetch(200, [wire]);
    const list = await liveApi.listGateways();
    const entry: GatewayEntry = list[0];
    expect(entry).toEqual(wire);
  });
});

describe("mock gateway registry", () => {
  afterEach(() => {
    resetMockGateways();
  });

  it("starts empty; add/connect/disconnect/remove round-trip", async () => {
    expect(await mockApi.listGateways()).toEqual([]);
    const gw = await mockApi.addGateway({ url: "https://gw.example" });
    expect(gw.enabled).toBe(true);
    expect(gw.status).toBe("disconnected");

    await mockApi.connectGateway(gw.id);
    expect((await mockApi.listGateways())[0].status).toBe("connected");

    await mockApi.disconnectGateway(gw.id);
    const after = (await mockApi.listGateways())[0];
    expect(after.status).toBe("disconnected");
    expect(after.enabled).toBe(false);

    await mockApi.removeGateway(gw.id);
    expect(await mockApi.listGateways()).toEqual([]);
  });

  it("notifies watch subscribers on gateway mutations", async () => {
    let pushes = 0;
    const unsub = mockApi.watchWindows(() => {
      pushes += 1;
    });
    const seen = pushes; // the on-connect snapshot
    const gw = await mockApi.addGateway({ url: "https://gw.example" });
    await mockApi.removeGateway(gw.id);
    expect(pushes).toBeGreaterThan(seen);
    unsub();
  });
});

describe("demo gateway registry", () => {
  it("the populated variant fabricates one sample gateway", async () => {
    const demo = createLauncherDemoApi();
    const gateways = await demo.listGateways();
    expect(gateways).toHaveLength(1);
    expect(gateways[0].status).toBe("connected");
  });

  it("the empty variant seeds none", async () => {
    const demo = createLauncherDemoApi({ variant: "empty" });
    expect(await demo.listGateways()).toEqual([]);
  });
});
