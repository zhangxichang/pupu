import { createRootRoute, HeadContent, Outlet } from "@tanstack/react-router";

export const Route = createRootRoute({
  head: () => ({
    meta: [{ title: "星链" }],
  }),
  component: Component,
});
function Component() {
  return (
    <>
      <HeadContent />
      <Outlet />
    </>
  );
}
