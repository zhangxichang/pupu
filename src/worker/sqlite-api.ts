import type { SQLiteUpdateEvent } from "@/lib/sqlite";

export type Command =
  | {
      kind: "open";
      return: MessagePort;
      path: string;
    }
  | {
      kind: "close";
      return: MessagePort;
      db: number;
    }
  | {
      kind: "execute";
      return: MessagePort;
      db: number;
      sql: string;
      params?: any[];
    }
  | {
      kind: "query";
      return: MessagePort;
      db: number;
      sql: string;
      params?: any[];
    }
  | {
      kind: "on_update";
      return: MessagePort;
      db: number;
    };

export type Result<T> =
  | {
      kind: "ok";
      value: T;
    }
  | {
      kind: "error";
      error: Error;
    };

export class SQLiteAPI {
  private command: MessagePort;

  constructor(command: MessagePort) {
    this.command = command;
  }
  async open(path: string) {
    return await new Promise<number>((resolve, reject) => {
      const { port1, port2 } = new MessageChannel();
      this.command.postMessage(
        {
          kind: "open",
          return: port2,
          path,
        } satisfies Command,
        { transfer: [port2] },
      );
      port1.onmessage = (e: MessageEvent<Result<number>>) => {
        if (e.data.kind === "ok") {
          resolve(e.data.value);
        } else {
          reject(e.data.error);
        }
      };
    });
  }
  async close(db: number) {
    await new Promise((resolve, reject) => {
      const { port1, port2 } = new MessageChannel();
      this.command.postMessage(
        {
          kind: "close",
          return: port2,
          db,
        } satisfies Command,
        { transfer: [port2] },
      );
      port1.onmessage = (e: MessageEvent<Result<null>>) => {
        if (e.data.kind === "ok") {
          resolve(e.data.value);
        } else {
          reject(e.data.error);
        }
      };
    });
  }
  async execute(db: number, sql: string, params?: any[]) {
    await new Promise((resolve, reject) => {
      const { port1, port2 } = new MessageChannel();
      this.command.postMessage(
        {
          kind: "execute",
          return: port2,
          db,
          sql,
          params,
        } satisfies Command,
        { transfer: [port2] },
      );
      port1.onmessage = (e: MessageEvent<Result<null>>) => {
        if (e.data.kind === "ok") {
          resolve(e.data.value);
        } else {
          reject(e.data.error);
        }
      };
    });
  }
  async query<T>(db: number, sql: string, params?: any[]) {
    return await new Promise<ReadableStream<T>>((resolve, reject) => {
      const { port1, port2 } = new MessageChannel();
      this.command.postMessage(
        {
          kind: "query",
          return: port2,
          db,
          sql,
          params,
        } satisfies Command,
        { transfer: [port2] },
      );
      port1.onmessage = (e: MessageEvent<Result<ReadableStream<T>>>) => {
        if (e.data.kind === "ok") {
          resolve(e.data.value);
        } else {
          reject(e.data.error);
        }
      };
    });
  }
  on_update(
    db: number,
    callback: (event: SQLiteUpdateEvent) => void | Promise<void>,
  ) {
    const { port1, port2 } = new MessageChannel();
    this.command.postMessage(
      {
        kind: "on_update",
        return: port2,
        db,
      } satisfies Command,
      { transfer: [port2] },
    );
    port1.onmessage = (e: MessageEvent<SQLiteUpdateEvent>) => callback(e.data);
  }
}

export function ok<T>(value: T): Result<T> {
  return { kind: "ok", value };
}
export function err(error: Error): Result<never> {
  return { kind: "error", error };
}
