import { Archive02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { COMPACT_NOW_TITLE } from "../lib/paperImportChrome.js";
import { IconBtn } from "./IconBtn.js";

export type CompactNowControlProps = {
  disabled?: boolean;
  onClick: () => void;
};

/**
 * Status-bar control that triggers context compaction. Presentational; the host
 * owns busy/session gating and the `/compact` transport call.
 */
export function CompactNowControl({
  disabled,
  onClick,
}: CompactNowControlProps) {
  return (
    <IconBtn
      title={COMPACT_NOW_TITLE}
      onClick={onClick}
      disabled={disabled}
    >
      <HugeiconsIcon icon={Archive02Icon} size={13} strokeWidth={1.75} />
    </IconBtn>
  );
}
