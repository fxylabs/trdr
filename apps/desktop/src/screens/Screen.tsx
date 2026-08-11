import type { ReactNode } from "react";
import { InlineFeedback, Metric, Panel, TopBar } from "@trdr/ui";
import type { BackTarget } from "@trdr/ui";

import { ERROR, ORIGIN, REQUEST } from "../copy/ko";
import { instant } from "../copy/format";
import type { ModelState, NamedModel } from "../ipc/model";

/**
 * The one road from a model to a screen, and the reason the synthetic statement
 * cannot be forgotten.
 *
 * Milestone M2 requires the window to say at all times that what is on it is
 * not an account and not a market. Leaving that to each screen would make it
 * each screen's option, and the fifth screen written in a hurry is the one that
 * drops it. So the statement is not something a screen renders — it is
 * something this component renders out of `origin`, and a screen reaches its own
 * content by passing through here. There is no branch from `state.status ===
 * "ready"` to a rendered model that does not pass the statement on the way.
 *
 * The other half of the promise is that the statement is visible without
 * scrolling. This renders exactly two elements: a head that does not scroll and
 * a body that does. `shell.css` gives `.trdr-workspace` those two rows, which is
 * shell layout rather than a screen's — every screen gets the same two, and none
 * of them can choose otherwise.
 *
 * Loading and error live here too, for the same reason: they are the two states
 * every screen has and none of them has a different answer to.
 */
export type ScreenProps<Model extends NamedModel> =
{
    /** The view's `h1`. */
    readonly title: string;
    /** What this view is, beside its title. `초안`, `결과`. */
    readonly context?: string;
    /** Present on a detail view, by contract. */
    readonly back?: BackTarget;
    readonly state: ModelState<Model>;
    /** The view, given a model that has been checked to be the right one. */
    readonly children: (model: Model) => ReactNode;
};

/** The origin's short label, after whatever the screen calls itself. */
function contextLine(context: string | undefined, origin: string): string
{
    return context === undefined ? origin : `${context} · ${origin}`;
}

export function Screen<Model extends NamedModel>(props: ScreenProps<Model>)
{
    const { title, context, back, state, children } = props;
    const model = state.status === "ready" ? state.model : undefined;
    const origin = model === undefined ? undefined : ORIGIN[model.origin];

    return (
        <>
            <div data-screen-row="head">
                <TopBar
                    title={title}
                    {...(origin === undefined ? {} : { context: contextLine(context, origin.label) })}
                    {...(model === undefined ? {} : { metadata: instant(model.built_at) })}
                    {...(back === undefined ? {} : { back })}
                />

                {model === undefined ? null : (
                    <InlineFeedback
                        tone={model.origin === "synthetic" ? "warning" : "neutral"}
                        message={ORIGIN[model.origin].statement}
                    />
                )}
            </div>

            <div data-screen-row="body">
                <ScreenBody state={state}>{children}</ScreenBody>
            </div>
        </>
    );
}

/** Waiting, refused, or the view itself. */
function ScreenBody<Model extends NamedModel>(props: {
    readonly state: ModelState<Model>;
    readonly children: (model: Model) => ReactNode;
})
{
    const { state, children } = props;

    if (state.status === "loading")
    {
        return (
            <Panel state="loading">
                <Metric label={REQUEST.loadingSection} value="" state="loading" />
            </Panel>
        );
    }

    if (state.status === "error")
    {
        return <InlineFeedback tone="error" message={`${REQUEST.failed} ${ERROR[state.code]}`} />;
    }

    return <>{children(state.model)}</>;
}
