import { createFileRoute } from "@tanstack/react-router"

export const Route = createFileRoute("/window/app/chat/$account_id/chatbar")({
    component: Component,
    pendingComponent: PendingComponent,
})

function Component() {
    return <div className="flex-1 flex">
        你好，世界
    </div>
}
function PendingComponent() {
    return <div className="flex-1 flex items-center justify-center gap-1">
        <div className="select-none font-bold">正在加载聊天栏</div>
        <div className="icon-[line-md--loading-loop] w-6 h-6" />
    </div>
}
