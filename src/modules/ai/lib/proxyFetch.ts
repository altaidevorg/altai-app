import {
  arrayBufferToBytes,
  arrayBufferViewToBytes,
  headersInitToRecord as headersInitToRecordShared,
  requestMethodFromInit,
  requestUrlToString,
  utf8StringToBytes,
  uint8ArrayToBytes,
} from "@altai/agent-ui";
import { Channel, invoke } from "@tauri-apps/api/core";

/** Streaming events emitted by the Rust `ai_http_stream` command. */
type AiStreamEvent =
  | { kind: "headers"; status: number; headers: Record<string, string> }
  | { kind: "chunk"; bytes: number[] }
  | { kind: "end" }
  | { kind: "error"; message: string };

async function bodyToBytes(
  body: BodyInit | null | undefined,
): Promise<number[] | undefined> {
  if (body == null) return undefined;
  if (typeof body === "string") {
    return utf8StringToBytes(body);
  }
  if (body instanceof ArrayBuffer) return arrayBufferToBytes(body);
  if (ArrayBuffer.isView(body)) {
    return arrayBufferViewToBytes(body as ArrayBufferView);
  }
  if (body instanceof Blob)
    return uint8ArrayToBytes(new Uint8Array(await body.arrayBuffer()));
  // FormData / URLSearchParams / ReadableStream — uncommon for AI SDK calls.
  const text = await new Response(body as BodyInit).text();
  return utf8StringToBytes(text);
}

export function createProxyFetch(
  opts: { allowPrivateNetwork?: boolean } = {},
): typeof fetch {
  const allowPrivate = opts.allowPrivateNetwork === true;
  return async (input, init) => proxyFetchImpl(input, init, allowPrivate);
}

/** Backwards-compatible default — refuses private networks unless the caller
 *  explicitly opts in via {@link createProxyFetch}. */
export const proxyFetch: typeof fetch = (input, init) =>
  proxyFetchImpl(input, init, false);

async function proxyFetchImpl(
  input: RequestInfo | URL,
  init: RequestInit | undefined,
  allowPrivateNetwork: boolean,
): Promise<Response> {
  const url = requestUrlToString(input);
  const method = requestMethodFromInit(init);
  const headers = headersInitToRecordShared(init?.headers);
  const body = await bodyToBytes(init?.body);

  const signal = init?.signal;
  if (signal?.aborted) {
    throw makeAbortError();
  }

  return new Promise<Response>((resolve, reject) => {
    let resolved = false;
    let streamController: ReadableStreamDefaultController<Uint8Array> | null =
      null;
    let cancelled = false;

    const onAbort = () => {
      cancelled = true;
      if (!resolved) {
        reject(makeAbortError());
      } else if (streamController) {
        try {
          streamController.error(makeAbortError());
        } catch {
          /* already closed */
        }
      }
    };
    signal?.addEventListener("abort", onAbort, { once: true });

    const channel = new Channel<AiStreamEvent>();
    channel.onmessage = (event) => {
      if (cancelled) return;
      switch (event.kind) {
        case "headers": {
          const stream = new ReadableStream<Uint8Array>({
            start(controller) {
              streamController = controller;
            },
            cancel() {
              cancelled = true;
            },
          });
          resolved = true;
          resolve(
            new Response(stream, {
              status: event.status,
              headers: new Headers(event.headers),
            }),
          );
          break;
        }
        case "chunk": {
          streamController?.enqueue(Uint8Array.from(event.bytes));
          break;
        }
        case "end": {
          streamController?.close();
          break;
        }
        case "error": {
          if (!resolved) {
            reject(new Error(event.message));
          } else {
            streamController?.error(new Error(event.message));
          }
          break;
        }
      }
    };

    invoke("ai_http_stream", {
      url,
      method,
      headers,
      body,
      allowPrivateNetwork,
      onEvent: channel,
    }).catch((e) => {
      if (resolved) return; // headers already arrived; chunk-side error wins
      reject(e instanceof Error ? e : new Error(String(e)));
    });
  });
}

function makeAbortError(): DOMException {
  return new DOMException("Request aborted", "AbortError");
}
