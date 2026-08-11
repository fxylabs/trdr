/**
 * The trdr UI system.
 *
 * Every reusable role in `design/ui-kit/contracts.v2.json` that Phase 1/2
 * executes, and nothing else. A screen composes from these; a screen that needs
 * a visual rule of its own has found a gap in the contract, and the gap is
 * closed here rather than in the screen.
 *
 * Two things are deliberately absent. The terminal — `TerminalShell`,
 * `TerminalTitleBar`, `XtermHost`, `TerminalStatusBar` — is the agent track's,
 * and its classes ship in this package's stylesheet without a component in front
 * of them. `ApprovalDialog` belongs to registration approval in M6; its styles
 * ship for the same reason.
 *
 * The stylesheet is a separate import and is not optional:
 *
 *     import "@trdr/ui/styles.css";
 */

export { AppShell } from "./components/AppShell";
export type { AppShellProps, WorkArea } from "./components/AppShell";

export { BrokerConnection } from "./components/BrokerConnection";
export type { BrokerConnectionProps } from "./components/BrokerConnection";

export { Button } from "./components/Button";
export type { ButtonProps, ButtonVariant } from "./components/Button";

export { CoverageCard } from "./components/CoverageCard";
export type { CoverageCardProps, CoverageCardState } from "./components/CoverageCard";

export { DataTable } from "./components/DataTable";
export type {
    DataTableColumn,
    DataTableProps,
    DataTableSort,
    DataTableState
} from "./components/DataTable";

export { DayProgress } from "./components/DayProgress";
export type { DayProgressProps, DayProgressState } from "./components/DayProgress";

export { EmptyState } from "./components/EmptyState";
export type { EmptyStateProps } from "./components/EmptyState";

export { EventList } from "./components/EventList";
export type { EventContent, EventListProps, EventListState } from "./components/EventList";

export { Field } from "./components/Field";
export type { FieldProps } from "./components/Field";

export { InlineFeedback } from "./components/InlineFeedback";
export type { FeedbackTone, InlineFeedbackProps } from "./components/InlineFeedback";

export { Metric } from "./components/Metric";
export type { MetricProps, MetricState } from "./components/Metric";

export { NavigationItem } from "./components/NavigationItem";
export type { NavigationItemProps } from "./components/NavigationItem";

export { Panel } from "./components/Panel";
export type { PanelProps, PanelState } from "./components/Panel";

export { RuleGrid } from "./components/RuleGrid";
export type { RuleGridProps, RuleGridSectionId, RuleGridState, RuleSection } from "./components/RuleGrid";

export { Sidebar } from "./components/Sidebar";
export type { NavigationItemModel, SidebarProps } from "./components/Sidebar";

export { SignalLog } from "./components/SignalLog";
export type { SignalContent, SignalLogProps, SignalLogState } from "./components/SignalLog";

export { StockCell } from "./components/StockCell";
export type { StockCellProps } from "./components/StockCell";

export { StrategyCard } from "./components/StrategyCard";
export type {
    StrategyCardProps,
    StrategyCardState,
    StrategyStat,
    StrategySummary
} from "./components/StrategyCard";

export { Tag } from "./components/Tag";
export type { TagProps, TagTone } from "./components/Tag";

export { TopBar } from "./components/TopBar";
export type { BackTarget, TopBarProps } from "./components/TopBar";

export { ViewLead } from "./components/ViewLead";
export type { ViewLeadProps } from "./components/ViewLead";

export {
    APPROVAL_STATES,
    ASYNC_DATA_STATES,
    BROKER_CONNECTION_STATES,
    CONTROL_STATES,
    MARKET_VALUE_TONES,
    PAPER_VALIDATION_STATES,
    TERMINAL_PROCESS_STATES,
    TRDR_SCOPE_CLASS
} from "./contracts";
export type {
    ApprovalState,
    AsyncDataState,
    BrokerConnectionState,
    ControlState,
    MarketValueTone,
    PaperValidationState,
    TerminalProcessState
} from "./contracts";
