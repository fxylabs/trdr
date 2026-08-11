/**
 * `design/ui-kit/contracts.v2.json`, loaded rather than transcribed.
 *
 * The design directory is the authority for what a component is called, what
 * states it has and what rules it is under. Tests read it from there, so a
 * contract that moves is a suite that fails on the next run instead of a copy of
 * the contract that nobody notices has gone stale.
 */

import contracts from "../../../../design/ui-kit/contracts.v2.json?raw";

export type ComponentContract =
{
    class: string;
    states: readonly string[];
    accessibility: readonly string[];
    rules: readonly string[];
};

export type Contracts =
{
    stateModels: Readonly<Record<string, readonly string[]>>;
    components: Readonly<Record<string, ComponentContract>>;
    recipes: Readonly<Record<string, readonly string[]>>;
};

export const CONTRACTS = JSON.parse(contracts) as Contracts;
