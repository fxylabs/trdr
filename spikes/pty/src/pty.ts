import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";

export interface EnvReport
{
    login_shell: string;
    app_path: string;
    login_path: string | null;
}

export interface SpawnInfo
{
    program: string;
    pid: number | null;
}

export interface SpawnRequest
{
    program: string;
    args: string[];
    cwd: string | null;
    cols: number;
    rows: number;
}

// The stream crosses as base64 of the bytes the host read. Decoding it here to
// a Uint8Array and handing that straight to xterm is what keeps the WebView a
// renderer: no string normalization, no framing, nothing that could turn output
// into product state.
export function decode(payload: string): Uint8Array
{
    const binary = atob(payload);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1)
    {
        bytes[index] = binary.charCodeAt(index);
    }
    return bytes;
}

export interface Size
{
    cols: number;
    rows: number;
}

export function envReport(): Promise<EnvReport>
{
    return invoke<EnvReport>("env_report");
}

export function spawn(request: SpawnRequest): Promise<SpawnInfo>
{
    return invoke<SpawnInfo>("pty_spawn", { request });
}

export function write(data: string): Promise<void>
{
    return invoke("pty_write", { data });
}

export function resize(cols: number, rows: number): Promise<void>
{
    return invoke("pty_resize", { cols, rows });
}

export function kill(): Promise<void>
{
    return invoke("pty_kill");
}

export function running(): Promise<boolean>
{
    return invoke<boolean>("pty_running");
}

export function loadScrollback(): Promise<string>
{
    return invoke<string>("scrollback_load");
}

export function clearScrollback(): Promise<void>
{
    return invoke("scrollback_clear");
}

export function onOutput(handler: (bytes: Uint8Array) => void): Promise<UnlistenFn>
{
    return listen<string>("pty://output", (event) => handler(decode(event.payload)));
}

export function onClosed(handler: () => void): Promise<UnlistenFn>
{
    return listen("pty://closed", () => handler());
}
