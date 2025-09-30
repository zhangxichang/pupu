import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Button } from "@/components/ui/button"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"
import { Input } from "@/components/ui/input"
import { Network } from "@/lib/network"
import { createFileRoute, Link, Outlet } from "@tanstack/react-router"
import { Account } from "@zhangxichang/network"
import { Clipboard, LogOut } from "lucide-react"
import { useEffect } from "react"

declare global {
    var network: Network | null
}

export const Route = createFileRoute("/window/app/chat/$account_id")({
    component: Component,
    pendingComponent: PendingComponent,
    loader: async ({ params }) => {
        if (!window.network) window.network = await Network.new(Account.from_json(await database!.get("accounts", params.account_id)));
    }
})

function Component() {
    const params = Route.useParams();
    //获取当前用户
    const account = (() => {
        const account = network!.account()
        return {
            id: account.id,
            name: account.name,
            key: account.key,
            avatar_url: account.avatar && URL.createObjectURL(new Blob([Uint8Array.from(account.avatar)]))
        }
    })()
    //加载与清理
    useEffect(() => {
        if (account.avatar_url) {
            const avatar_url = account.avatar_url
            return () => URL.revokeObjectURL(avatar_url)
        }
    }, [])
    return <div className="flex-1 flex border-t">
        <div className="w-64 flex flex-col border-r">
            <div className="h-12 flex border-b items-center px-1">
                <Input placeholder="搜索用户" />
            </div>
            <div className="flex-1 flex flex-col border-b">
                <Link
                    to="/window/app/chat/$account_id/chatbar"
                    params={params}
                    className="h-12 flex items-center px-2 gap-2 hover:bg-neutral-100"
                >
                    <Avatar>
                        <AvatarImage src={account.avatar_url} />
                        <AvatarFallback>{account.name[0]}</AvatarFallback>
                    </Avatar>
                    <span>{account.name}</span>
                </Link>
            </div>
            <div className="h-15 flex items-center px-2">
                <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                        <Button variant={"ghost"} className="flex-1 h-11 p-0 justify-start px-2">
                            <Avatar>
                                <AvatarImage src={account.avatar_url} />
                                <AvatarFallback>{account.name[0]}</AvatarFallback>
                            </Avatar>
                            <span>{account.name}</span>
                        </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start">
                        <DropdownMenuItem onClick={() => navigator.clipboard.writeText(account.id)}>
                            <Clipboard />
                            <span>复制用户ID</span>
                        </DropdownMenuItem>
                        <DropdownMenuItem asChild>
                            <Link
                                to="/window/app/login"
                                onClick={async () => {
                                    await network!.shutdown()
                                    window.network = null
                                }}
                            >
                                <LogOut />
                                <span>登出</span>
                            </Link>
                        </DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
            </div>
        </div>
        <Outlet />
    </div>
}

function PendingComponent() {
    return <div className="flex-1 flex items-center justify-center gap-1">
        <div className="select-none font-bold">正在加载聊天界面</div>
        <div className="icon-[line-md--loading-loop] w-6 h-6" />
    </div>
}
