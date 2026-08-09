import { describe, expect, it } from "vitest";
import {
  arrayBufferToBytes,
  arrayBufferViewToBytes,
  utf8StringToBytes,
  uint8ArrayToBytes,
} from "../lib/bodyBytes.js";

describe("bodyBytes", () => {
  it("encodes string", () => {
    expect(utf8StringToBytes("hi")).toEqual([104, 105]);
  });
  it("copies ArrayBuffer", () => {
    const buf = new Uint8Array([1, 2, 3]).buffer;
    expect(arrayBufferToBytes(buf)).toEqual([1, 2, 3]);
  });
  it("copies ArrayBufferView", () => {
    expect(arrayBufferViewToBytes(new Uint8Array([4, 5]))).toEqual([4, 5]);
  });
  it("maps Uint8Array", () => {
    expect(uint8ArrayToBytes(new Uint8Array([9, 8]))).toEqual([9, 8]);
  });
});
