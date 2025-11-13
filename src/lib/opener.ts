type Native = { kind: "Native" } & typeof import("@tauri-apps/plugin-opener");
type Web = { kind: "Web" };

let api: Native | Web;
if (import.meta.env.TAURI_ENV_PLATFORM) {
  api = { kind: "Native", ...(await import("@tauri-apps/plugin-opener")) };
}
if (!import.meta.env.TAURI_ENV_PLATFORM) {
  api = { kind: "Web" };
}

export async function open_url(url: string) {
  if (api.kind === "Native") {
    await api.openUrl(url);
  } else if (api.kind === "Web") {
    open(url);
  } else {
    throw new Error("API缺失");
  }
}
