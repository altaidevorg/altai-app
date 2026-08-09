import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  setFavoriteModelIds,
  setRecentModelIds,
} from "@/modules/settings/store";
import {
  pushRecentId,
  sameIdSequence,
  toggleIdInList,
} from "@altai/agent-ui";

const RECENTS_MAX = 5;

export async function toggleFavoriteModel(id: string): Promise<void> {
  const current = usePreferencesStore.getState().favoriteModelIds;
  await setFavoriteModelIds(toggleIdInList(current, id));
}

export async function pushRecentModel(id: string): Promise<void> {
  const current = usePreferencesStore.getState().recentModelIds;
  const next = pushRecentId(current, id, RECENTS_MAX);
  if (sameIdSequence(current, next)) {
    return;
  }
  await setRecentModelIds(next);
}
