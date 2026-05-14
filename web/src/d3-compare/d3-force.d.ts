// Minimal ambient declaration so the demo can import d3-force
// without pulling in `@types/d3-force` as a workspace dep. We
// only use a handful of the library's surface; everything else
// stays `any` and the call-site uses TS generics for safety.
declare module "d3-force" {
  export interface SimulationNodeDatum {
    index?: number;
    x?: number;
    y?: number;
    vx?: number;
    vy?: number;
    fx?: number | null;
    fy?: number | null;
  }
  export interface SimulationLinkDatum<N extends SimulationNodeDatum> {
    source: string | N;
    target: string | N;
    index?: number;
  }

  export interface Force<N extends SimulationNodeDatum> {
    (alpha: number): void;
    initialize?(nodes: N[], random: () => number): void;
  }

  export interface Simulation<N extends SimulationNodeDatum, L> {
    restart(): this;
    stop(): this;
    tick(iterations?: number): this;
    nodes(): N[];
    nodes(nodes: N[]): this;
    alpha(): number;
    alpha(alpha: number): this;
    alphaMin(): number;
    alphaMin(min: number): this;
    alphaDecay(): number;
    alphaDecay(decay: number): this;
    alphaTarget(): number;
    alphaTarget(target: number): this;
    velocityDecay(): number;
    velocityDecay(decay: number): this;
    force(name: string, force?: unknown): this;
    on(event: string, listener: () => void): this;
    find(x: number, y: number, radius?: number): N | undefined;
  }

  export function forceSimulation<N extends SimulationNodeDatum>(
    nodes?: N[],
  ): Simulation<N, unknown>;

  export interface ForceLink<N extends SimulationNodeDatum, L> {
    (alpha: number): void;
    links(links?: L[]): L[] | this;
    id(id?: (d: N) => string | number): this;
    distance(d: number | ((link: L) => number)): this;
    strength(s: number | ((link: L) => number)): this;
    iterations(n?: number): number | this;
  }
  export function forceLink<N extends SimulationNodeDatum, L>(
    links?: L[],
  ): ForceLink<N, L>;

  export interface ForceManyBody<N extends SimulationNodeDatum> {
    (alpha: number): void;
    strength(s: number | ((node: N) => number)): this;
    distanceMin(d?: number): number | this;
    distanceMax(d?: number): number | this;
    theta(t?: number): number | this;
  }
  export function forceManyBody<N extends SimulationNodeDatum>(): ForceManyBody<N>;

  export interface ForceCollide<N extends SimulationNodeDatum> {
    (alpha: number): void;
    radius(r: number | ((node: N) => number)): this;
    strength(s?: number): number | this;
    iterations(n?: number): number | this;
  }
  export function forceCollide<N extends SimulationNodeDatum>(
    radius?: number | ((node: N) => number),
  ): ForceCollide<N>;

  export interface ForceCenter<N extends SimulationNodeDatum> {
    (alpha: number): void;
    x(x?: number): number | this;
    y(y?: number): number | this;
    strength(s?: number): number | this;
  }
  export function forceCenter<N extends SimulationNodeDatum>(
    x?: number,
    y?: number,
  ): ForceCenter<N>;

  export interface ForceXY<N extends SimulationNodeDatum> {
    (alpha: number): void;
    strength(s?: number | ((node: N) => number)): number | this;
    x(x?: number | ((node: N) => number)): number | this;
    y(y?: number | ((node: N) => number)): number | this;
  }
  export function forceX<N extends SimulationNodeDatum>(
    x?: number | ((node: N) => number),
  ): ForceXY<N>;
  export function forceY<N extends SimulationNodeDatum>(
    y?: number | ((node: N) => number),
  ): ForceXY<N>;
}
