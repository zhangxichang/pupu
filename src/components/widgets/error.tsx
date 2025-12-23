import { BugIcon } from "lucide-solid";

export default function Error(props: { error: Error }) {
  return (
    <div class="flex-1 flex items-center justify-center gap-1">
      <BugIcon class="text-error" />
      <span class="text-error">{props.error.message}</span>
    </div>
  );
}
