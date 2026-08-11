import { useEffect, useState } from "react";

import type { ErrorCode } from "../bindings";
import { commands } from "../bindings";
import { newRequestId } from "../ipc/requestId";

/**
 * What the shell knows about the host it is running inside.
 *
 * Three states and no fourth. In particular there is no "ready, but some fields
 * are missing": either the round trip finished and the workspace is known, or
 * it did not and the screen says so. A status line that half-renders is how a
 * broken host comes to look like a working one.
 */
export type HostStatus =
    | { readonly kind: "connecting" }
    | {
          readonly kind: "ready";
          readonly workspaceId: string;
          readonly appVersion: string;
      }
    | { readonly kind: "unreachable"; readonly code: ErrorCode };

/**
 * The round trip section 14's stop condition asks for.
 *
 * Two commands, in this order and for two different reasons. `ping` proves the
 * bridge carries a validated type — the id below is parsed into a ULID on the
 * Rust side before the handler is entered, so a bridge that is merely connected
 * but not typed fails here. `bootstrap.get` then asks the question a screen
 * actually has, and answers it out of the workspace the host opened under the
 * writer lease.
 *
 * Both ids are minted here rather than reused. They correlate two separate
 * requests, and a screen that sent one id twice would be claiming they were the
 * same request.
 */
async function askTheHost(): Promise<HostStatus>
{
    const pong = await commands.ping(newRequestId());

    if (pong.outcome.status !== "ok")
    {
        return { kind: "unreachable", code: pong.outcome.value.code };
    }

    const bootstrap = await commands.bootstrapGet(newRequestId());

    if (bootstrap.outcome.status !== "ok")
    {
        return { kind: "unreachable", code: bootstrap.outcome.value.code };
    }

    return {
        kind: "ready",
        workspaceId: bootstrap.outcome.value.workspace_id,
        appVersion: bootstrap.outcome.value.app_version
    };
}

/**
 * Asks the host once, when the shell mounts.
 *
 * A command returns an error envelope rather than rejecting, so the `catch`
 * below is not the ordinary failure path — it is the case where there is no
 * host to answer at all: a WebView with no Tauri behind it, or a command the
 * capability file does not grant. `APP_NOT_RUNNING` is the code section 12 has
 * for exactly that, and it is what `trdr app status` says in the same
 * situation.
 */
export function useHostStatus(): HostStatus
{
    const [status, setStatus] = useState<HostStatus>({ kind: "connecting" });

    useEffect(() =>
    {
        let listening = true;

        const settle = (answer: HostStatus) =>
        {
            if (listening)
            {
                setStatus(answer);
            }
        };

        askTheHost()
            .catch((): HostStatus => ({ kind: "unreachable", code: "APP_NOT_RUNNING" }))
            .then(settle);

        // React runs an effect twice in development's strict mode, and a WebView
        // that navigated away mid-flight would otherwise set state on a torn
        // down component. The second answer is dropped rather than raced.
        return () =>
        {
            listening = false;
        };
    }, []);

    return status;
}
