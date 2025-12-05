import { Loading } from "@/components/loading";
import { createFileRoute, Outlet, useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";
import { createStore } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import { AppPath, FileSystem } from "@/lib/file_system";
import { Sqlite } from "@/lib/sqlite";
import { Errored } from "@/components/errored";
import { Endpoint } from "@/lib/endpoint";

let invoke_log: typeof import("@/lib/invoke/log") | undefined;
if (import.meta.env.TAURI_ENV_PLATFORM !== undefined) {
  invoke_log = await import("@/lib/invoke/log");
}

export const AppStore = createStore(
  subscribeWithSelector(() => ({
    fs: new FileSystem(),
    db: new Sqlite(),
    endpoint: new Endpoint(),
    on_resets: new Map<string, () => void | Promise<void>>(),
  })),
);
export const Route = createFileRoute("/window/app")({
  pendingComponent: () => <Loading />,
  errorComponent: () => {
    return <Errored text="我的天呀，应用程序运行出错了" />;
  },
  beforeLoad: async () => {
    if (invoke_log && !self.onunhandledrejection) {
      self.onunhandledrejection = async (e) => {
        if (e.reason instanceof Error) {
          await invoke_log.log_error(
            e.reason.stack ?? "未捕获的异常:异常没有栈信息",
          );
        } else {
          await invoke_log.log_error("未捕获的异常:非标准异常错误");
        }
      };
    }
    AppStore.getState().fs.init();
    await AppStore.getState().db.init();
    if (!(await AppStore.getState().db.is_open())) {
      await AppStore.getState().db.open(AppPath.DatabaseFile, true);
    }
    await AppStore.getState().endpoint.init();
  },
  component: () => {
    const navigate = useNavigate();
    //导航到子路由
    useEffect(() => {
      void navigate({ to: "/window/app/login" });
    }, []);
    return <Outlet />;
  },
});
