/**
 * The shipped stylesheet, as something a test can ask questions of.
 *
 * A test that asserts a class name is on an element proves the component and the
 * test agree. It does not prove the stylesheet agrees with either of them: the
 * kit's CSS was written for hand-written HTML, and it reaches for structure —
 * `.trdr-connection strong`, `.trdr-nav-item[aria-current="page"]`,
 * `.trdr-data-table td:first-child` — that a React tree can quietly fail to
 * produce while still carrying every class name anyone looked for.
 *
 * So the selectors are read out of the CSS and run against the rendered DOM. A
 * selector that is not in the stylesheet is an error rather than a pass, which
 * is what stops the check from succeeding against a rule nobody wrote.
 */

import components from "../styles/components.v2.css?raw";
import roles from "../styles/roles.css?raw";
import tokens from "../styles/tokens.v2.css?raw";

/** Everything `@trdr/ui/styles.css` pulls in, in the order it pulls it in. */
export const KIT_CSS = `${tokens}\n${components}\n${roles}`;

export type StyleRule =
{
    selector: string;
    declarations: string;
};

/** Comments and `@import` lines removed; whatever is left is rules. */
function stripped(css: string): string
{
    return css.replace(/\/\*[\s\S]*?\*\//g, "").replace(/@import[^;]*;/g, "");
}

function normalize(selector: string): string
{
    return selector.replace(/\s+/g, " ").trim();
}

/**
 * Every rule in the sheet, flattened.
 *
 * The pattern matches innermost blocks, so an `@media` wrapper contributes its
 * inner rules and never its own prelude.
 */
export function rules(css: string = KIT_CSS): StyleRule[]
{
    const found: StyleRule[] = [];

    for (const [, prelude = "", declarations = ""] of stripped(css).matchAll(/([^{}]+)\{([^{}]*)\}/g))
    {
        for (const selector of prelude.split(","))
        {
            found.push({ selector: normalize(selector), declarations: normalize(declarations) });
        }
    }

    return found;
}

const SELECTORS = new Set(rules().map((rule) => rule.selector));

/** Whether the stylesheet actually contains this selector, written this way. */
export function hasSelector(selector: string): boolean
{
    return SELECTORS.has(normalize(selector));
}

/** What the sheet declares for a property on a selector, last declaration winning. */
export function declaration(selector: string, property: string): string | undefined
{
    const declarations = rules()
        .filter((rule) => rule.selector === normalize(selector))
        .flatMap((rule) => rule.declarations.split(";"))
        .map((entry) => entry.split(":"))
        .filter(([name]) => name !== undefined && name.trim() === property)
        .map(([, ...value]) => value.join(":").trim());

    return declarations.at(-1);
}

/** The same selector with the states a static DOM cannot be in taken out. */
function matchable(selector: string): string
{
    return normalize(selector).replace(/::(before|after)/g, "").replace(/:(hover|focus-visible|focus|active)/g, "");
}

/**
 * Does anything in this tree match a selector the stylesheet really has?
 *
 * Throws rather than returns false when the selector is not in the sheet: that
 * is a test asserting against a rule that does not exist, which would otherwise
 * pass forever the moment the CSS moved underneath it.
 */
export function matchesKitSelector(root: Element, selector: string): boolean
{
    if (!hasSelector(selector))
    {
        throw new Error(`the shipped stylesheet has no rule for "${selector}"`);
    }

    const target = matchable(selector);

    return root.matches(target) || root.querySelector(target) !== null;
}
