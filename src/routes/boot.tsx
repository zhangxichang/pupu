import { DndContext, useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { useEffect, useState } from "react";
import { restrictToWindowEdges } from "@dnd-kit/modifiers";
import { createFileRoute, Outlet, useNavigate } from "@tanstack/react-router";
import { Loading } from "@/components/loading";

let tauri_error: typeof import("@/lib/invoke/error") | undefined;
if (import.meta.env.TAURI_ENV_PLATFORM) {
  tauri_error = await import("@/lib/invoke/error");
}

export const Route = createFileRoute("/boot")({
  component: Component,
  pendingComponent: () => {
    return <Loading hint_text="正在引导应用程序" mode="screen" />;
  },
  beforeLoad: () => {
    if (tauri_error) {
      self.onunhandledrejection = (e) => {
        tauri_error.fatal_error(e.reason);
      };
    }
  },
});
function Component() {
  const navigate = useNavigate();
  const [position, set_position] = useState<{ x: number; y: number }>({
    x: 4,
    y: 4,
  });
  const [is_open, set_is_open] = useState(false);
  useEffect(() => {
    const update = (e: KeyboardEvent) => {
      if (e.key === "`") {
        set_is_open((v) => !v);
      }
    };
    self.addEventListener("keydown", update);
    return () => {
      self.removeEventListener("keydown", update);
    };
  }, []);
  //导航到应用
  useEffect(() => {
    navigate({ to: "/boot/app" });
  }, []);
  return (
    <>
      <Outlet />
      <DndContext
        modifiers={[restrictToWindowEdges]}
        onDragEnd={(e) =>
          set_position((v) => ({
            x: v.x + e.delta.x,
            y: v.y + e.delta.y,
          }))
        }
      >
        {is_open && <Window position={position} />}
      </DndContext>
    </>
  );
}

function Window(props: { position: { x: number; y: number } }) {
  const { setNodeRef, transform, attributes, listeners } = useDraggable({
    id: "debug_view",
  });
  return (
    <div
      ref={setNodeRef}
      className="absolute w-5xl h-128 flex flex-col border bg-white"
      style={{
        top: props.position.y,
        left: props.position.x,
        transform: CSS.Translate.toString(transform),
      }}
    >
      <div {...attributes} {...listeners} className="h-8">
        调试界面
      </div>
      <DebugView />
    </div>
  );
}

function DebugView() {
  return <div className="flex-1 flex flex-col"></div>;
}
