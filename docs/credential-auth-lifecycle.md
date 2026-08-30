# Credential and loopback authentication lifecycle

## Storage policy

Interactive authentication is session-only by default. The submitted password is used for the Docmost login request and discarded after the origin-bound session is saved. Selecting **Remember password** requests persistence in the operating system credential store under an origin-specific account.

Before a successful session-only login commits its config/session transition,
every remembered credential representation for that canonical origin is cleared
(secure keyring, explicitly acknowledged encrypted fallback, and relevant legacy
state). Automatic reauthentication independently requires the remembered email
to exactly match the active configured email; a mismatch is cleared and fails
closed before any login request is sent.

A missing keyring entry means no remembered password. A keyring initialization, read, or write failure does not silently activate local fallback storage. Remember-password login fails before config or session files change unless the operator has explicitly acknowledged the weaker model with `--allow-insecure-credential-file` or `DOCMOST_ALLOW_INSECURE_CREDENTIAL_FILE=true`.

Persistence is never assumed from a successful write call. After storing a remembered password, the secret is read back through an independent credential handle and compared. If the value is missing, unreadable, or different, the entry is removed and the login fails closed with the same unavailable-storage error. This matters because a build or host without a real platform credential store resolves to an entry-scoped store that accepts a write and retains nothing; without the read-back check an operator would be told the password was remembered when it had been discarded.

**Supported build note.** The reviewed dependency graph enables no platform credential-store backend, and the supported Atlas host is a headless Linux server with no desktop secret service. On that target the keyring path therefore always fails closed. Session-only authentication is the supported and recommended mechanism; unattended reauthentication requires the operator to accept the weaker encrypted-file fallback explicitly. Do not read a successful `remember_password` request as proof that an OS keyring is in use.

The acknowledged fallback uses origin-specific AES-256-GCM ciphertext and key files with directory mode `0700` and file mode `0600` on Unix. Its key and ciphertext share the same directory, so it protects against casual disclosure rather than compromise of that account or directory. A working keyring remains preferred even when fallback is enabled.

## Forget/logout deletion matrix

`docmost-local-mcp forget --base-url <canonical-origin>` is supported, origin-scoped, and idempotent.

| Artifact | Identity | Forget behavior |
| --- | --- | --- |
| OS keyring credential | `credentials|<canonical-origin>` | Deleted; missing entry is success |
| Legacy OS keyring credential | `credentials` | Deleted; never reused |
| Session file | SHA-256 origin filename plus embedded origin | Deleted only for the requested origin |
| Fallback ciphertext | SHA-256 origin filename plus embedded origin | Deleted only for the requested origin |
| Fallback encryption key | SHA-256 origin filename | Deleted only for the requested origin |
| Config | embedded canonical base URL | Deleted only when it matches the requested origin |
| Legacy session | legacy filename and optional embedded origin | Unscoped/stale or matching state is deleted; explicitly different-origin state is preserved |
| Legacy ciphertext/key | legacy filenames and optional encrypted embedded origin | Unscoped/stale or matching state is deleted; explicitly different-origin state is preserved |

Failure to access the keyring is reported before filesystem state changes; it is
not reported as successful complete deletion. This ordering prevents a partial
session-only transition from leaving an ambiguous credential state.

## Loopback request and response security matrix

Each server start binds only to literal IPv4 loopback on an ephemeral port and generates a fresh 32-byte operating-system-random flow secret. The secret is URL-safe encoded into the login/success URL and must also be supplied in the `X-Docmost-Auth-Flow` header on the state-changing request.

| Route | Method | Required request properties | State effect |
| --- | --- | --- | --- |
| `/login?flow=…` | GET | exact flow secret and exact loopback `Host` authority | Display only |
| `/auth` | POST | exact loopback `Host`, exact loopback `Origin`, exact flow header, JSON content type, valid login fields | Completes only after Docmost authentication and persistence policy succeed |
| `/success?flow=…` | GET | exact flow secret and exact loopback `Host` authority | Display only; cannot complete authentication |
| any invalid request | any | missing or mismatched required property | Rejected without invoking the login handler |

Every loopback response carries `Cache-Control: no-store`, `Pragma: no-cache`, `Referrer-Policy: no-referrer`, `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, and a nonce-bound Content Security Policy with `frame-ancestors 'none'`, `form-action 'none'`, and `base-uri 'none'`.
