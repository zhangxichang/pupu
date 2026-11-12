import type { ResourceInfo } from "./Root/ResourceLoading/types";
import { mainResource } from "./resources/main";
import { fontResource } from "./resources/font";
import { storageResource } from "./resources/storage";

export const resourceDefinitions: readonly ResourceInfo[] = [
  mainResource(),
  fontResource(),
  storageResource(),
];
