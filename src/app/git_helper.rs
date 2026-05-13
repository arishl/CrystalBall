pub(crate) const GUIDE: &str = r#"# Git Helper

## Daily Flow

- `git init`
  Creates a new repository in the current folder.

- `git clone <url>`
  Downloads an existing repository.

- `git status`
  Shows changed, staged, untracked, and conflicted files.

- `git diff`
  Shows unstaged changes in tracked files.

- `git diff --staged`
  Shows changes that are staged for the next commit.

- `git add <path>`
  Stages a file or folder for commit.

- `git restore <path>`
  Discards unstaged changes in a file.

- `git restore --staged <path>`
  Unstages a file while keeping the edits.

- `git commit -m "message"`
  Commits staged changes with a short message.

- `git rm <path>`
  Removes a tracked file and stages the removal.

- `git mv <old> <new>`
  Renames or moves a tracked file and stages the change.

## Branches

- `git branch`
  Lists local branches.

- `git switch <branch>`
  Moves to an existing branch.

- `git switch -c <branch>`
  Creates a new branch and switches to it.

- `git merge <branch>`
  Merges another branch into the current branch.

- `git rebase <branch>`
  Replays current branch commits on top of another branch.

- `git cherry-pick <commit>`
  Applies one specific commit onto the current branch.

## Remote Work

- `git remote -v`
  Lists configured remotes.

- `git remote add origin <url>`
  Adds a remote named `origin`.

- `git fetch`
  Downloads remote branch and tag information without changing your files.

- `git pull`
  Fetches and merges the remote branch into your current branch.

- `git push`
  Uploads local commits to the remote branch.

- `git push -u origin <branch>`
  Pushes a new branch and sets its upstream.

## Temporary Shelving

- `git stash`
  Temporarily saves uncommitted changes and restores a clean working tree.

- `git stash pop`
  Reapplies the latest stash and removes it from the stash list.

- `git stash list`
  Lists saved stashes.

## History

- `git log --oneline --decorate --graph --all`
  Shows a compact commit graph.

- `git show <commit>`
  Shows one commit and its patch.

- `git blame <path>`
  Shows which commit last changed each line.

- `git reflog`
  Shows where branch heads and `HEAD` have recently pointed.

- `git bisect`
  Runs a binary search through history to find the commit that introduced a bug.

## Undo and Cleanup

- `git reset --soft <commit>`
  Moves the branch pointer while keeping changes staged.

- `git reset --mixed <commit>`
  Moves the branch pointer while keeping changes unstaged.

- `git reset --hard <commit>`
  Discards tracked changes and moves the branch pointer. Use with care.

- `git clean -fd`
  Deletes untracked files and folders. Use with care.

## Tags and Releases

- `git tag`
  Lists tags.

- `git tag v1.2.3`
  Creates a local release tag.

- `git push origin v1.2.3`
  Pushes the tag to GitHub, which can trigger release builds.

## Advanced Project Tools

- `git worktree add <path> <branch>`
  Checks out another branch into a separate folder.

- `git submodule update --init --recursive`
  Downloads nested repositories used by the project.

- `git config --list`
  Shows Git configuration values.

## Conflict Basics

1. Run `git status` to see conflicted files.
2. Open each conflicted file and resolve the marked sections.
3. Run `git add <path>` for each resolved file.
4. Finish with `git commit`, or continue the operation Git asked you to continue.
"#;
