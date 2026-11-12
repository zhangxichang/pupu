import type { ResourceInfo } from "../Root/ResourceLoading/types";
import mainFontUrl from "../../../assets/fonts/MiSans VF.ttf?url";
import mainFontSize from "../../../assets/fonts/MiSans VF.ttf?size";

let fontData: Uint8Array | undefined;

export function fontResource(): ResourceInfo {
  return {
    kind: "basic",
    id: "font",
    label: "字体",
    url: mainFontUrl,
    size: mainFontSize,
    onDownload: setFontData,
  };
}

export function setFontData(bytes: ArrayBuffer): void {
  fontData = new Uint8Array(bytes);
}

export function takeFontData(): Uint8Array {
  if (!fontData) {
    throw new Error("字体数据缺失");
  }
  const data = fontData;
  fontData = undefined;
  return data;
}
