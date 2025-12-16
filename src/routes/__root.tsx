import { createRootRoute, HeadContent, Outlet } from "@tanstack/solid-router";

export const Route = createRootRoute({
  head: () => ({
    meta: [{ title: "星链" }],
  }),
  component: () => (
    <>
      <HeadContent />
      <Outlet />
    </>
  ),
});
