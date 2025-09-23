export class IndexedDB {
    idb: IDBDatabase;

    private constructor(idb: IDBDatabase) {
        this.idb = idb;
    }
    static async new(name: string, version: number, onupgradeneeded: (e: IDBVersionChangeEvent, idb: IDBDatabase) => void) {
        return new IndexedDB(await new Promise((resolve, reject) => {
            const request = indexedDB.open(name, version);
            request.onerror = () => reject(request.error);
            request.onsuccess = () => resolve(request.result);
            request.onupgradeneeded = (e) => onupgradeneeded(e, request.result);
        }));
    }
}
