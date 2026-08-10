import { memo, type ReactNode } from "react";
import { Streamdown, type Components } from "streamdown";
import { cn } from "../lib/cn.js";

export type AgentMarkdownLink = {
  href: string;
  children: ReactNode;
};

export type AgentMarkdownProps = {
  content: string;
  streaming?: boolean;
  className?: string;
  /** Optional host-specific renderers, such as Desktop code cards. */
  components?: Components;
  /** Hosts keep privileged file and external-link actions behind their ports. */
  renderLink?: (link: AgentMarkdownLink) => ReactNode;
};

function safeMarkdownUrl(value: string): string | null {
  const href = value.trim();
  if (!href || href.startsWith("#")) {
    return href || null;
  }
  try {
    const parsed = new URL(href, "https://altai.invalid");
    if (
      parsed.protocol === "https:" ||
      parsed.protocol === "http:" ||
      parsed.protocol === "mailto:" ||
      parsed.protocol === "file:"
    ) {
      return href;
    }
  } catch {
    // Streamdown will render the text rather than a navigable link.
  }
  return null;
}

/**
 * Shared, sanitized GFM renderer for both ALTAI hosts.
 *
 * Raw HTML is always skipped. Hosts inject any privileged link handling so
 * neither the renderer nor the Webview receives filesystem authority.
 */
export const AgentMarkdown = memo(function AgentMarkdown({
  content,
  streaming = false,
  className,
  components,
  renderLink,
}: AgentMarkdownProps) {
  const markdownComponents: Components | undefined = renderLink
    ? {
        ...components,
        a: ({ href, children }) => (
          <>{renderLink({ href: href ?? "", children })}</>
        ),
      }
    : components;

  return (
    <Streamdown
      className={cn(
        "size-full min-w-0 max-w-full [overflow-wrap:anywhere] [word-break:break-word] [&>*:first-child]:mt-0 [&>*:last-child]:mb-0",
        className,
      )}
      components={markdownComponents}
      mode={streaming ? "streaming" : "static"}
      parseIncompleteMarkdown={streaming}
      skipHtml
      urlTransform={safeMarkdownUrl}
    >
      {content}
    </Streamdown>
  );
});
