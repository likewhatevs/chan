import { describe, expect, test } from "vitest";
import type { Terminal } from "@xterm/xterm";
import {
  installTerminalReportGuards,
  suppressOscIndexedColorReport,
  suppressOscSpecialColorReport,
} from "./xtermReports";

describe("xterm report guards", () => {
  test("registers OSC color report guards", () => {
    const registered: number[] = [];
    const term = {
      parser: {
        registerOscHandler(ident: number) {
          registered.push(ident);
          return { dispose() {} };
        },
      },
    } as unknown as Terminal;

    installTerminalReportGuards(term);

    expect(registered).toEqual([4, 10, 11, 12]);
  });

  test("suppresses special color queries but lets color sets fall through", () => {
    expect(suppressOscSpecialColorReport("?")).toBe(true);
    expect(suppressOscSpecialColorReport("?;?")).toBe(true);
    expect(suppressOscSpecialColorReport("#ffffff;?")).toBe(true);
    expect(suppressOscSpecialColorReport("#ffffff")).toBe(false);
    expect(suppressOscSpecialColorReport("rgb:eeee/eeee/f0f0")).toBe(false);
  });

  test("suppresses indexed color query pairs", () => {
    expect(suppressOscIndexedColorReport("1;?")).toBe(true);
    expect(suppressOscIndexedColorReport("1;#ffffff;2;?")).toBe(true);
    expect(suppressOscIndexedColorReport("1;#ffffff")).toBe(false);
    expect(suppressOscIndexedColorReport("1;rgb:eeee/eeee/f0f0")).toBe(false);
  });
});
