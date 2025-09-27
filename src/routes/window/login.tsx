import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from "@/components/ui/select";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useMemo, useState } from "react";
import { useLiveQuery } from "dexie-react-hooks";
import { Database } from "@/database";
import { Account, Network } from "@zhangxichang/network";
import { toast } from "sonner";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

declare global {
    var database: Database | null;
    var network: Network | null;
}

export const Route = createFileRoute("/window/login")({
    component: Component,
    pendingComponent: pendingComponent,
    loader: async () => {
        try {
            if (!window.database) {
                window.database = await Database.init();
            }
        } catch (error) { toast(`${error}`); }
    }
});
function Component() {
    const [card_type, set_card_type] = useState("login");
    const [selected_account_id, set_selected_account_id] = useState("");
    const [input_account_name, set_input_account_name] = useState("");
    const accounts = useLiveQuery(() => database!.get_all<Account>("accounts"));
    const navigate = useNavigate();
    const form_completed = useMemo(() => {
        switch (card_type) {
            case "login": return selected_account_id !== "";
            case "register": return input_account_name !== "";
        }
        return false;
    }, [card_type, selected_account_id, input_account_name]);
    const [is_disabled_main_button, set_is_disabled_main_button] = useState(form_completed);
    return <>
        <div className="flex-1 flex items-center justify-center">
            <Tabs value={card_type} onValueChange={set_card_type}>
                <TabsList>
                    <TabsTrigger value="login">登录</TabsTrigger>
                    <TabsTrigger value="register">注册</TabsTrigger>
                </TabsList>
                <Card className="w-72">
                    <CardHeader>
                        <CardTitle>
                            <TabsContent value="login">登录你的账户</TabsContent>
                            <TabsContent value="register">注册你的账户</TabsContent>
                        </CardTitle>
                        <CardDescription>
                            <TabsContent value="login">请在下方选择你的账户进行登录</TabsContent>
                            <TabsContent value="register">输入你的账户信息进行注册</TabsContent>
                        </CardDescription>
                    </CardHeader>
                    <CardContent>
                        <TabsContent value="login">
                            <div className="flex flex-col gap-2">
                                <Label>账户</Label>
                                <Select value={selected_account_id} onValueChange={set_selected_account_id}>
                                    <SelectTrigger className="w-full">
                                        <SelectValue placeholder="选择账户" />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectGroup>
                                            <SelectLabel>账户</SelectLabel>
                                            {accounts?.map((value) => <SelectItem key={value.id} value={value.id}>{value.name}</SelectItem>)}
                                        </SelectGroup>
                                    </SelectContent>
                                </Select>
                            </div>
                        </TabsContent>
                        <TabsContent value="register">
                            <div className="flex flex-col gap-2">
                                <Label>用户名</Label>
                                <Input value={input_account_name} onChange={(e) => set_input_account_name(e.target.value)} placeholder="输入用户名" />
                            </div>
                        </TabsContent>
                    </CardContent>
                    <CardFooter>
                        <Button disabled={!form_completed || is_disabled_main_button} className="w-full" onClick={async () => {
                            set_is_disabled_main_button(true);
                            try {
                                switch (card_type) {
                                    case "login": {
                                        window.network = await Network.new(Account.from_json(await database!.get("accounts", selected_account_id)));
                                        await navigate({ to: "/window/chat" });
                                        break;
                                    }
                                    case "register": {
                                        await database!.add("accounts", Account.new(input_account_name).json());
                                        set_input_account_name("");
                                        break;
                                    }
                                }
                            } catch (error) { toast(`${error}`); }
                            set_is_disabled_main_button(false);
                        }}>
                            <TabsContent value="login">登录</TabsContent>
                            <TabsContent value="register">注册</TabsContent>
                        </Button>
                    </CardFooter>
                </Card>
            </Tabs>
        </div>
    </>;
}
function pendingComponent() {
    return <>
        <div className="flex-1 flex items-center justify-center gap-1">
            <div className="select-none font-bold">正在加载登录界面</div>
            <div className="icon-[line-md--loading-loop] w-6 h-6" />
        </div>
    </>;
}
