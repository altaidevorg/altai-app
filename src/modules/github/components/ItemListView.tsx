import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  type GHItem,
  type ItemKind,
  type ItemStateFilter,
  listItems,
  relativeTime,
  type RepoSlug,
} from "@/modules/github/lib/items";
import {
  ArrowDown01Icon,
  ArrowReloadHorizontalIcon,
  ArrowRight01Icon,
  Comment01Icon,
  InboxIcon,
  PlusSignIcon,
  Search01Icon,
  Tag01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useMemo, useState } from "react";
import { AssignAgentButton } from "./AssignAgentButton";
import {
  Avatar,
  ItemStateIcon,
  itemState,
  Labels,
  StateText,
} from "./itemBits";

type Props = {
  slug: RepoSlug;
  kind: ItemKind;
  onKindChange: (kind: ItemKind) => void;
  onOpenItem: (kind: ItemKind, number: number) => void;
  onCreate: (kind: ItemKind) => void;
  reloadKey: number;
};

type FocusFilter =
  | "all"
  | "ready"
  | "drafts"
  | "unlabeled"
  | "discussion";
type SortMode = "updated" | "oldest" | "discussion";

export function ItemListView({
  slug,
  kind,
  onKindChange,
  onOpenItem,
  onCreate,
  reloadKey,
}: Props) {
  const [stateFilter, setStateFilter] = useState<ItemStateFilter>("open");
  const [focusFilter, setFocusFilter] = useState<FocusFilter>("all");
  const [sortMode, setSortMode] = useState<SortMode>("updated");
  const [query, setQuery] = useState("");
  const [labelFilter, setLabelFilter] = useState("all");
  const [items, setItems] = useState<GHItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadTick, setReloadTick] = useState(0);

  useEffect(() => {
    setFocusFilter("all");
    setLabelFilter("all");
  }, [kind]);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    setError(null);
    setItems([]);
    listItems(slug, kind, stateFilter)
      .then((list) => {
        if (alive) setItems(list);
      })
      .catch((cause: unknown) => {
        if (!alive) return;
        setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [slug, kind, stateFilter, reloadTick, reloadKey]);

  const labelOptions = useMemo(() => {
    const seen = new Set<string>();
    for (const item of items) {
      for (const label of item.labels) seen.add(label.name);
    }
    return [...seen].sort((left, right) => left.localeCompare(right));
  }, [items]);

  useEffect(() => {
    if (
      labelFilter !== "all" &&
      !labelOptions.includes(labelFilter)
    ) {
      setLabelFilter("all");
    }
  }, [labelFilter, labelOptions]);

  const filtered = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    const next = items.filter((item) => {
      if (
        labelFilter !== "all" &&
        !item.labels.some((label) => label.name === labelFilter)
      ) {
        return false;
      }
      if (focusFilter === "ready" && (item.draft || item.state !== "open"))
        return false;
      if (focusFilter === "drafts" && !item.draft) return false;
      if (focusFilter === "unlabeled" && item.labels.length > 0) return false;
      if (focusFilter === "discussion" && item.comments === 0) return false;
      if (!normalizedQuery) return true;
      return [
        item.title,
        item.body ?? "",
        String(item.number),
        item.user?.login ?? "",
        ...item.labels.map((label) => label.name),
      ].some((value) => value.toLowerCase().includes(normalizedQuery));
    });

    return next.sort((left, right) => {
      if (sortMode === "oldest") {
        return (
          new Date(left.created_at).getTime() -
          new Date(right.created_at).getTime()
        );
      }
      if (sortMode === "discussion") {
        return right.comments - left.comments;
      }
      return (
        new Date(right.updated_at).getTime() -
        new Date(left.updated_at).getTime()
      );
    });
  }, [focusFilter, items, labelFilter, query, sortMode]);

  const focusOptions: Array<{ id: FocusFilter; label: string }> =
    kind === "pulls"
      ? [
          { id: "all", label: "All" },
          { id: "ready", label: "Ready" },
          { id: "drafts", label: "Drafts" },
          { id: "discussion", label: "Discussed" },
        ]
      : [
          { id: "all", label: "All" },
          { id: "unlabeled", label: "Unlabeled" },
          { id: "discussion", label: "Discussed" },
        ];

  const clearFilters = () => {
    setQuery("");
    setLabelFilter("all");
    setFocusFilter("all");
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <div className="flex rounded-xl border border-border/45 bg-muted/35 p-0.5">
          {(["pulls", "issues"] as const).map((itemKind) => (
            <button
              key={itemKind}
              type="button"
              aria-pressed={kind === itemKind}
              onClick={() => onKindChange(itemKind)}
              className={cn(
                "flex h-7 items-center gap-1.5 rounded-[9px] px-3 text-[11.5px] font-medium transition-colors",
                kind === itemKind
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {itemKind === "pulls" ? "Pull requests" : "Issues"}
            </button>
          ))}
        </div>

        <div className="ml-auto flex items-center gap-1.5">
          <Button
            size="xs"
            variant="ghost"
            className="size-7 p-0"
            aria-label="Refresh GitHub items"
            title="Refresh"
            onClick={() => setReloadTick((tick) => tick + 1)}
            disabled={loading}
          >
            <HugeiconsIcon
              icon={ArrowReloadHorizontalIcon}
              size={13}
              strokeWidth={2}
              className={cn(loading && "animate-spin")}
            />
          </Button>
          <Button
            size="xs"
            className="h-7 gap-1.5 text-[10.5px]"
            onClick={() => onCreate(kind)}
          >
            <HugeiconsIcon icon={PlusSignIcon} size={12} strokeWidth={2} />
            {kind === "pulls" ? "New pull request" : "New issue"}
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-1.5">
        <Metric
          label={stateFilter === "open" ? "Open" : "Loaded"}
          value={items.length}
        />
        <Metric
          label={kind === "pulls" ? "Drafts" : "Unlabeled"}
          value={
            kind === "pulls"
              ? items.filter((item) => item.draft).length
              : items.filter((item) => item.labels.length === 0).length
          }
          tone="warning"
        />
        <Metric
          label="Active discussion"
          value={items.filter((item) => item.comments > 0).length}
          tone="info"
        />
      </div>

      <div className="rounded-xl border border-border/50 bg-card/35 p-2">
        <div className="flex flex-wrap items-center gap-1.5">
          <div className="flex rounded-lg bg-muted/45 p-0.5">
            {(["open", "closed", "all"] as const).map((state) => (
              <button
                key={state}
                type="button"
                aria-pressed={stateFilter === state}
                onClick={() => setStateFilter(state)}
                className={cn(
                  "h-6 rounded-md px-2 text-[9.5px] font-medium capitalize transition-colors",
                  stateFilter === state
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {state}
              </button>
            ))}
          </div>

          <div className="flex min-w-0 flex-wrap gap-1">
            {focusOptions.map((option) => (
              <button
                key={option.id}
                type="button"
                aria-pressed={focusFilter === option.id}
                onClick={() => setFocusFilter(option.id)}
                className={cn(
                  "h-6 rounded-md border px-2 text-[9.5px] font-medium transition-colors",
                  focusFilter === option.id
                    ? "border-primary/30 bg-primary/[0.08] text-primary"
                    : "border-transparent text-muted-foreground hover:border-border/60 hover:text-foreground",
                )}
              >
                {option.label}
              </button>
            ))}
          </div>

          <select
            value={sortMode}
            onChange={(event) =>
              setSortMode(event.target.value as SortMode)
            }
            aria-label="Sort items"
            className="ml-auto h-6 rounded-md border border-border/55 bg-background px-2 text-[9.5px] text-foreground outline-none"
          >
            <option value="updated">Recently updated</option>
            <option value="oldest">Oldest first</option>
            <option value="discussion">Most discussed</option>
          </select>
        </div>

        <div className="mt-2 flex flex-wrap items-center gap-1.5">
          <div className="relative min-w-[12rem] flex-1">
            <HugeiconsIcon
              icon={Search01Icon}
              size={13}
              strokeWidth={1.85}
              className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/50"
            />
            <input
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={`Search ${kind === "pulls" ? "pull requests" : "issues"}, authors, labels…`}
              aria-label="Search GitHub items"
              spellCheck={false}
              className="h-8 w-full rounded-lg border border-border/60 bg-background pl-7 pr-2 text-[10.5px] text-foreground outline-none placeholder:text-muted-foreground/45 focus:border-ring"
            />
          </div>

          {labelOptions.length > 0 ? (
            <div className="relative">
              <HugeiconsIcon
                icon={Tag01Icon}
                size={12}
                strokeWidth={1.85}
                className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground/50"
              />
              <select
                value={labelFilter}
                onChange={(event) => setLabelFilter(event.target.value)}
                aria-label="Filter by label"
                className="h-8 max-w-[11rem] appearance-none rounded-lg border border-border/60 bg-background pl-6 pr-6 text-[10px] text-foreground outline-none focus:border-ring"
              >
                <option value="all">All labels</option>
                {labelOptions.map((label) => (
                  <option key={label} value={label}>
                    {label}
                  </option>
                ))}
              </select>
              <HugeiconsIcon
                icon={ArrowDown01Icon}
                size={11}
                strokeWidth={1.85}
                className="pointer-events-none absolute right-1.5 top-1/2 -translate-y-1/2 text-muted-foreground/50"
              />
            </div>
          ) : null}
        </div>
      </div>

      {error ? (
        <div
          role="alert"
          className="rounded-xl border border-destructive/20 bg-destructive/[0.07] px-3 py-2 text-[10.5px] text-destructive"
        >
          <p className="font-medium">GitHub items could not be loaded.</p>
          <p className="mt-0.5 opacity-80">{error}</p>
        </div>
      ) : null}

      {loading ? (
        <SkeletonList />
      ) : filtered.length === 0 ? (
        <EmptyState
          filtered={items.length > 0}
          kind={kind}
          stateFilter={stateFilter}
          onClear={clearFilters}
        />
      ) : (
        <>
          <div className="flex items-center justify-between px-1">
            <p
              className="text-[9.5px] text-muted-foreground/60"
              aria-live="polite"
            >
              {filtered.length} result{filtered.length === 1 ? "" : "s"}
            </p>
            <p className="text-[9px] text-muted-foreground/45">
              Open an item to review readiness, discussion, and actions.
            </p>
          </div>
          <ul className="-mx-1 flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto px-1 pb-1">
            {filtered.map((item) => (
              <ItemRow
                key={`${kind}-${item.number}`}
                item={item}
                kind={kind}
                slug={slug}
                onOpen={() => onOpenItem(kind, item.number)}
              />
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

function ItemRow({
  item,
  kind,
  slug,
  onOpen,
}: {
  item: GHItem;
  kind: ItemKind;
  slug: RepoSlug;
  onOpen: () => void;
}) {
  const state = itemState(item);
  const resolvedAt =
    state === "merged"
      ? item.merged_at ?? item.pull_request?.merged_at
      : state === "closed" || state === "not_planned"
        ? item.closed_at
        : null;
  const verb =
    state === "merged"
      ? "merged"
      : state === "closed" || state === "not_planned"
        ? "closed"
        : "updated";

  return (
    <li className="group/row min-w-0">
      <div className="flex items-stretch gap-1 rounded-xl border border-border/45 bg-card/30 pr-1.5 transition-colors hover:border-border hover:bg-muted/30">
        <button
          type="button"
          onClick={onOpen}
          className="flex min-w-0 flex-1 items-start gap-2.5 rounded-xl px-2.5 py-2.5 text-left outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          <span className="mt-0.5 flex size-5 items-center justify-center">
            <ItemStateIcon state={state} kind={kind} size={15} />
          </span>
          <span className="min-w-0 flex-1">
            <span className="flex min-w-0 items-start gap-2">
              <span className="min-w-0 flex-1 text-[11.5px] font-medium leading-snug text-foreground">
                {item.title}
              </span>
              <span className="shrink-0 font-mono text-[9.5px] text-muted-foreground/50">
                #{item.number}
              </span>
            </span>
            <span className="mt-1 flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-1 text-[9.5px] text-muted-foreground/65">
              {state !== "open" ? (
                <>
                  <StateText state={state} kind={kind} />
                  <span className="text-muted-foreground/35">·</span>
                </>
              ) : null}
              <span className="flex items-center gap-1">
                <Avatar url={item.user?.avatar_url} size={13} />
                <span>{item.user?.login ?? "unknown"}</span>
              </span>
              <span className="text-muted-foreground/35">·</span>
              <span>
                {verb} {relativeTime(resolvedAt ?? item.updated_at)}
              </span>
              {item.comments > 0 ? (
                <span className="flex items-center gap-1">
                  <HugeiconsIcon
                    icon={Comment01Icon}
                    size={10}
                    strokeWidth={1.9}
                  />
                  {item.comments}
                </span>
              ) : null}
            </span>
            {item.labels.length > 0 ? (
              <span className="mt-1.5 block max-h-5 overflow-hidden">
                <Labels labels={item.labels.slice(0, 4)} />
              </span>
            ) : null}
          </span>
          <HugeiconsIcon
            icon={ArrowRight01Icon}
            size={13}
            strokeWidth={1.9}
            className="mt-1 shrink-0 text-muted-foreground/0 transition-colors group-hover/row:text-muted-foreground/45"
          />
        </button>
        <span className="flex items-center">
          <AssignAgentButton
            kind={kind === "pulls" ? "pr" : "issue"}
            slug={slug}
            number={item.number}
            title={item.title}
            body={item.body}
            url={item.html_url}
            variant="chip"
          />
        </span>
      </div>
    </li>
  );
}

function Metric({
  label,
  value,
  tone = "neutral",
}: {
  label: string;
  value: number;
  tone?: "neutral" | "warning" | "info";
}) {
  return (
    <div className="rounded-xl border border-border/45 bg-card/30 px-2.5 py-2">
      <p className="text-[8.5px] font-medium uppercase tracking-wide text-muted-foreground/55">
        {label}
      </p>
      <p
        className={cn(
          "mt-0.5 text-[13px] font-semibold",
          tone === "warning"
            ? "text-amber-500"
            : tone === "info"
              ? "text-sky-500"
              : "text-foreground",
        )}
      >
        {value}
      </p>
    </div>
  );
}

function SkeletonList() {
  return (
    <ul className="flex flex-col gap-1 px-1" aria-hidden>
      {Array.from({ length: 6 }).map((_, index) => (
        <li
          key={index}
          className="flex items-start gap-2.5 rounded-xl border border-border/35 px-2.5 py-3"
          style={{ opacity: 1 - index * 0.12 }}
        >
          <span className="mt-0.5 size-4 shrink-0 animate-pulse rounded-full bg-muted" />
          <span className="flex min-w-0 flex-1 flex-col gap-2">
            <span
              className="h-3 animate-pulse rounded bg-muted"
              style={{ width: `${76 - index * 6}%` }}
            />
            <span className="h-2 w-2/5 animate-pulse rounded bg-muted/70" />
          </span>
        </li>
      ))}
    </ul>
  );
}

function EmptyState({
  filtered,
  kind,
  stateFilter,
  onClear,
}: {
  filtered: boolean;
  kind: ItemKind;
  stateFilter: ItemStateFilter;
  onClear: () => void;
}) {
  const noun = kind === "pulls" ? "pull requests" : "issues";
  const stateWord = stateFilter === "all" ? "" : `${stateFilter} `;
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 py-12 text-center">
      <span className="flex size-10 items-center justify-center rounded-2xl bg-foreground/[0.04] text-muted-foreground/60">
        <HugeiconsIcon icon={InboxIcon} size={20} strokeWidth={1.6} />
      </span>
      <p className="text-[12px] font-medium text-foreground/80">
        {filtered ? "No matches in this view" : `No ${stateWord}${noun}`}
      </p>
      <p className="max-w-[19rem] text-[10.5px] leading-relaxed text-muted-foreground/60">
        {filtered
          ? "Clear the focused view, search, or label filter to see the full list."
          : `When ${noun} arrive, this view will surface their state, discussion, and agent actions.`}
      </p>
      {filtered ? (
        <Button
          size="xs"
          variant="outline"
          className="mt-1 h-7 text-[10px]"
          onClick={onClear}
        >
          Clear filters
        </Button>
      ) : null}
    </div>
  );
}
