import type { ResourceInfo } from "./ResourceLoading/types";
import { mainResource } from "./resources/main";
import { fontResource } from "./resources/font";
import { storageResource } from "./resources/storage";

export const resourceDefinitions: readonly ResourceInfo[] = [
  mainResource(),
  fontResource(),
  storageResource(),
];
