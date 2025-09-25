import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/window/chat")({
    component: Component
});
function Component() {
    return <>
        <div className="flex-1 flex">
        </div>
    </>;
}
