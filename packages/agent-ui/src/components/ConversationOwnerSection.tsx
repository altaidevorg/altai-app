import type { ReactNode } from "react";
import { SurfaceSectionHeader } from "./AuxiliarySurface.js";

export type ConversationOwnerSectionProps = {
  picker: ReactNode;
  children?: ReactNode;
  title?: string;
  description?: string;
  runInLabel?: string;
};

/**
 * Automations create-form conversation owner chrome. Host supplies the chat
 * picker (Radix/menu) and footer actions as children.
 */
export function ConversationOwnerSection({
  picker,
  children,
  title = "Conversation",
  description = "The automation continues with the context of its owning chat.",
  runInLabel = "Run in",
}: ConversationOwnerSectionProps) {
  return (
    <section className="px-3.5 py-3.5">
      <SurfaceSectionHeader title={title} description={description} />
      <div className="mt-3 flex items-center gap-2">
        <span className="text-[10px] text-muted-foreground">{runInLabel}</span>
        {picker}
      </div>
      {children}
    </section>
  );
}
