use std::rc::Rc;

use slint::{ModelRc, VecModel};

use crate::{ConversationItemData, GroupMemberData, MessageData, MessageKind, Viewport};

const PRIVATE_NAMES: [(&str, &str); 40] = [
    ("张伟", "张"),
    ("李娜", "李"),
    ("王强", "王"),
    ("刘洋", "刘"),
    ("陈静", "陈"),
    ("杨帆", "杨"),
    ("赵磊", "赵"),
    ("孙丽", "孙"),
    ("周杰", "周"),
    ("吴敏", "吴"),
    ("郑爽", "郑"),
    ("林晨", "林"),
    ("何雨", "何"),
    ("高鹏", "高"),
    ("马超", "马"),
    ("朱琳", "朱"),
    ("胡军", "胡"),
    ("郭婷", "郭"),
    ("罗浩", "罗"),
    ("梁雪", "梁"),
    ("宋涛", "宋"),
    ("唐悦", "唐"),
    ("许峰", "许"),
    ("邓丽", "邓"),
    ("冯磊", "冯"),
    ("曹阳", "曹"),
    ("彭飞", "彭"),
    ("曾敏", "曾"),
    ("肖楠", "肖"),
    ("田甜", "田"),
    ("董强", "董"),
    ("袁莉", "袁"),
    ("潘伟", "潘"),
    ("于洋", "于"),
    ("蒋欣", "蒋"),
    ("蔡明", "蔡"),
    ("余乐", "余"),
    ("杜娟", "杜"),
    ("叶青", "叶"),
    ("程勇", "程"),
];
const PRIVATE_TIMES: [&str; 40] = [
    "刚刚",
    "10:38",
    "10:24",
    "09:58",
    "昨天",
    "昨天",
    "星期三",
    "星期二",
    "7月28日",
    "7月25日",
    "18:06",
    "12:30",
    "刚刚",
    "10:38",
    "10:24",
    "09:58",
    "昨天",
    "昨天",
    "星期三",
    "星期二",
    "7月28日",
    "7月25日",
    "18:06",
    "12:30",
    "刚刚",
    "10:38",
    "10:24",
    "09:58",
    "昨天",
    "昨天",
    "星期三",
    "星期二",
    "7月28日",
    "7月25日",
    "18:06",
    "12:30",
    "刚刚",
    "10:38",
    "10:24",
    "09:58",
];
const PRIVATE_UNREAD: [i32; 40] = [
    0, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9, 1, 8, 0, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9,
    1, 8, 0, 7, 14, 6, 13, 5, 12, 4, 11, 3,
];

const GROUP_NAMES: [(&str, &str); 30] = [
    ("产品研发一群", "产"),
    ("产品研发二群", "产"),
    ("周末篮球局", "周"),
    ("相亲相爱一家人", "相"),
    ("前端技术交流群", "前"),
    ("公司公告通知群", "公"),
    ("深夜食堂吃货群", "深"),
    ("大学室友四年群", "大"),
    ("项目组-凤凰计划", "项"),
    ("王者荣耀开黑群", "王"),
    ("小区业主交流群", "小"),
    ("设计灵感分享群", "设"),
    ("读书打卡群", "读"),
    ("跑步训练营", "跑"),
    ("摄影爱好者", "摄"),
    ("周末露营群", "周"),
    ("咖啡品鉴群", "咖"),
    ("猫友交流群", "猫"),
    ("狗友交流群", "狗"),
    ("电影讨论群", "电"),
    ("音乐分享群", "音"),
    ("旅行结伴群", "旅"),
    ("健身打卡群", "健"),
    ("游戏开发群", "游"),
    ("Rust 学习小组", "R"),
    ("Slint 交流群", "S"),
    ("开源项目协作群", "开"),
    ("远程办公交流群", "远"),
    ("副业交流群", "副"),
    ("二手交易群", "二"),
];
const GROUP_TIMES: [&str; 30] = [
    "刚刚",
    "10:38",
    "10:24",
    "09:58",
    "昨天",
    "昨天",
    "星期三",
    "星期二",
    "7月28日",
    "7月25日",
    "18:06",
    "12:30",
    "刚刚",
    "10:38",
    "10:24",
    "09:58",
    "昨天",
    "昨天",
    "星期三",
    "星期二",
    "7月28日",
    "7月25日",
    "18:06",
    "12:30",
    "刚刚",
    "10:38",
    "10:24",
    "09:58",
    "昨天",
    "昨天",
];
const GROUP_UNREAD: [i32; 30] = [
    0, 5, 10, 15, 20, 2, 7, 12, 17, 22, 4, 9, 14, 19, 1, 6, 11, 16, 21, 3, 8, 13, 18, 0, 5, 10, 15,
    20, 2, 7,
];

const MEMBER_NAMES: [(&str, &str); 11] = [
    ("张伟", "张"),
    ("李娜", "李"),
    ("王强", "王"),
    ("刘洋", "刘"),
    ("陈静", "陈"),
    ("杨帆", "杨"),
    ("赵磊", "赵"),
    ("孙丽", "孙"),
    ("周杰", "周"),
    ("吴敏", "吴"),
    ("郑爽", "郑"),
];

const MESSAGE_POOL: [&str; 28] = [
    "晚上一起吃饭？",
    "这个方案怎么样？",
    "稍等，我查一下",
    "记得带电脑",
    "好，就按这个来",
    "文件已经发你了",
    "早点休息",
    "这个版本可以发布",
    "好的，没问题",
    "我再想想",
    "已经处理好了",
    "路上注意安全",
    "在吗？",
    "谢谢！",
    "我同意",
    "再改一版吧",
    "收到",
    "可以的",
    "没问题，交给我",
    "到了说一声",
    "今天有空吗？",
    "刚看到消息",
    "下午三点开会",
    "测试通过了",
    "我晚点回复你",
    "辛苦了",
    "哈哈哈",
    "明天见",
];

const FILE_NAMES: [&str; 4] = ["海报.jpg", "截图.png", "设计稿.pdf", "报告.docx"];

fn model<T: Clone + 'static>(rows: Vec<T>) -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::from(rows)))
}

fn thread(messages: Vec<MessageData>) -> ModelRc<MessageData> {
    model(messages)
}

fn time_for(index: usize) -> slint::SharedString {
    let minutes = 8 * 60 + index * 3;
    format!("{:02}:{:02}", minutes / 60, minutes % 60).into()
}

fn text_message(
    id: i32,
    sender: &str,
    avatar: &str,
    time: slint::SharedString,
    content: &str,
    is_self: bool,
) -> MessageData {
    MessageData {
        id,
        sender_name: sender.into(),
        avatar_text: avatar.into(),
        time,
        content: content.into(),
        kind: MessageKind::Text,
        is_self,
        read: is_self,
        ..Default::default()
    }
}

fn special_message(
    id: i32,
    sender: &str,
    avatar: &str,
    time: slint::SharedString,
    kind: MessageKind,
    file_name: &str,
    is_self: bool,
) -> MessageData {
    MessageData {
        id,
        sender_name: sender.into(),
        avatar_text: avatar.into(),
        time,
        file_name: file_name.into(),
        file_size: "412 KB".into(),
        kind,
        is_self,
        read: is_self,
        ..Default::default()
    }
}

fn preview(message: &MessageData) -> String {
    if !message.content.is_empty() {
        return message.content.to_string();
    }
    if matches!(message.kind, MessageKind::File) {
        message.file_name.to_string()
    } else {
        "图片".to_string()
    }
}

fn private_data() -> (Vec<ModelRc<MessageData>>, Vec<ConversationItemData>) {
    let mut threads = Vec::with_capacity(PRIVATE_NAMES.len());
    let mut conversations = Vec::with_capacity(PRIVATE_NAMES.len());
    for (index, (name, avatar)) in PRIVATE_NAMES.iter().enumerate() {
        let count = if index % 2 == 0 { 30 } else { 31 };
        let mut messages = Vec::with_capacity(count);
        for message_index in 0..count {
            let id = message_index as i32 + 1;
            let time = time_for(message_index);
            let is_self = message_index % 2 == 1;
            let (sender, sender_avatar) = if is_self {
                ("我", "我")
            } else {
                (*name, *avatar)
            };
            let message = if message_index == 7 || message_index == 23 {
                special_message(
                    id,
                    sender,
                    sender_avatar,
                    time,
                    MessageKind::File,
                    FILE_NAMES[(index + message_index) % FILE_NAMES.len()],
                    is_self,
                )
            } else if message_index == 15 {
                special_message(
                    id,
                    sender,
                    sender_avatar,
                    time,
                    MessageKind::Image,
                    "",
                    is_self,
                )
            } else {
                text_message(
                    id,
                    sender,
                    sender_avatar,
                    time,
                    MESSAGE_POOL[(index * 3 + message_index) % MESSAGE_POOL.len()],
                    is_self,
                )
            };
            messages.push(message);
        }
        let last = messages.last().unwrap();
        let latest_message = if last.is_self {
            format!("我：{}", preview(last))
        } else {
            preview(last)
        };
        conversations.push(ConversationItemData {
            name: (*name).into(),
            avatar_text: (*avatar).into(),
            latest_message: latest_message.into(),
            time: PRIVATE_TIMES[index].into(),
            unread_count: PRIVATE_UNREAD[index],
            ..Default::default()
        });
        threads.push(thread(messages));
    }
    (threads, conversations)
}

fn group_data() -> (Vec<ModelRc<MessageData>>, Vec<ConversationItemData>) {
    let mut threads = Vec::with_capacity(GROUP_NAMES.len());
    let mut conversations = Vec::with_capacity(GROUP_NAMES.len());
    for (index, (name, avatar)) in GROUP_NAMES.iter().enumerate() {
        let mut messages = Vec::with_capacity(30);
        for message_index in 0..30 {
            let id = message_index as i32 + 1;
            let time = time_for(message_index);
            let is_self = message_index % 4 == 3;
            let (sender, sender_avatar) = if is_self {
                ("我", "我")
            } else {
                MEMBER_NAMES[(index + message_index) % MEMBER_NAMES.len()]
            };
            let message = if message_index == 7 || message_index == 23 {
                special_message(
                    id,
                    sender,
                    sender_avatar,
                    time,
                    MessageKind::File,
                    FILE_NAMES[(index + message_index) % FILE_NAMES.len()],
                    is_self,
                )
            } else if message_index == 15 {
                special_message(
                    id,
                    sender,
                    sender_avatar,
                    time,
                    MessageKind::Image,
                    "",
                    is_self,
                )
            } else {
                text_message(
                    id,
                    sender,
                    sender_avatar,
                    time,
                    MESSAGE_POOL[(index * 5 + message_index) % MESSAGE_POOL.len()],
                    is_self,
                )
            };
            messages.push(message);
        }
        let last = messages.last().unwrap();
        let latest_message = format!("{}：{}", last.sender_name, preview(last));
        conversations.push(ConversationItemData {
            name: (*name).into(),
            avatar_text: (*avatar).into(),
            latest_message: latest_message.into(),
            time: GROUP_TIMES[index].into(),
            unread_count: GROUP_UNREAD[index],
            ..Default::default()
        });
        threads.push(thread(messages));
    }
    (threads, conversations)
}

fn group_member_data() -> Vec<ModelRc<GroupMemberData>> {
    let mut groups = Vec::with_capacity(GROUP_NAMES.len());
    for group_index in 0..GROUP_NAMES.len() {
        let mut members = Vec::with_capacity(MEMBER_NAMES.len() + 1);
        for member_index in 0..MEMBER_NAMES.len() {
            let (name, avatar) = MEMBER_NAMES[(group_index + member_index) % MEMBER_NAMES.len()];
            let role = if member_index == 0 {
                "群主"
            } else if member_index <= 2 {
                "管理员"
            } else {
                ""
            };
            members.push(GroupMemberData {
                name: name.into(),
                avatar_text: avatar.into(),
                role: role.into(),
                online: (group_index + member_index) % 3 != 0,
                ..Default::default()
            });
        }
        members.push(GroupMemberData {
            name: "我".into(),
            avatar_text: "我".into(),
            role: "".into(),
            online: true,
            ..Default::default()
        });
        groups.push(model(members));
    }
    groups
}

pub fn setup(viewport: &Viewport) {
    let (private_threads, private_conversations) = private_data();
    let (group_threads, group_conversations) = group_data();
    let group_members = group_member_data();
    viewport.set_private_conversations(model(private_conversations));
    viewport.set_private_threads(model(private_threads));
    viewport.set_group_conversations(model(group_conversations));
    viewport.set_group_threads(model(group_threads));
    viewport.set_group_members(model(group_members));
}
