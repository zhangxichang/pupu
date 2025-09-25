import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from "@/components/ui/select";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useLiveQuery } from "dexie-react-hooks";
import { Database } from "@/database";
import { Account, Network } from "network";

declare global {
    var database: Database | null;
    var network: Network | null;
}

export const Route = createFileRoute("/window/login")({
    component: Component,
    pendingComponent: Loading,
    loader: async () => {
        if (!window.database) {
            window.database = await Database.init();
        }
    }
});
function Component() {
    const [is_login, set_is_login] = useState(true);
    const [selected_account_id, set_selected_account_id] = useState("");
    const [input_account_name, set_input_account_name] = useState("");
    const accounts = useLiveQuery(() => database!.get_all<Account>("accounts"));
    const navigate = useNavigate();
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
                    {is_login ? <Select value={selected_account_id} onValueChange={(e) => set_selected_account_id(e)}>
                        <SelectTrigger className="w-full">
                            <SelectValue placeholder="选择账户" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectGroup>
                                <SelectLabel>账户</SelectLabel>
                                {accounts?.map((value) => <SelectItem key={value.id} value={value.id}>{value.name}</SelectItem>)}
                            </SelectGroup>
                        </SelectContent>
                    </Select> : <div className="flex flex-col gap-2">
                        <Label>用户名</Label>
                        <Input value={input_account_name} onChange={(e) => set_input_account_name(e.target.value)} placeholder="输入用户名" />
                    </div>}
                </CardContent>
                <CardFooter>
                    <Button disabled={is_login ? selected_account_id === "" : input_account_name === ""} className="w-full" onClick={async () => {
                        if (is_login) {
                            window.network = await Network.new(Account.from_json(await database!.get("accounts", selected_account_id)));
                            navigate({ to: "/window/chat" });
                        } else {
                            await database!.add("accounts", Account.new(input_account_name).json());
                            set_input_account_name("");
                        }
                    }}>{is_login ? "登录" : "注册"}</Button>
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
