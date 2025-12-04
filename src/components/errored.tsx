import clsx from "clsx";
import { Label } from "@/shadcn/components/ui/label";
import { Bug } from "lucide-react";

export function Errored(props: { mode?: "screen" | "flex"; text: string }) {
  return (
    <div
      className={clsx(
        "flex items-center justify-center gap-1",
        props.mode === "screen" && "w-dvw h-dvh",
        props.mode === "flex" || (!props.mode && "flex-1"),
      )}
    >
      <Bug className="size-6 text-red-600" />
      <Label className="font-bold text-red-600">{props.text}</Label>
    </div>
  );
}
