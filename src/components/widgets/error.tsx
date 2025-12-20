import { Bug } from "lucide-solid";

export default function Error(props: { error: Error }) {
  return (
    <div class="flex-1 flex items-center justify-center">
      <Bug class="bg-error" />
      <span>{props.error.stack}</span>
    </div>
  );
}
