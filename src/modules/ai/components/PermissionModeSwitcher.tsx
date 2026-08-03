import {
  PermissionModeSwitcher as PermissionModeSwitcherView,
  type PermissionModeSwitcherProps,
} from "@altai/agent-ui";
import { openSettingsWindow } from "@/modules/settings/openSettingsWindow";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  setBypassPermissionsEnabled,
  setPermissionMode,
  type PermissionMode,
} from "@/modules/settings/store";

type Props = Pick<PermissionModeSwitcherProps, "variant">;

/**
 * Desktop adapter: binds the shared permission switcher to preferences +
 * settings window APIs.
 */
export function PermissionModeSwitcher({ variant }: Props) {
  const mode = usePreferencesStore((s) => s.permissionMode);
  const bypassEnabled = usePreferencesStore((s) => s.bypassPermissionsEnabled);

  const selectMode = (next: PermissionMode) => {
    void (async () => {
      // Selecting bypass from the toolbar also unlocks the settings gate so
      // the mode is actually effective (not silently downgraded to ask).
      if (next === "bypass" && !bypassEnabled) {
        await setBypassPermissionsEnabled(true);
      }
      await setPermissionMode(next);
    })();
  };

  return (
    <PermissionModeSwitcherView
      mode={mode}
      bypassEnabled={bypassEnabled}
      variant={variant}
      onSelectMode={selectMode}
      onManagePermissions={() => void openSettingsWindow("general")}
    />
  );
}
