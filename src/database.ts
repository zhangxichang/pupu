import { IndexedDB } from "./indexeddb";

declare global {
    var database: Database;
}
export class Database {
    idb: IndexedDB;

    private constructor(idb: IndexedDB) {
        this.idb = idb;
    }
    static async init() {
        return new Database(await IndexedDB.new("database", 1, (e, idb) => {
            if (e.oldVersion === 0) {
                idb.createObjectStore("accounts", { keyPath: "id" });
            }
        }));
    }
}
