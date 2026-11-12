export type WasmInit = (input: {
  module_or_path: Response;
}) => Promise<unknown>;

export type ResourcePhase = "download" | "execute";

export type ResourceDependency = {
  id: string;
  phase: ResourcePhase;
};

type CommonResourceInfo = {
  id: string;
  label: string;
  url: string;
  size: number;
  onDownload?: (bytes: ArrayBuffer) => void;
};

export type BasicResourceInfo = CommonResourceInfo & {
  kind: "basic";
};

export type WasmResourceInfo = CommonResourceInfo & {
  kind: "wasm";
  init: WasmInit;
  onStart: () => Promise<unknown>;
  dependsOn: readonly ResourceDependency[];
};

export type ResourceInfo = BasicResourceInfo | WasmResourceInfo;
