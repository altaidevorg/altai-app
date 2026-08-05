import { AutomationCard, type AutomationCardProps } from "./AutomationCard.js";
import { SurfaceListGroup } from "./SurfaceListGroup.js";

export type AutomationListItem = Omit<AutomationCardProps, "className"> & {
  id: string;
};

export type AutomationListProps = {
  items: AutomationListItem[];
  title?: string;
  description?: string;
  ariaLabel?: string;
};

/**
 * Presentational Scheduled-work list. Hosts resolve schedule/job labels and
 * inject all navigation and mutation callbacks.
 */
export function AutomationList({
  items,
  title = "Workspace schedules",
  description = "Ordered by the next expected run",
  ariaLabel = "Workspace automations",
}: AutomationListProps) {
  return (
    <SurfaceListGroup
      title={title}
      description={description}
      count={items.length}
      containerAs="ul"
      containerAriaLabel={ariaLabel}
    >
      {items.map(({ id, ...item }, index) => (
        <AutomationCard
          key={id}
          {...item}
          className={index > 0 ? "border-t border-border-subtle" : undefined}
        />
      ))}
    </SurfaceListGroup>
  );
}
