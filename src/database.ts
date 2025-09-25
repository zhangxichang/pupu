import { Dexie } from "dexie";

export class Database {
    dexie: Dexie;

    private constructor(dexie: Dexie) {
        this.dexie = dexie;
    }
    static async init() {
        const dexie = new Dexie("database");
        dexie.version(1).stores({ accounts: "&id,name,key" });
        return new Database(await dexie.open());
    }
    async add<T>(title: string, value: T) {
        await this.dexie.table(title).add(value);
    }
    async put<T>(title: string, value: T) {
        await this.dexie.table(title).put(value);
    }
    async get<T>(title: string, id: string) {
        return await this.dexie.table<T>(title).get(id);
    }
    async get_all<T>(title: string) {
        return await this.dexie.table<T>(title).toArray();
    }
}
