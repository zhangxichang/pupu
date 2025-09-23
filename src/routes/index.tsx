import { Config } from "@/config";
import { createFileRoute, Navigate } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
    component: () => <Navigate to="/viewport" />,
    pendingComponent: Loading,
    loader: async () => {
        window.config = await Config.init();
    },
});
function Loading() {
    return <>
        <div className="w-screen h-screen flex items-center justify-center gap-1 bg-neutral-100">
            <div className="select-none font-bold">初始化中</div>
            <div className="icon-[line-md--loading-loop] w-6 h-6" />
        </div>
    </>;
}
