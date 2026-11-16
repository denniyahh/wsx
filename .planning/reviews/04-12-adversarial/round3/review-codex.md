## CONFIRMED

1. **CRITICAL — CRLF packed records bypass every proposed check.**  
   `.planning/phases/04-repo-model/04-13-PLAN.md:132–137` checks ordering, duplicates and object kinds, but permits this single, canonical replacement record:

   ```bash
   printf '# pack-refs with: peeled fully-peeled sorted \n%s refs/replace/%s\r\n' "$B" "$A" > .git/packed-refs
   ```

   I ran `git cat-file blob refs/heads/main:.wsx.conf` with existing blobs A=`only a\n`, B=`only b\n`: **Git 2.56.1 exited 128, `fatal: ... bad file`**. A linked gix 0.90.0 probe’s `find_blob(A)` returned **`only b\n`**. LF control: both returned `only b\n`.

   gix explicitly accepts CRLF in `gix-ref-0.69.1/src/store/packed/decode/mod.rs:33–38,80–86`. The proposed guard sees one ordered record, no duplicate, and matching blob kinds; it allows a model where Git refuses. **Refuse CRLF records before gix normalizes them.**

2. **MAJOR — missing originals bypass the target-kind guard.**  
   `.planning/phases/04-repo-model/04-13-PLAN.md:137` explicitly passes an original absent from the object store.

   I constructed a tree using `git mktree --missing` with entry `100644 blob 1111111111111111111111111111111111111111<TAB>.wsx.conf`, committed it, and replaced that missing ID with an annotated tag of blob B. **Git printed `only b\n`, exit 0.** The gix probe printed:

   ```text
   original header: Ok(None)
   blob error: Object named 1111111111111111111111111111111111111111 was supposed to be of kind blob, but was kind tag.
   ```

   Control: replacing the same missing ID directly with B made both readers return `only b\n`.

   gix builds the map without requiring the original object (`gix-0.90.0/src/open/repository.rs:539–545`); `find_blob` checks the replacement’s kind without peeling its tag (`src/repository/object.rs:104–105`). The planned early pass therefore reaches `src/git/refs.rs:248–251`, then becomes `ManifestUnreadable` at `src/model/mod.rs:488–492`. **Refuse missing originals, or explicitly handle this target case.**

## SUSPECTED

None retained.

## S1–S15 status

“Closed” below means addressed by the plan instructions, not implemented. Citations abbreviate the two target plan filenames.

| Item | Status | Plan evidence |
|---|---|---|
| S1 | Closed for errors returned by the guard; finding 2 escapes it | 04-12:261–262 |
| S2 | Closed | 04-12:199,249–250,296; 04-13:198,251–252 |
| S3 | Closed for the reported misordering; finding 1 remains | 04-13:131–135,244,262–263 |
| S4 | Closed: slashless-base dispute explicitly settled | 04-12:123,247,257 |
| S5 | Not fully closed: missing-original tag case remains | 04-13:137,246–247 |
| S6 | Closed | 04-13:127,209 |
| S7 | Closed | 04-12:130,212; 04-13:265 |
| S8 | Closed | 04-12:298,305 |
| S9 | Closed | 04-12:298; 04-13:298 |
| S10 | Closed | 04-13:315–316 |
| S11 | Closed: unsupported ordering-proof claim dropped | 04-12:25 |
| S12 | Closed | 04-13:127,194 |
| S13 | Closed by excluding per-worktree bases | 04-12:252,260 |
| S14 | Closed | 04-13:319–320 |
| S15 | Closed | 04-12:90,192–198,245–248,295 |

## Scope

The checks address measured divergences, but the header-dependent ordering check and separate duplicate sort (`04-13:132–134`) can become one strict-order scan, refusing `name <= previous` regardless of header. I checked that this preserves refusal of the measured misordering and duplicate, while accepting the ordered control; it additionally refuses unsorted files without a `sorted` header, permitted by the brief.

All created scratch directories and the probe binary were removed; no build output remains.
