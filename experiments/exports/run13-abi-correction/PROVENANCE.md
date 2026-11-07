# Provenance

Program source and the four ELF bytes are commit `380a54df2cfac5088b4b177d2e568c4751520463`.

```
commit 380a54df2cfac5088b4b177d2e568c4751520463
Author:     Cursor Agent <cursoragent@cursor.com>
AuthorDate: Sat Nov 7 03:10:05 2026 +0000
Commit:     Cursor Agent <cursoragent@cursor.com>
CommitDate: Sat Nov 7 03:10:05 2026 +0000
tree:       d77b344539b3af2cf458305e4314fed7f1b0584a
```

The previous ABI export is commit `fedc2cfd7ffcbe41343a6652719caf1966c84388`.

```
commit fedc2cfd7ffcbe41343a6652719caf1966c84388
Author:     Cursor Agent <cursoragent@cursor.com>
AuthorDate: Sat Nov 7 03:21:36 2026 +0000
Commit:     Cursor Agent <cursoragent@cursor.com>
CommitDate: Sat Nov 7 03:21:36 2026 +0000
tree:       5297da5e893c1275362292e2114d44a195145fb2
```

`git diff 380a54df2cfac5088b4b177d2e568c4751520463 fedc2cfd7ffcbe41343a6652719caf1966c84388` hashes to `3e26425cdf1743c64c82d98960f6f9f5f6cbd12e0debbad0a72334cdfcbf9467`.

`git diff 380a54df2cfac5088b4b177d2e568c4751520463 HEAD -- experiments/integrated` is empty. Its sha256 is the empty-file hash `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. No program file changed after `380a54d`.

The four ELF hashes in the manifest are the files from that program commit. They were not rebuilt for this correction.

Trust limit: this file can quote those two commits because they already exist. The commit that adds this correction is named in the bundle's `THIS-COMMIT.txt`, which is written after that commit and is not part of the commit object. Check it with `git rev-parse` and `git show -s --format=fuller` on the hash printed there. A match of author, date, subject, and tree is the attestation. The text of `THIS-COMMIT.txt` itself is not inside the tree it describes.
