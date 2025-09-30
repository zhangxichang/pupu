import { Database } from "@/lib/database"
import { createFileRoute, Navigate, Outlet } from "@tanstack/react-router"

declare global {
    var database: Database | null
}

export const Route = createFileRoute("/window/app")({
    component: () => <>
        <Outlet />
        <Navigate to="/window/app/login" />
    </>,
    pendingComponent: PendingComponent,
    loader: async () => {
        if (!window.database) window.database = await Database.new();
    }
})

function PendingComponent() {
    return <div className="flex-1 flex items-center justify-center gap-1">
        <div className="select-none font-bold">正在加载应用界面</div>
        <div className="icon-[line-md--loading-loop] w-6 h-6" />
    </div>
}
