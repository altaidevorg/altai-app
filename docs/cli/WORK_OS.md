# Work OS CLI

Canonical Work commands open the same workspace `work.db` used by Desktop and
native stdio hosts. Commands that target an existing Work item take its ID as
the only positional value and an optional named workspace:

```text
altai-cli work show <WORK_ID> [--workspace <PATH>]
altai-cli work start <WORK_ID> --revision <N> [--workspace <PATH>]
altai-cli work ready-for-review <WORK_ID> --revision <N> [--workspace <PATH>]
altai-cli work review <WORK_ID> --revision <N> (--accept | --return) [--workspace <PATH>]
```

Omitting `--workspace` selects the current directory. The named option replaces
the earlier unreleased draft shape `[path] <id>`: an optional positional path
before a required positional ID is ambiguous to Clap and could panic while
building the command parser. Scripts should use `--workspace <PATH>` rather
than relying on either positional order.
