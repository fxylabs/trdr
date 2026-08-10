# Third-party notices and dependency license policy

trdr is distributed under the AGPLv3. Every dependency shipped inside the app or the CLI
must be redistributable under those terms, and its notice must reach the user.

## Current state

**No third-party dependencies are vendored or declared yet.** There is no
`package.json` dependency block, no lockfile, no `Cargo.toml` and no vendored source in
this repository. There is nothing to notice, and no license conflict to resolve.

This file becomes a generated artifact as soon as the first dependency lands: the notice
list below is produced from the lockfiles at release time and committed with the release,
not maintained by hand.

## Which licenses are allowed

| Class | Examples | Allowed |
|---|---|---|
| Permissive | MIT, ISC, BSD-2/3-Clause, Apache-2.0, Zlib, Unlicense, CC0, public domain | Yes |
| Weak copyleft, linked | MPL-2.0, LGPL-3.0 | Yes, keep the source offer intact |
| Strong copyleft, same family | GPL-3.0, AGPL-3.0, GPL-2.0-**or-later** | Yes |
| GPL-2.0-**only** | — | No. Incompatible with AGPLv3 |
| Source-available, not free | SSPL, BUSL, Elastic License, Commons Clause | No |
| No license, or unclear license | — | No |
| Proprietary or per-seat licensed data or SDKs | — | No |

Apache-2.0 code may be combined into this AGPLv3 work, and not the reverse. Anything
outside the allowed rows needs an explicit decision recorded before it is added, not a
review comment after the fact.

## Rules for adding a dependency

1. Record the license before the dependency is merged, not at release time.
2. Keep the upstream `LICENSE`, `NOTICE` and attribution requirements. Apache-2.0 and
   several chart and font licenses require visible attribution — honor it in the app's
   about surface, not only in this file.
3. Prefer a dependency that carries an SPDX identifier. A package whose license is only
   a prose sentence in a README needs a recorded decision.
4. Bundled binaries and patched libraries count. The bundled SQLite build is a shipped
   dependency and appears in the generated notices like any other.
5. Fonts, icons and sample data are dependencies too, and their terms are usually
   stricter than code licenses.

## Data is not a dependency

Market data, disclosures and macro statistics collected on a user's Mac with the user's
own credentials are the user's data, not a redistributed component of trdr. They are
never bundled into a release.

- No release ships real market data, account data, or a licensed dataset.
- The source's URL, terms URL, collection time, raw hash and transform version go into
  the provenance record for that collection run.
- A bundle's `use_basis` field is the user's own declaration of their right to use that
  data. It is not a legal check performed by trdr.
- There is no official KRX collector, credential field or support path.

## Generated notices

*(Empty. Regenerated from the dependency lockfiles when the first release is cut, and
shipped alongside the DMG and in the app's about surface.)*
