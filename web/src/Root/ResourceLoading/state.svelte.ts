// TODO AI实现待审查
import type { ResourceInfo, ResourceDependency, WasmInit } from "./types";
import { resourceDefinitions } from "../../resources";

export type ResourceStatus =
  | "pending"
  | "downloading"
  | "initializing"
  | "waiting"
  | "running"
  | "done"
  | "error"
  | "blocked";

export type ResourceState = {
  id: string;
  label: string;
  status: ResourceStatus;
  current: number;
  total: number;
  error: Error | null;
};

type PhaseDeferred = {
  promise: Promise<void>;
  readonly settled: boolean;
  readonly failed: boolean;
  readonly error: Error | null;
  resolve: () => void;
  fail: (error: Error) => void;
};

type ResourceRuntime = {
  definition: ResourceInfo;
  state: ResourceState;
  download: PhaseDeferred;
  execute: PhaseDeferred;
};

type ValidationFailure = {
  error: Error;
  resourceIds: readonly string[];
};

function createPhaseDeferred(): PhaseDeferred {
  let resolvePromise: () => void = () => {};
  let settled = false;
  let failed = false;
  let error: Error | null = null;
  const promise = new Promise<void>((resolve) => {
    resolvePromise = resolve;
  });

  return {
    promise,
    get settled() {
      return settled;
    },
    get failed() {
      return failed;
    },
    get error() {
      return error;
    },
    resolve() {
      if (settled) return;
      settled = true;
      resolvePromise();
    },
    fail(cause) {
      if (settled) return;
      settled = true;
      failed = true;
      error = cause;
      resolvePromise();
    },
  };
}

export const resourceLoading = $state<{ resources: ResourceState[] }>({
  resources: resourceDefinitions.map(({ id, label }): ResourceState => ({
    id,
    label,
    status: "pending",
    current: 0,
    total: 0,
    error: null,
  })),
});

let started = false;

export function startResourceLoading() {
  if (started) return;
  started = true;

  const validation = validateResourceDefinitions(resourceDefinitions);
  const runtimes: ResourceRuntime[] = [];

  for (const definition of resourceDefinitions) {
    const state = resourceLoading.resources.find(
      (resource) => resource.id === definition.id,
    );
    if (!state) throw new Error(`资源状态缺失：${definition.id}`);
    runtimes.push({
      definition,
      state,
      download: createPhaseDeferred(),
      execute: createPhaseDeferred(),
    });
  }

  if (validation) {
    for (const state of resourceLoading.resources) {
      const failed = validation.resourceIds.includes(state.id);
      state.status = failed ? "error" : "blocked";
      state.error = failed
        ? validation.error
        : new Error(`资源「${state.label}」前置资源配置错误`);
    }
    for (const runtime of runtimes) {
      const error =
        runtime.state.error ??
        new Error(`资源「${runtime.definition.label}」前置资源配置错误`);
      runtime.download.fail(error);
      runtime.execute.fail(error);
    }
    console.error(validation.error);
    return;
  }

  for (const runtime of runtimes) {
    void runResource(runtime, runtimes);
  }
}

async function runResource(
  runtime: ResourceRuntime,
  runtimes: readonly ResourceRuntime[],
): Promise<void> {
  const { definition, state } = runtime;
  let phase = "加载";

  try {
    state.status = "downloading";
    state.current = 0;
    state.total = 0;
    state.error = null;

    const response = await fetch(definition.url);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} ${response.statusText}`);
    }
    state.total = definition.size;

    const bytes = await trackDownload(
      response,
      (loaded) => {
        state.current =
          state.total > 0 ? Math.min(loaded, state.total) : loaded;
      },
      () => {
        state.current = state.total;
      },
    ).arrayBuffer();

    state.total = state.total || bytes.byteLength;
    state.current = state.total;
    definition.onDownload?.(bytes);
    runtime.download.resolve();

    if (await waitForDependencies(runtime, runtimes)) {
      return;
    }

    if (definition.kind === "wasm") {
      phase = "初始化";
      state.status = "initializing";
      await definition.init({
        module_or_path: new Response(bytes, {
          headers: { "Content-Type": "application/wasm" },
        }),
      });

      phase = "执行";
      state.status = "running";
      await definition.onStart();
    }

    runtime.execute.resolve();
    state.status = "done";
  } catch (cause) {
    const detail = cause instanceof Error ? `：${cause.message}` : "";
    const error = new Error(
      `资源「${definition.label}」${phase}失败${detail}`,
      { cause },
    );
    state.status = "error";
    state.error = error;
    if (!runtime.download.settled) {
      runtime.download.fail(error);
    }
    if (!runtime.execute.settled) {
      runtime.execute.fail(error);
    }
    console.error(error);
    refreshBlockedStatuses(runtimes);
  }
}

async function waitForDependencies(
  runtime: ResourceRuntime,
  runtimes: readonly ResourceRuntime[],
): Promise<boolean> {
  const dependencies =
    runtime.definition.kind === "wasm" ? runtime.definition.dependsOn : [];
  if (dependencies.length === 0) return false;

  if (
    dependencies.some((dependency) => dependencyFailed(dependency, runtimes))
  ) {
    markBlocked(runtime, runtimes);
    return true;
  }

  runtime.state.status = "waiting";
  await Promise.all(
    dependencies.map(
      (dependency) => dependencyDeferred(dependency, runtimes).promise,
    ),
  );

  if (
    dependencies.some((dependency) => dependencyFailed(dependency, runtimes))
  ) {
    markBlocked(runtime, runtimes);
    return true;
  }

  return false;
}

function markBlocked(
  runtime: ResourceRuntime,
  runtimes: readonly ResourceRuntime[],
): void {
  const error = new Error(`资源「${runtime.definition.label}」前置资源失败`);
  runtime.state.status = "blocked";
  runtime.state.error = error;
  if (!runtime.execute.settled) {
    runtime.execute.fail(error);
  }
  refreshBlockedStatuses(runtimes);
}

function findRuntime(
  runtimes: readonly ResourceRuntime[],
  id: string,
): ResourceRuntime | undefined {
  return runtimes.find((runtime) => runtime.definition.id === id);
}

function dependencyDeferred(
  dependency: ResourceDependency,
  runtimes: readonly ResourceRuntime[],
): PhaseDeferred {
  const target = findRuntime(runtimes, dependency.id);
  if (!target) {
    throw new Error(`资源依赖缺失：${dependency.id}`);
  }
  return dependency.phase === "download" ? target.download : target.execute;
}

function dependencyFailed(
  dependency: ResourceDependency,
  runtimes: readonly ResourceRuntime[],
): boolean {
  const target = findRuntime(runtimes, dependency.id);
  if (!target) return true;
  return dependencyDeferred(dependency, runtimes).failed;
}

function refreshBlockedStatuses(runtimes: readonly ResourceRuntime[]) {
  let changed = true;
  while (changed) {
    changed = false;
    for (const runtime of runtimes) {
      if (
        runtime.state.status !== "waiting" &&
        runtime.state.status !== "blocked"
      ) {
        continue;
      }

      const dependencies =
        runtime.definition.kind === "wasm" ? runtime.definition.dependsOn : [];
      const blocked = dependencies.some((dependency) =>
        dependencyFailed(dependency, runtimes),
      );
      const nextStatus: ResourceStatus = blocked ? "blocked" : "waiting";

      if (runtime.state.status !== nextStatus) {
        runtime.state.status = nextStatus;
        if (blocked) {
          runtime.state.error ??= new Error(
            `资源「${runtime.definition.label}」前置资源失败`,
          );
          if (!runtime.execute.settled) {
            runtime.execute.fail(runtime.state.error);
          }
        }
        changed = true;
      }
    }
  }
}

function validateResourceDefinitions(
  definitions: readonly ResourceInfo[],
): ValidationFailure | null {
  const allIds = definitions.map((definition) => definition.id);
  const ids: string[] = [];
  for (const definition of definitions) {
    if (ids.includes(definition.id)) {
      return {
        error: new Error(`重复的资源 id：${definition.id}`),
        resourceIds: allIds,
      };
    }
    ids.push(definition.id);
  }

  const urls: string[] = [];
  const inits: WasmInit[] = [];
  for (const definition of definitions) {
    if (urls.includes(definition.url)) {
      return {
        error: new Error(`重复的资源 url：${definition.id}`),
        resourceIds: allIds,
      };
    }
    urls.push(definition.url);

    if (definition.kind === "wasm") {
      if (inits.includes(definition.init)) {
        return {
          error: new Error(`重复的资源 init：${definition.id}`),
          resourceIds: allIds,
        };
      }
      inits.push(definition.init);
    }
  }

  for (const definition of definitions) {
    const dependencies = definition.kind === "wasm" ? definition.dependsOn : [];
    for (const dependency of dependencies) {
      if (dependency.id === definition.id) {
        return {
          error: new Error(`资源「${definition.label}」不能依赖自身`),
          resourceIds: [definition.id],
        };
      }

      if (!ids.includes(dependency.id)) {
        return {
          error: new Error(
            `资源「${definition.label}」依赖未知资源：${dependency.id}`,
          ),
          resourceIds: [definition.id],
        };
      }
    }
  }

  const cycle = findExecuteDependencyCycle(definitions);
  if (cycle) {
    return {
      error: new Error(`资源执行依赖成环：${cycle.join(" -> ")}`),
      resourceIds: cycle,
    };
  }

  return null;
}

function findExecuteDependencyCycle(
  definitions: readonly ResourceInfo[],
): string[] | null {
  const visiting: string[] = [];
  const visited: string[] = [];
  const stack: string[] = [];

  const visit = (id: string): string[] | null => {
    if (visiting.includes(id)) {
      const start = stack.indexOf(id);
      return [...stack.slice(start), id];
    }
    if (visited.includes(id)) return null;

    visiting.push(id);
    stack.push(id);

    const definition = definitions.find((item) => item.id === id);
    const dependencies =
      definition?.kind === "wasm" ? definition.dependsOn : [];
    for (const dependency of dependencies) {
      if (dependency.phase !== "execute") continue;
      const cycle = visit(dependency.id);
      if (cycle) return cycle;
    }

    stack.pop();
    visiting.pop();
    visited.push(id);
    return null;
  };

  for (const definition of definitions) {
    const cycle = visit(definition.id);
    if (cycle) return cycle;
  }

  return null;
}

function trackDownload(
  response: Response,
  onProgress: (current: number) => void,
  onComplete: () => void,
): Response {
  if (!response.body) {
    onComplete();
    return response;
  }

  let loaded = 0;
  return new Response(
    response.body.pipeThrough(
      new TransformStream<Uint8Array, Uint8Array>({
        transform(chunk, controller) {
          loaded += chunk.byteLength;
          onProgress(loaded);
          controller.enqueue(chunk);
        },
        flush() {
          onComplete();
        },
      }),
    ),
    {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers,
    },
  );
}
