import { describe, expect, it } from "vitest";
import {
  arrayBufferToBytes,
  arrayBufferViewToBytes,
  utf8StringToBytes,
} from "../lib/bodyBytes.js";

describe("body bytes helpers", () => {
  it("encodes utf8 strings", () => {
    expect(utf8StringToBytes("ab")).toEqual([97, 98]);
  });

  it("copies ArrayBuffer and views", () => {
    const buf = new Uint8Array([1, 2, 3]).buffer;
    expect(arrayBufferToBytes(buf)).toEqual([1, 2, 3]);
    const view = new Uint8Array(buf, 1, 2);
    expect(arrayBufferViewToBytes(view)).toEqual([2, 3]);
  });
});
