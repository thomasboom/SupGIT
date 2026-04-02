# Missing Features Compared to Git CLI

## Core Operations

- `fetch` - Fetch from remote without merging
- `merge` - Merge branches
- `rebase` - Rebase onto another branch
- `clean` - Remove untracked files

## History & Inspection

- `blame` - Show who changed what
- `show` - Show commit/object details
- `reflog` - Reference log history
- `rev-parse` - Parse git references

## Collaboration

- `cherry-pick` - Apply specific commits
- `revert` - Create revert commits

## Advanced

- `submodule` - Manage submodules
- `bisect` - Binary search for commits

## Missing Flags on Existing Commands

### log
- `--oneline` - One line per commit
- `--graph` - ASCII graph of branch structure
- `--author` - Filter by author
- `-n` - Limit number of commits
- `--since/--until` - Filter by date range

### branch
- `-l` - List branches
- `-r` - List remote branches
- `-m` - Rename branch
- `-v` - Verbose output

### push
- `-u` - Set upstream branch
- `--force` - Force push
- `--tags` - Push all tags

### pull
- `--rebase` - Rebase instead of merge

### diff
- `--stat` - Show diffstat summary
- `--name-only` - Show only file names
- `--name-status` - Show file name and status
