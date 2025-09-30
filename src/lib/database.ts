import Dexie from "dexie";

export class Database {
    private dexie: Dexie;

    private constructor(dexie: Dexie) {
        this.dexie = dexie;
    }
    static async new() {
        const dexie = new Dexie("database");
        dexie.version(1).stores({
            accounts: "&id,name,key,avatar"
        });
        return new Database(await dexie.open());
    }
    async add(table: string, value: any) {
        await this.dexie.table(table).add(value);
    }
    async put(table: string, value: any) {
        await this.dexie.table(table).put(value);
    }
    async get(table: string, id: string) {
        return await this.dexie.table<Record<string, any>>(table).get(id);
    }
    async get_all(table: string) {
        return await this.dexie.table<Record<string, any>>(table).toArray();
    }
    async delete_record(table: string, id: string) {
        await this.dexie.table(table).delete(id);
    }
    async clear_all_table() {
        for (const table of this.dexie.tables) {
            await table.clear();
        }
    }
    async query(table: string, where: string, equals: string) {
        return await this.dexie.table<Record<string, any>>(table).where(where).equals(equals).first();
    }
}
