import { Loading } from "@/components/loading";
import { Form, FormControl, FormField, FormItem } from "@/components/ui/form";
import type { UserInfo } from "@/lib/types";
import { zodResolver } from "@hookform/resolvers/zod";
import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useMemo } from "react";
import { useForm } from "react-hook-form";
import Textarea from "react-textarea-autosize";
import z from "zod";

export const Route = createFileRoute("/app/home/$user_id/chat/$friend_id")({
  component: Component,
  pendingComponent: () => <Loading hint_text="正在初始化聊天栏" />,
  beforeLoad: async ({ context, params }) => {
    const friend = (
      await context.db.query<UserInfo>(
        `SELECT name,avatar,bio FROM friends WHERE id='${params.friend_id}' LIMIT 1`,
      )
    )[0];
    console.info(friend);
    // if (!context.chat_connections.get(params.friend_id)) {
    //   (async () => {
    //     const connection = await context.endpoint.request_chat(
    //       params.friend_id,
    //     );
    //     if (!connection) return;
    //     context.chat_connections.set(params.friend_id, connection);
    //     while (true) {
    //       const connection = context.chat_connections.get(params.friend_id);
    //       if (!connection) break;
    //       const message = await connection.read();
    //       if (!message) break;
    //       context.dexie.chat_records.add({
    //         timestamp: Date.now(),
    //         sender: params.friend_id,
    //         receiver: params.user_id,
    //         message,
    //       });
    //     }
    //     context.chat_connections.delete(params.friend_id);
    //   })();
    // }
  },
});
function Component() {
  // const context = Route.useRouteContext();
  // const params = Route.useParams();
  // const [connection, set_connection] = useState<Connection | undefined>(
  //   context.chat_connections.get(params.friend_id),
  // );
  // const [connection_type, set_connection_type] = useState<
  //   ConnectionType | undefined
  // >(context.endpoint.connection_type(params.friend_id));
  // const [connection_latency, set_connection_latency] = useState<
  //   number | undefined
  // >(context.endpoint.latency(params.friend_id));
  //聊天记录
  // const chat_messages = useLiveQuery(() =>
  //   context.dexie.chat_records
  //     .orderBy("timestamp")
  //     .filter(
  //       (value) =>
  //         value.sender === params.user_id ||
  //         value.receiver === params.user_id ||
  //         value.sender === params.friend_id ||
  //         value.receiver === params.friend_id,
  //     )
  //     .toArray(),
  // );
  //聊天消息列表
  // const chat_message_list_ref = useRef(null);
  // const chat_message_virtualizer = useVirtualizer({
  //   getScrollElement: () => chat_message_list_ref.current,
  //   count: chat_messages?.length ?? 0,
  //   estimateSize: () => 60,
  // });
  // const chat_message_items = chat_message_virtualizer.getVirtualItems();
  //发送消息表单规则
  const send_message_form_schema = useMemo(
    () =>
      z.object({
        message: z.string().min(1),
      }),
    [],
  );
  //发送消息表单
  const send_message_form = useForm<z.infer<typeof send_message_form_schema>>({
    resolver: zodResolver(send_message_form_schema),
    defaultValues: {
      message: "",
    },
  });
  //自动滚动到最新消息
  // useEffect(() => {
  //   if (chat_messages?.length) {
  //     if (
  //       chat_message_virtualizer.getVirtualIndexes().at(-1) ===
  //       chat_messages.length - 1
  //     ) {
  //       chat_message_virtualizer.scrollToIndex(Infinity);
  //     }
  //   }
  // }, [chat_messages?.length]);
  //每次渲染时自动获取聊天输入框焦点
  useEffect(() => send_message_form.setFocus("message"));
  //监听连接状态变化
  // useEffect(() => {
  //   context.chat_connections.listen_set(params.friend_id, (connection) => {
  //     set_connection(connection);
  //   });
  //   context.chat_connections.listen_delete(params.friend_id, () =>
  //     set_connection(undefined),
  //   );
  //   const update_connection_type_task = setInterval(
  //     () =>
  //       set_connection_type(context.endpoint.connection_type(params.friend_id)),
  //     1000,
  //   );
  //   const update_connection_latency_task = setInterval(
  //     () => set_connection_latency(context.endpoint.latency(params.friend_id)),
  //     1000,
  //   );
  //   return () => {
  //     clearInterval(update_connection_type_task);
  //     clearInterval(update_connection_latency_task);
  //   };
  // }, []);
  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="flex items-center px-2 py-1 gap-1 border-b">
        {/*<Avatar>
          <AvatarImage src={context.friend.avatar_url} />
          <AvatarFallback className="select-none">
            {context.friend.name.at(0)}
          </AvatarFallback>
        </Avatar>
        <Button variant={"link"} className="p-0 font-bold">
          {context.friend.name}
        </Button>*/}
        <div className="flex gap-1">
          {/*{!connection && <WifiOff className="size-5 text-red-700" />}
          {connection_type &&
            connection_type !== "None" &&
            (() => {
              if (connection_type === "Direct") {
                return <Radio className="size-5 text-green-700" />;
              } else if (connection_type === "Relay") {
                return <RadioTower className="size-5 text-yellow-600" />;
              } else if (connection_type === "Mixed") {
                return <Waypoints className="size-5 text-blue-500" />;
              }
            })()}
          {connection_latency &&
            (() => {
              if (connection_latency < 100) {
                return <Signal className="size-5 text-green-700" />;
              } else if (connection_latency < 200) {
                return <SignalHigh className="size-5 text-yellow-600" />;
              } else if (connection_latency < 400) {
                return <SignalMedium className="size-5 text-red-700" />;
              } else if (connection_latency < 800) {
                return <SignalLow className="size-5 text-red-700" />;
              } else if (connection_latency < 1600) {
                return <SignalZero className="size-5 text-red-700" />;
              }
            })()}*/}
        </div>
      </div>
      <div className="flex-1 flex flex-col p-2 min-h-0 gap-1">
        {/*<div ref={chat_message_list_ref} className="flex-1 overflow-y-auto">
          <div
            className="w-full relative"
            style={{ height: chat_message_virtualizer.getTotalSize() }}
          >
            <div
              className="absolute top-0 left-0 w-full"
              style={{
                transform: `translateY(${chat_message_items.at(0)?.start}px)`,
              }}
            >
              {chat_messages &&
                chat_message_items.map((value) => (
                  <div
                    ref={chat_message_virtualizer.measureElement}
                    key={value.key}
                    data-index={value.index}
                    className="flex gap-1"
                  >
                    <Avatar className="size-10">
                      <AvatarImage
                        src={
                          (chat_messages[value.index].sender ===
                            params.user_id &&
                            context.user.avatar_url) ||
                          (chat_messages[value.index].sender ===
                            params.friend_id &&
                            context.friend.avatar_url) ||
                          undefined
                        }
                      />
                      <AvatarFallback className="select-none">
                        {(chat_messages[value.index].sender ===
                          params.user_id &&
                          context.user.name.at(0)) ||
                          (chat_messages[value.index].sender ===
                            params.friend_id &&
                            context.friend.name.at(0))}
                      </AvatarFallback>
                    </Avatar>
                    <div className="flex flex-col items-start">
                      <Button
                        variant={"link"}
                        className="p-0 font-bold text-base"
                      >
                        {(chat_messages[value.index].sender ===
                          params.user_id &&
                          context.user.name) ||
                          (chat_messages[value.index].sender ===
                            params.friend_id &&
                            context.friend.name)}
                      </Button>
                      <span>{chat_messages[value.index].message}</span>
                    </div>
                  </div>
                ))}
            </div>
          </div>
        </div>*/}
        <Form {...send_message_form}>
          <FormField
            control={send_message_form.control}
            name="message"
            render={({ field }) => (
              <FormItem>
                <FormControl>
                  <Textarea
                    {...field}
                    // disabled={formState.isSubmitting || !connection}
                    maxRows={16}
                    className="resize-none border rounded p-2 focus:outline-none focus:border-neutral-200 focus:ring-1 focus:ring-neutral-200"
                    placeholder="发送消息"
                    onKeyDown={async (e) => {
                      if (e.key !== "Enter" || e.shiftKey) return;
                      e.preventDefault();
                      await send_message_form.handleSubmit(async (_form) => {
                        // await connection!.send(form.message);
                        // await context.dexie.chat_records.add({
                        //   timestamp: Date.now(),
                        //   sender: params.user_id,
                        //   receiver: params.friend_id,
                        //   message: form.message,
                        // });
                        send_message_form.reset();
                      })();
                    }}
                  />
                </FormControl>
              </FormItem>
            )}
          />
        </Form>
      </div>
    </div>
  );
}
