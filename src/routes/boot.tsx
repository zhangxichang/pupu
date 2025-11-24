import "@xterm/xterm/css/xterm.css";
import { DndContext, useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { useEffect, useRef, useState } from "react";
import { Terminal as XTerm } from "@xterm/xterm";
import { restrictToWindowEdges } from "@dnd-kit/modifiers";
import { createFileRoute, Outlet, useNavigate } from "@tanstack/react-router";
import { Loading } from "@/components/loading";
import { AppStore } from "./boot/app";

export const Route = createFileRoute("/boot")({
  component: Component,
  pendingComponent: () => {
    return <Loading hint_text="正在引导应用程序" mode="screen" />;
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
    addEventListener("keydown", update);
    return () => {
      removeEventListener("keydown", update);
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
    id: "terminal",
  });
  return (
    <div
      ref={setNodeRef}
      className="absolute"
      style={{
        top: props.position.y,
        left: props.position.x,
        transform: CSS.Translate.toString(transform),
      }}
    >
      <div {...attributes} {...listeners} className="h-6 bg-blue-500" />
      <Terminal />
    </div>
  );
}

function Terminal() {
  const self_ref = useRef(null);
  useEffect(() => {
    if (self_ref.current) {
      const xterm = new XTerm({ fontSize: 12 });
      xterm.open(self_ref.current);
      const prompt = ">";
      let input_buffer = "";
      xterm.writeln(
        `开发者终端
        \r使用"?"命令获取帮助`,
      );
      xterm.write(prompt);
      xterm.onData(async (str) => {
        switch (str) {
          case "\r":
            xterm.write("\r\n");
            await execute_command(xterm, input_buffer as any);
            input_buffer = "";
            xterm.write(prompt);
            break;
          case "\x7F":
          case "\b":
            if (!input_buffer) return;
            input_buffer = input_buffer.slice(0, -1);
            xterm.write("\b \b");
            break;
          default:
            input_buffer += str;
            xterm.write(str);
            break;
        }
      });
      return () => xterm.dispose();
    }
  }, []);
  return <div ref={self_ref} />;
}

type Command = "?" | "db_close";
async function execute_command(xterm: XTerm, command: Command) {
  switch (command) {
    case "?":
      xterm.writeln(`db_close:关闭数据库连接`);
      break;
    case "db_close":
      await AppStore.getState().db.close();
      break;
    default:
      xterm.writeln(`未知命令:${command}`);
      break;
  }
}
