export type FrameLimits = {
  maxHeaderBytes: number;
  maxFrameBytes: number;
};

export const DEFAULT_FRAME_LIMITS: FrameLimits = {
  maxHeaderBytes: 8 * 1024,
  maxFrameBytes: 4 * 1024 * 1024,
};

export class FrameError extends Error {
  constructor(public readonly kind: "header_too_large" | "invalid_header" | "missing_content_length" | "duplicate_content_length" | "invalid_content_length" | "frame_too_large") {
    super(kind);
  }
}

/** Browser- and Node-safe incremental LSP-style framing decoder. */
export class FrameDecoder {
  private buffer = new Uint8Array();

  constructor(private readonly limits: FrameLimits = DEFAULT_FRAME_LIMITS) {}

  push(bytes: Uint8Array): Uint8Array[] {
    this.buffer = concat(this.buffer, bytes);
    const frames: Uint8Array[] = [];
    for (;;) {
      const headerEnd = findHeaderEnd(this.buffer);
      if (headerEnd === -1) {
        if (this.buffer.length > this.limits.maxHeaderBytes) throw new FrameError("header_too_large");
        return frames;
      }
      if (headerEnd > this.limits.maxHeaderBytes) throw new FrameError("header_too_large");
      const length = parseContentLength(this.buffer.slice(0, headerEnd));
      if (length > this.limits.maxFrameBytes) throw new FrameError("frame_too_large");
      const bodyStart = headerEnd + 4;
      if (this.buffer.length < bodyStart + length) return frames;
      frames.push(this.buffer.slice(bodyStart, bodyStart + length));
      this.buffer = this.buffer.slice(bodyStart + length);
    }
  }
}

export function encodeFrame(body: Uint8Array): Uint8Array {
  return concat(new TextEncoder().encode(`Content-Length: ${body.length}\r\n\r\n`), body);
}

function parseContentLength(header: Uint8Array): number {
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(header);
  } catch {
    throw new FrameError("invalid_header");
  }
  let contentLength: number | undefined;
  for (const line of text.split("\r\n")) {
    const separator = line.indexOf(":");
    if (separator === -1) throw new FrameError("invalid_header");
    if (line.slice(0, separator).toLowerCase() !== "content-length") continue;
    if (contentLength !== undefined) throw new FrameError("duplicate_content_length");
    const value = line.slice(separator + 1).trim();
    if (!/^\d+$/.test(value)) throw new FrameError("invalid_content_length");
    contentLength = Number(value);
    if (!Number.isSafeInteger(contentLength)) throw new FrameError("invalid_content_length");
  }
  if (contentLength === undefined) throw new FrameError("missing_content_length");
  return contentLength;
}

function findHeaderEnd(bytes: Uint8Array): number {
  for (let index = 0; index <= bytes.length - 4; index += 1) {
    if (bytes[index] === 13 && bytes[index + 1] === 10 && bytes[index + 2] === 13 && bytes[index + 3] === 10) return index;
  }
  return -1;
}

function concat(first: Uint8Array, second: Uint8Array): Uint8Array {
  const combined = new Uint8Array(first.length + second.length);
  combined.set(first);
  combined.set(second, first.length);
  return combined;
}
