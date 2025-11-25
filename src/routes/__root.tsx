import { Loading } from "@/components/loading";
import { createRootRoute, HeadContent, Outlet } from "@tanstack/react-router";

export const Route = createRootRoute({
  head: () => ({
    meta: [{ title: "星链" }],
  }),
  component: Component,
  pendingComponent: () => <Loading hint_text="正在初始化框架" mode="screen" />,
});
function Component() {
  return (
    <>
      <HeadContent />
      <Outlet />
    </>
  );
}
