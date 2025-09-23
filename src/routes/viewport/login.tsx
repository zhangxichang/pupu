import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Database } from "@/database";
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

interface Account {
    id: string;
    name: string;
}

export const Route = createFileRoute("/viewport/login")({
    component: Component,
    pendingComponent: Loading,
    loader: async () => {
        window.database = await Database.init();
    }
});
function Component() {
    const [is_login, set_is_login] = useState(true);
    const [accounts, set_accounts] = useState<Array<Account>>([]);
    return <>
        <div className="flex-1 flex items-center justify-center">
            <Card className="w-72">
                <CardHeader>
                    <CardTitle>{is_login ? "登录你的账户" : "注册你的账户"}</CardTitle>
                    <CardDescription>{is_login ? "请在下方选择你的账户进行登录" : "输入你的账户信息进行注册"}</CardDescription>
                    <CardAction>
                        <Button variant="link" onClick={() => set_is_login(!is_login)}>{is_login ? "注册" : "登录"}</Button>
                    </CardAction>
                </CardHeader>
                <CardContent>
                    {is_login ? <Select>
                        <SelectTrigger className="w-full">
                            <SelectValue placeholder="选择账户" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectGroup>
                                <SelectLabel>账户</SelectLabel>
                                {accounts.map((value) => <SelectItem key={value.id} value={value.id}>{value.name}</SelectItem>)}
                            </SelectGroup>
                        </SelectContent>
                    </Select> : <div className="flex flex-col gap-2">
                        <Label>用户名</Label>
                        <Input placeholder="输入用户名" />
                    </div>}
                </CardContent>
                <CardFooter>
                    <Button className="w-full" onClick={() => { }}>{is_login ? "登录" : "注册"}</Button>
                </CardFooter>
            </Card>
        </div>
    </>;
}
function Loading() {
    return <>
        <div className="flex-1 flex items-center justify-center gap-1 bg-neutral-100">
            <div className="select-none font-bold">初始化中</div>
            <div className="icon-[line-md--loading-loop] w-6 h-6" />
        </div>
    </>;
}
