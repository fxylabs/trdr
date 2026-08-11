/**
 * Does the CSS actually reach what the components render?
 *
 * The kit's stylesheet was written against hand-written HTML and selects on
 * structure as often as on class names. A React component can carry every class
 * anyone thought to assert and still render a `<div>` where `.trdr-connection
 * strong` expected a `<strong>`, at which point the class name is present, the
 * test is green, and the dot is not on the screen. So the selectors come out of
 * the shipped stylesheet and are run against the rendered tree.
 */

import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";

import { CATALOGUE } from "./testing/catalogue";
import { declaration, hasSelector, matchesKitSelector } from "./testing/stylesheet";

afterEach(cleanup);

/** The helper is only evidence if it can say no. */
test("a selector the stylesheet does not have is an error, not a pass", () =>
{
    const { container } = render(<div className="trdr-panel" />);
    const root = container.firstElementChild!;

    expect(hasSelector(".trdr-panel")).toBe(true);
    expect(hasSelector(".trdr-not-a-rule")).toBe(false);
    expect(() => matchesKitSelector(root, ".trdr-not-a-rule")).toThrow();
    expect(matchesKitSelector(root, ".trdr-metric")).toBe(false);
});

test.each(
    CATALOGUE.flatMap((role) =>
        role.cases.flatMap((one, index) =>
            [...(index === 0 ? role.selectors : []), ...(one.selectors ?? [])].map(
                (selector) => [`${role.contract} · ${one.state ?? "default"} · ${selector}`, one.element, selector] as const
            )
        )
    )
)("%s", (_name, element, selector) =>
{
    const { container } = render(element);
    const root = container.firstElementChild;

    expect(root).not.toBeNull();
    expect(matchesKitSelector(root!, selector)).toBe(true);
});

/**
 * The rules that pass a unit test and fail a visual review.
 *
 * A class on an element proves nothing about what the class does. These assert
 * the other half — that the declaration behind the class is the one the contract
 * asked for — so that a stylesheet edit which drops tabular figures or
 * right-alignment fails here rather than in front of someone at 1440×900.
 */
test("numerals are tabular and data cells align right", () =>
{
    expect(declaration(".trdr-num", "font-variant-numeric")).toBe("tabular-nums");
    expect(declaration(".trdr-data-table td", "text-align")).toBe("right");
    expect(declaration(".trdr-data-table th", "text-align")).toBe("right");
    expect(declaration(".trdr-data-table td:first-child", "text-align")).toBe("left");
    expect(declaration(".trdr-data-table th:first-child", "text-align")).toBe("left");
});

test("data rows are the 52px the contract asks for", () =>
{
    expect(declaration(".trdr-data-table td", "height")).toBe("var(--trdr-size-row-data)");
    expect(declaration(":root", "--trdr-size-row-data")).toBe("52px");
});

/**
 * Focus has to be visible, and in this system it is one rule that hangs off
 * `.trdr-scope`. `AppShell` carries the class; anything mounted outside it has
 * to, which is why the class is exported rather than left as a detail.
 */
test("focus is visible, from the scope the shell puts around everything", () =>
{
    expect(declaration(".trdr-scope :focus-visible", "outline")).toBe("2px solid var(--trdr-color-focus)");
    expect(declaration(".trdr-scope :focus-visible", "outline-offset")).toBe("2px");
    expect(declaration(".trdr-field:focus", "outline")).toBe("2px solid rgb(200 242 76 / 58%)");
});

/** Up is red and down is blue. Korean market convention, and not a bug. */
test("market tone resolves to the Korean convention", () =>
{
    expect(declaration(".trdr-market-up", "color")).toBe("var(--trdr-color-market-up)");
    expect(declaration(":root", "--trdr-color-market-up")).toBe("var(--trdr-red-500)");
    expect(declaration(":root", "--trdr-color-market-down")).toBe("var(--trdr-blue-500)");
});

/** Lime is the current position and the primary button's dot. Never a fill. */
test("the accent is a marker, not a surface", () =>
{
    expect(declaration(".trdr-button--primary", "background")).toBe("var(--trdr-color-shell)");
    expect(declaration(".trdr-button--primary::before", "background")).toBe("var(--trdr-color-focus)");
    expect(declaration('.trdr-nav-item[aria-current="page"]::after', "background")).toBe("var(--trdr-color-focus)");
    expect(declaration('.trdr-daybar i[data-state="today"]', "background")).toBe("var(--trdr-color-focus)");
    expect(declaration('.trdr-daybar i[data-state="done"]', "background")).toBe("var(--trdr-graphite-800)");
});

/** Green is the system connection and never a number. */
test("the connection dot is the system colour", () =>
{
    expect(declaration(".trdr-connection strong::before", "background")).toBe("var(--trdr-color-system-success)");
});

/** The shell's three columns, at the reference size. */
test("the shell is 196 / fluid / 420 at 1440x900", () =>
{
    expect(declaration(".trdr-app-shell", "grid-template-columns")).toBe(
        "var(--trdr-size-sidebar) minmax(0, 1fr) var(--trdr-size-agent-rail)"
    );
    expect(declaration(":root", "--trdr-size-sidebar")).toBe("196px");
    expect(declaration(":root", "--trdr-size-agent-rail")).toBe("420px");
    expect(declaration(":root", "--trdr-size-desktop-width")).toBe("1440px");
    expect(declaration(":root", "--trdr-size-desktop-height")).toBe("900px");
});
