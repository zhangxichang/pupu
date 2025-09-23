import { IndexedDB } from "./indexeddb";

declare global {
    var config: Config;
}
export class Config {
    idb: IndexedDB;

    private constructor(idb: IndexedDB) {
        this.idb = idb;
    }
    static async init() {
        return new Config(await IndexedDB.new("config", 1, (e, idb) => {
            if (e.oldVersion === 0) {
                idb.createObjectStore("config", { autoIncrement: true });
            }
        }));
    }
}
