# Trademark and release identity policy

The trdr source code is licensed under the AGPLv3. Two things are not part of that
grant, and this document says exactly where the line is.

## Reserved

| Reserved | Meaning |
|---|---|
| The name **trdr** | The product name, as a name for trading software |
| The trdr logo and wordmark | The project's visual identity |
| The Apple Developer ID signature | The certificate the official DMG is signed with |
| The notarized official DMG | The build Apple has notarized for this project |
| The official update channel and its signing key | The feed a user's app checks for updates |
| The `com.fxylabs.trdr` bundle identifier | The macOS bundle and Keychain service id |

Signing keys and the notarization identity are secrets, not licensable marks. They stay
with the project because they are the only way a user can tell an official build from
someone else's.

## What you may do

- Use, study, modify and redistribute the source under the AGPLv3, commercially included.
- Say truthfully that your software is *based on trdr*, *a fork of trdr*, or *derived
  from trdr*, and link here. Nominative reference like that needs no permission.
- Keep the `trdr` CLI command name inside your own installation if renaming it would
  break your users' scripts. Do not ship it as if it were the official binary.
- Distribute your fork under your own name, your own bundle identifier, and your own
  Apple Developer ID.

## What you may not do

- Name your fork or product `trdr`, or a name close enough to be mistaken for it.
- Ship the trdr logo or wordmark as the identity of your build.
- Use `com.fxylabs.trdr` as your bundle identifier. It also decides which Keychain items
  the app reads, so reusing it makes your build read credentials the user stored for the
  official app.
- Point your build at the official update channel, or publish updates that the official
  app would accept.
- Present your build as official, endorsed by, or supported by this project.

## Renaming a fork

Change all of these before you publish:

1. the product name and any user-visible wordmark
2. the bundle identifier, and with it the Keychain service name
3. the update feed URL and its verification key
4. the signing identity, to your own Apple Developer ID
5. the support and issue links, so reports reach you and not us

The AGPLv3 requires you to keep the copyright notices and the license, and to offer your
users the corresponding source of your modified version. Renaming does not change that.

## Questions

Open an issue. If a use is honest and unlikely to confuse anyone about which build they
are running, the answer is usually yes.
