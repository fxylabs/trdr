import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";

import * as pty from "./pty";

// The controls are the spike's answer sheet: the PATH the app inherited beside
// the PATH the user actually has, and the two ways a user can point at an
// executable when the first one does not contain it.
export function Controls({ size }: { size: pty.Size })
{
    const [report, setReport] = useState<pty.EnvReport | null>(null);
    const [program, setProgram] = useState("claude");
    const [status, setStatus] = useState("");
    const [live, setLive] = useState(false);

    useEffect(() =>
    {
        void pty.envReport().then(setReport);
        void pty.running().then(setLive);
    }, []);

    const start = async () =>
    {
        try
        {
            const [name, ...args] = program.trim().split(/\s+/);
            const info = await pty.spawn({ program: name ?? "", args, cwd: null, cols: size.cols, rows: size.rows });
            setStatus(`spawned ${info.program} (pid ${info.pid ?? "unknown"}) at ${size.cols}×${size.rows}`);
            setLive(true);
        }
        catch (error)
        {
            setStatus(String(error));
        }
    };

    const pick = async () =>
    {
        const chosen = await open({ multiple: false, directory: false, title: "Pick the agent executable" });
        if (typeof chosen === "string")
        {
            setProgram(chosen);
        }
    };

    const stop = async () =>
    {
        await pty.kill();
        setLive(false);
        setStatus("killed");
    };

    return <section className="controls">
        <div className="row">
            <input
                value={program}
                onChange={(event) => setProgram(event.target.value)}
                spellCheck={false}
                placeholder="claude · codex · seq 1 500"
            />
            <button onClick={() => void pick()}>pick executable…</button>
            <button onClick={() => void start()} disabled={live}>spawn</button>
            <button onClick={() => void stop()} disabled={!live}>kill</button>
            <button onClick={() => void pty.clearScrollback()}>clear scrollback</button>
        </div>
        {status !== "" && <p className="status">{status}</p>}
        <PathReport report={report} />
    </section>;
}

function PathReport({ report }: { report: pty.EnvReport | null })
{
    if (report === null)
    {
        return null;
    }
    const same = report.app_path === report.login_path;
    return <dl className="paths">
        <dt>login shell</dt>
        <dd>{report.login_shell}</dd>
        <dt>PATH the app inherited</dt>
        <dd><Path value={report.app_path} /></dd>
        <dt>PATH the login shell reports</dt>
        <dd><Path value={report.login_path} /></dd>
        <dt>same?</dt>
        <dd className="verdict">{same ? "yes" : "no — this is the pitfall"}</dd>
    </dl>;
}

function Path({ value }: { value: string | null })
{
    if (value === null || value === "")
    {
        return <span>(empty)</span>;
    }
    const entries = value.split(":").filter((entry) => entry !== "");
    return <details>
        <summary>{entries.length} entries</summary>
        <pre>{entries.join("\n")}</pre>
    </details>;
}
