import { Database } from "./state/database";

export class State {
  database = new Database();

  async init() {
    await this.database.init();
  }
}
