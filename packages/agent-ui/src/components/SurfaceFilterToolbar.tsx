import { SurfaceSearch, SurfaceTabs } from "./AuxiliarySurface.js";

export type SurfaceFilterTab = {
  id: string;
  label: string;
  count?: number;
};

export type SurfaceFilterToolbarProps = {
  query: string;
  onQueryChange: (value: string) => void;
  searchPlaceholder: string;
  tabsLabel: string;
  tabValue: string;
  onTabChange: (value: string) => void;
  tabs: SurfaceFilterTab[];
};

/**
 * Shared search + filter-tabs strip for Work Runs / Scheduled list views.
 * Host owns query/filter state and counts.
 */
export function SurfaceFilterToolbar({
  query,
  onQueryChange,
  searchPlaceholder,
  tabsLabel,
  tabValue,
  onTabChange,
  tabs,
}: SurfaceFilterToolbarProps) {
  return (
    <div className="altai-surface-filter-toolbar shrink-0 space-y-2 border-b border-border-subtle bg-card px-3 py-2.5">
      <SurfaceSearch
        value={query}
        onChange={onQueryChange}
        placeholder={searchPlaceholder}
        className="w-full"
      />
      <SurfaceTabs
        label={tabsLabel}
        value={tabValue}
        onChange={onTabChange}
        items={tabs}
        className="border-0 bg-transparent p-0"
      />
    </div>
  );
}
