import { Account, Network as Net } from "@zhangxichang/network";

export class Network {
    private net: Net;

    private constructor(net: Net) {
        this.net = net;
    }
    static async new(account: Account) {
        return new Network(await Net.new(account));
    }
    account() {
        return this.net.account();
    }
    async shutdown() {
        await this.net.shutdown();
    }
    async search_user(user_id: string) {
        return await this.net.search_user(user_id);
    }
}
