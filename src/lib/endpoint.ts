export let EndpointModuleAdapter: (typeof import("./endpoint/web"))["EndpointModuleImpl"];

if (import.meta.env.TAURI_ENV_PLATFORM !== undefined) {
  //TODO
} else {
  const mod = await import("./endpoint/web");
  EndpointModuleAdapter = mod.EndpointModuleImpl;
}
