import { PopoverContent } from "@/components/ui/popover";
import { Spinner } from "@/components/ui/spinner";
import { fileIconUrl } from "@/modules/explorer/lib/iconResolver";
import { FileSuggestionList } from "@altai/agent-ui";

type Props = {
  files: readonly string[];
  activeIndex: number;
  indexing: boolean;
  truncated: boolean;
  hasWorkspace: boolean;
  onPick: (file: string) => void;
  onHover: (index: number) => void;
};

/**
 * Desktop adapter: Popover chrome + explorer icon theme around the shared
 * file suggestion list.
 */
export function FilePickerContent({
  files,
  activeIndex,
  indexing,
  truncated,
  hasWorkspace,
  onPick,
  onHover,
}: Props) {
  return (
    <PopoverContent
      side="top"
      align="start"
      sideOffset={6}
      onOpenAutoFocus={(e) => e.preventDefault()}
      onCloseAutoFocus={(e) => e.preventDefault()}
      onMouseDown={(e) => e.preventDefault()}
      className="w-auto border-0 bg-transparent p-0 shadow-none"
    >
      <FileSuggestionList
        files={files}
        activeIndex={activeIndex}
        indexing={indexing}
        truncated={truncated}
        hasWorkspace={hasWorkspace}
        onPick={onPick}
        onHover={onHover}
        iconUrlForFile={fileIconUrl}
        indexingIndicator={<Spinner className="size-3" />}
      />
    </PopoverContent>
  );
}
