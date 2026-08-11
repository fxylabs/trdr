/**
 * Asking the host for a screen's model, in one place rather than in five.
 *
 * Every command in `bindings.ts` answers with the same envelope — a protocol
 * version, the request id it is answering, and an outcome that is either a
 * value or an error — and the promise resolves rather than rejecting, so a
 * refusal is a value to branch on and never a `catch`. That shape is identical
 * for all five models, which is why the reading of it belongs here and a screen
 * gets a discriminated union it can render.
 *
 * # Three ways an answer can be wrong before it is even read
 *
 * The envelope arriving is not the same as the envelope being the answer.
 *
 * 1. **It answers a different request.** The id is minted here and travels with
 *    the call, and a response naming another one is a correlation bug — the
 *    kind that shows up as one screen rendering another screen's numbers.
 *    Section 12 has `APP_PROTOCOL_VERSION` for a message that does not fit the
 *    protocol, and this is one.
 * 2. **It carries a different contract.** Section 11 spells the models
 *    `TodayModel/v1`, and every model carries that name in its `model` field
 *    precisely so a screen can check it. A packaged app can meet a newer host,
 *    and the wrong thing to do at that moment is render whichever fields happen
 *    to line up. `DATA_UNSUPPORTED` is section 12's code for data of a kind
 *    this version does not support.
 * 3. **There is no host at all.** A WebView with no Tauri behind it rejects
 *    rather than resolving, which is the one case that is genuinely a `catch`.
 *    `APP_NOT_RUNNING` is what `trdr app status` says in the same situation.
 *
 * # React 19's strict mode, and why the id makes it safe
 *
 * In development every effect is invoked twice. A request layer that dropped
 * nothing would then race two answers into one piece of state, and the later
 * write would win by accident. The effect's cleanup flips `listening`, so the
 * first invocation's answer is discarded before it can be stored — the same
 * shape `useHostStatus` already uses. The two calls carry two different ids,
 * which is correct rather than merely tolerated: they *are* two requests, and
 * section 9.2 gives a reused id a meaning (the same result, not a second
 * execution) that would be a lie here. All five commands are reads, so a
 * duplicate costs a round trip and changes nothing.
 */

import { useEffect, useState } from "react";

import type {
    DataOrigin,
    ErrorCode,
    LabDraftModel,
    LabResultModel,
    StrategiesModel,
    StrategyDetailModel,
    TodayModel,
    UiResponseEnvelope_Serialize
} from "../bindings";
import { commands } from "../bindings";
import { newRequestId } from "./requestId";

/** What every query model carries, whatever screen it is for. */
export type NamedModel =
{
    readonly model: string;
    readonly origin: DataOrigin;
    readonly built_at: string;
};

/** Where a screen is: waiting, holding a model, or holding a reason it has none. */
export type ModelState<Model extends NamedModel> =
    | { readonly status: "loading" }
    | { readonly status: "ready"; readonly model: Model }
    | { readonly status: "error"; readonly code: ErrorCode };

/**
 * One command, named by the contract it answers.
 *
 * `key` is everything `ask` depends on besides the contract — the strategy id,
 * for the one screen that has an argument. It is also the hook's dependency, so
 * anything `ask` closes over that is not in the key would be captured once and
 * never noticed changing.
 */
export type ModelRequest<Model extends NamedModel> =
{
    readonly contract: string;
    readonly key: string;
    readonly ask: (id: string) => Promise<UiResponseEnvelope_Serialize<Model>>;
};

/** Section 11's model names, as the models spell them. */
export const CONTRACT =
{
    today: "TodayModel/v1",
    labDraft: "LabDraftModel/v1",
    labResult: "LabResultModel/v1",
    strategies: "StrategiesModel/v1",
    strategyDetail: "StrategyDetailModel/v1"
} as const;

export function todayRequest(): ModelRequest<TodayModel>
{
    return { contract: CONTRACT.today, key: "", ask: (id) => commands.todayGet(id) };
}

export function labDraftRequest(): ModelRequest<LabDraftModel>
{
    return { contract: CONTRACT.labDraft, key: "", ask: (id) => commands.labDraftGet(id) };
}

export function labResultRequest(): ModelRequest<LabResultModel>
{
    return { contract: CONTRACT.labResult, key: "", ask: (id) => commands.backtestGet(id) };
}

export function strategiesRequest(): ModelRequest<StrategiesModel>
{
    return { contract: CONTRACT.strategies, key: "", ask: (id) => commands.strategiesList(id) };
}

export function strategyRequest(strategy: string): ModelRequest<StrategyDetailModel>
{
    return {
        contract: CONTRACT.strategyDetail,
        key: strategy,
        ask: (id) => commands.strategyGet(id, strategy)
    };
}

/** A refusal a screen can render, from section 12's closed set. */
function refused<Model extends NamedModel>(code: ErrorCode): ModelState<Model>
{
    return { status: "error", code };
}

/** One round trip, with the three checks the module documentation names. */
async function askOnce<Model extends NamedModel>(
    request: ModelRequest<Model>
): Promise<ModelState<Model>>
{
    const id = newRequestId();
    const envelope = await request.ask(id).catch(() => undefined);

    if (envelope === undefined)
    {
        return refused("APP_NOT_RUNNING");
    }

    if (envelope.id !== id)
    {
        return refused("APP_PROTOCOL_VERSION");
    }

    if (envelope.outcome.status === "error")
    {
        return refused(envelope.outcome.value.code);
    }

    if (envelope.outcome.value.model !== request.contract)
    {
        return refused("DATA_UNSUPPORTED");
    }

    return { status: "ready", model: envelope.outcome.value };
}

/**
 * Round trips that have been started and not yet answered.
 *
 * Two things ask for the same model at the same moment and neither is wrong to.
 * Strict mode invokes every effect twice in development, and the shell needs
 * `TodayModel` for the sidebar's broker link at the same time the Today screen
 * needs it for its own content. Both are reads of the same fixture, so the
 * second caller joins the first round trip instead of making another.
 *
 * This is not a cache. An entry is removed the moment its answer arrives, so a
 * later mount always asks again and nothing on screen can be older than the
 * moment it was rendered — which is the failure mode a cache would introduce
 * and the reason there is not one here.
 */
const inFlight = new Map<string, Promise<unknown>>();

/** One round trip per contract and key, however many callers there are. */
function shared<Model extends NamedModel>(request: ModelRequest<Model>): Promise<ModelState<Model>>
{
    const slot = `${request.contract}:${request.key}`;
    const started = inFlight.get(slot);

    if (started !== undefined)
    {
        return started as Promise<ModelState<Model>>;
    }

    const answer = askOnce(request).finally(() => inFlight.delete(slot));

    inFlight.set(slot, answer);

    return answer;
}

/** Asks once per contract and key, and drops an answer nobody is waiting for. */
export function useModel<Model extends NamedModel>(request: ModelRequest<Model>): ModelState<Model>
{
    const [state, setState] = useState<ModelState<Model>>({ status: "loading" });
    const { contract, key, ask } = request;

    useEffect(() =>
    {
        let listening = true;

        setState({ status: "loading" });

        shared<Model>({ contract, key, ask }).then((answer) =>
        {
            if (listening)
            {
                setState(answer);
            }
        });

        return () =>
        {
            listening = false;
        };
        // `ask` is the command for this contract and key and nothing else, so
        // the pair identifies the request. It is deliberately not a dependency:
        // a caller builds it inline, so it is a new function on every render and
        // would re-ask on every render.
    }, [contract, key]);

    return state;
}
