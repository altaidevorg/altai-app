import {
  SelectionAskAi as SelectionAskAiView,
  type SelectionAskAiProps as SharedProps,
} from "@altai/agent-ui";
import { fmtShortcut, MOD_KEY } from "@/lib/platform";

export type SelectionAskAiProps = Omit<SharedProps, "shortcutLabel" | "viewportWidth">;

/**
 * Desktop adapter: injects platform shortcut labeling for the shared
 * selection popup.
 */
export function SelectionAskAi(props: SelectionAskAiProps) {
  return (
    <SelectionAskAiView
      {...props}
      shortcutLabel={fmtShortcut(MOD_KEY, "L")}
    />
  );
}
