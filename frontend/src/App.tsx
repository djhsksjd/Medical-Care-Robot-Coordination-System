import React, { useCallback, useEffect, useMemo, useState } from 'react';
import type {
  SystemState,
  Task,
  Robot,
  Zone,
  Config,
  TaskStatus,
  SchedulerKind,
  DemoInputTask,
  StrategySummary,
} from './types';
import './style.css';

const API_BASE = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/$/, '');
const SUPPORTED_SCHEDULERS: SchedulerKind[] = ['Fifo', 'Priority', 'Srt'];

function apiUrl(path: string): string {
  return `${API_BASE}${path}`;
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : 'Unknown error';
}

function formatDuration(ms: number): string {
  if (ms >= 1000) {
    return `${(ms / 1000).toFixed(ms % 1000 === 0 ? 0 : 1)} s`;
  }

  return `${ms} ms`;
}

function formatDelta(ms: number): string {
  const absolute = formatDuration(Math.abs(ms));
  if (ms > 0) {
    return `${absolute} faster`;
  }
  if (ms < 0) {
    return `${absolute} slower`;
  }
  return 'same as FIFO';
}

function getTaskStateTone(status: TaskStatus): 'blue' | 'grey' | 'red' {
  switch (status) {
    case 'Running':
      return 'blue';
    case 'Pending':
      return 'grey';
    case 'Finished':
    case 'Failed':
      return 'red';
  }
}

function getRobotStateTone(state: Robot['state']): 'blue' | 'grey' | 'red' {
  switch (state) {
    case 'Busy':
      return 'blue';
    case 'Idle':
      return 'grey';
    case 'Stopped':
      return 'red';
  }
}

function formatRobotState(state: Robot['state']): string {
  switch (state) {
    case 'Busy':
      return 'Working';
    case 'Idle':
      return 'Idle';
    case 'Stopped':
      return 'Stopped';
  }
}

async function fetchSystemState(): Promise<SystemState> {
  const res = await fetch(apiUrl('/api/state'));
  if (!res.ok) {
    throw new Error(`Failed to fetch state: ${res.status}`);
  }
  return (await res.json()) as SystemState;
}

async function updateConfig(config: Config): Promise<void> {
  const res = await fetch(apiUrl('/api/config'), {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(config),
  });

  if (!res.ok) {
    throw new Error(`Failed to update config: ${res.status}`);
  }
}

async function controlSystem(action: 'start' | 'pause' | 'stop' | 'run-demo'): Promise<void> {
  const res = await fetch(apiUrl('/api/system/control'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ action }),
  });

  if (!res.ok) {
    throw new Error(`Failed to control system: ${res.status}`);
  }
}

const App: React.FC = () => {
  const [state, setState] = useState<SystemState | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [configDraft, setConfigDraft] = useState<Config | null>(null);
  const [isConfigDirty, setIsConfigDirty] = useState(false);
  const [filterStatus, setFilterStatus] = useState<TaskStatus | 'All'>('All');
  const [filterRobotId, setFilterRobotId] = useState<number | 'All'>('All');
  const [filterZoneId, setFilterZoneId] = useState<number | 'All'>('All');

  const refreshState = useCallback(async (): Promise<SystemState | null> => {
    setLoading(true);
    try {
      const data = await fetchSystemState();
      setState(data);
      setConfigDraft((current) => (isConfigDirty && current ? current : data.config));
      setLoadError(null);
      return data;
    } catch (error) {
      setLoadError(getErrorMessage(error));
      return null;
    } finally {
      setLoading(false);
    }
  }, [isConfigDirty]);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      const data = await refreshState();
      if (cancelled || !data) {
        return;
      }
    };

    void load();
    const id = setInterval(load, 2000);

    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [refreshState]);

  const filteredTasks: Task[] = useMemo(() => {
    if (!state) return [];
    return state.tasks.filter((t) => {
      if (filterStatus !== 'All' && t.status !== filterStatus) return false;
      if (filterRobotId !== 'All' && t.robotId !== filterRobotId) return false;
      if (filterZoneId !== 'All' && t.zoneId !== filterZoneId) return false;
      return true;
    });
  }, [state, filterStatus, filterRobotId, filterZoneId]);

  const currentStrategySummary = useMemo(
    () =>
      state
        ? state.schedulingAnalysis.strategies.find(
            (summary) => summary.scheduler === state.config.scheduler,
          ) ?? state.schedulingAnalysis.strategies[0]
        : undefined,
    [state],
  );

  if (!state || !configDraft) {
    if (loadError) {
      return (
        <div className="app-loading app-loading-error">
          <div>
            <div>Failed to load dashboard: {loadError}</div>
            <button type="button" onClick={() => void refreshState()}>
              Retry
            </button>
          </div>
        </div>
      );
    }

    return <div className="app-loading">Loading dashboard...</div>;
  }

  return (
    <div className="dashboard-root">
      <SideFilters
        state={state}
        filterStatus={filterStatus}
        setFilterStatus={setFilterStatus}
        filterRobotId={filterRobotId}
        setFilterRobotId={setFilterRobotId}
        filterZoneId={filterZoneId}
        setFilterZoneId={setFilterZoneId}
      />
      <div className="dashboard-main">
        <TopBar
          systemStatus={state.systemStatus}
          metrics={state.metrics}
          scheduler={state.config.scheduler}
          currentStrategySummary={currentStrategySummary}
          loading={loading}
          busyAction={busyAction}
          error={loadError}
          onControl={async (action) => {
            setBusyAction(action);
            try {
              await controlSystem(action);
              await refreshState();
            } catch (error) {
              setLoadError(getErrorMessage(error));
            } finally {
              setBusyAction(null);
            }
          }}
        />
        <div className="dashboard-content">
          <div className="dashboard-left">
            <StrategyComparisonPanel
              analysis={state.schedulingAnalysis}
              currentScheduler={state.config.scheduler}
            />
            <TaskBoard tasks={filteredTasks} robots={state.robots} zones={state.zones} />
          </div>
          <div className="dashboard-right">
            <RobotsPanel robots={state.robots} tasks={state.tasks} />
            <ZonesPanel zones={state.zones} />
            <DemoDatasetPanel tasks={state.schedulingAnalysis.inputTasks} />
            <ConfigPanel
              config={configDraft}
              busy={busyAction === 'config'}
              onChange={(nextConfig) => {
                setConfigDraft(nextConfig);
                setIsConfigDirty(true);
              }}
              onApply={async () => {
                setBusyAction('config');
                try {
                  await updateConfig(configDraft);
                  setIsConfigDirty(false);
                  const refreshed = await refreshState();
                  if (refreshed) {
                    setConfigDraft(refreshed.config);
                  }
                } catch (error) {
                  setLoadError(getErrorMessage(error));
                } finally {
                  setBusyAction(null);
                }
              }}
            />
          </div>
        </div>
      </div>
    </div>
  );
};

interface TopBarProps {
  systemStatus: SystemState['systemStatus'];
  metrics: SystemState['metrics'];
  scheduler: SchedulerKind;
  currentStrategySummary?: StrategySummary;
  loading: boolean;
  busyAction: string | null;
  error: string | null;
  onControl: (action: 'start' | 'pause' | 'stop' | 'run-demo') => Promise<void>;
}

const TopBar: React.FC<TopBarProps> = ({
  systemStatus,
  metrics,
  scheduler,
  currentStrategySummary,
  loading,
  busyAction,
  error,
  onControl,
}) => {
  const statusColor =
    systemStatus === 'Running'
      ? '#52c41a'
      : systemStatus === 'Paused'
      ? '#faad14'
      : systemStatus === 'Stopped'
      ? '#999'
      : '#ff4d4f';

  return (
    <div className="topbar">
      <div className="topbar-left">
        <div className="topbar-title">Project Blaze Dashboard</div>
        <div className="topbar-status-dot" style={{ backgroundColor: statusColor }} />
        <div className="topbar-subtitle">
          {systemStatus} · {loading ? 'syncing…' : 'live'}
        </div>
      </div>
      <div className="topbar-right">
        <div className="topbar-metrics">
          Live: throughput {metrics.throughput} · avg latency {metrics.avgLatencyMs} ms
          {' · '}mode {scheduler}
          {currentStrategySummary
            ? ` · projected finish ${formatDuration(currentStrategySummary.makespanMs)}`
            : ''}
        </div>
        {error ? <div className="topbar-error">{error}</div> : null}
        <button disabled={busyAction !== null} onClick={() => void onControl('run-demo')}>
          {busyAction === 'run-demo' ? 'Running…' : 'Run Demo Once'}
        </button>
        <button disabled={busyAction !== null} onClick={() => void onControl('start')}>
          Start
        </button>
        <button disabled={busyAction !== null} onClick={() => void onControl('pause')}>
          Pause
        </button>
        <button disabled={busyAction !== null} onClick={() => void onControl('stop')}>
          Stop
        </button>
      </div>
    </div>
  );
};

interface SideFiltersProps {
  state: SystemState;
  filterStatus: TaskStatus | 'All';
  setFilterStatus: (v: TaskStatus | 'All') => void;
  filterRobotId: number | 'All';
  setFilterRobotId: (v: number | 'All') => void;
  filterZoneId: number | 'All';
  setFilterZoneId: (v: number | 'All') => void;
}

const SideFilters: React.FC<SideFiltersProps> = ({
  state,
  filterStatus,
  setFilterStatus,
  filterRobotId,
  setFilterRobotId,
  filterZoneId,
  setFilterZoneId,
}) => {
  return (
    <div className="side-filters">
      <div className="side-filters-section">
        <div className="side-filters-title">Filters</div>
        <label className="side-filters-field">
          <span>Status</span>
          <select
            value={filterStatus}
            onChange={(e) => setFilterStatus(e.target.value as any)}
          >
            <option value="All">All</option>
            <option value="Pending">Pending</option>
            <option value="Running">Running</option>
            <option value="Finished">Finished</option>
            <option value="Failed">Failed</option>
          </select>
        </label>
        <label className="side-filters-field">
          <span>Robot</span>
          <select
            value={filterRobotId}
            onChange={(e) =>
              setFilterRobotId(e.target.value === 'All' ? 'All' : Number(e.target.value))
            }
          >
            <option value="All">All</option>
            {state.robots.map((r) => (
              <option key={r.id} value={r.id}>
                {r.name}
              </option>
            ))}
          </select>
        </label>
        <label className="side-filters-field">
          <span>Zone</span>
          <select
            value={filterZoneId}
            onChange={(e) =>
              setFilterZoneId(e.target.value === 'All' ? 'All' : Number(e.target.value))
            }
          >
            <option value="All">All</option>
            {state.zones.map((z) => (
              <option key={z.id} value={z.id}>
                {z.name}
              </option>
            ))}
          </select>
        </label>
      </div>
    </div>
  );
};

interface TaskBoardProps {
  tasks: Task[];
  robots: Robot[];
  zones: Zone[];
}

const TaskBoard: React.FC<TaskBoardProps> = ({ tasks, robots, zones }) => {
  const robotMap = useMemo(() => new Map(robots.map((r) => [r.id, r])), [robots]);
  const zoneMap = useMemo(() => new Map(zones.map((z) => [z.id, z])), [zones]);

  return (
    <div className="panel">
      <div className="section-title">Tasks</div>
      <div className="state-legend">
        <StateBadge label="Running" tone="blue" />
        <StateBadge label="Pending" tone="grey" />
        <StateBadge label="Finished" tone="red" />
        <span className="state-legend-note">
          Task colors match the demo palette: blue = active, grey = queued, red = finished.
        </span>
      </div>
      <table className="tasks-table">
        <thead>
          <tr>
            <th>ID</th>
            <th>Name</th>
            <th>Priority</th>
            <th>Status</th>
            <th>Robot</th>
            <th>Zone</th>
            <th>Expected (ms)</th>
          </tr>
        </thead>
        <tbody>
          {tasks.map((t) => {
            const r = t.robotId != null ? robotMap.get(t.robotId) : undefined;
            const z = t.zoneId != null ? zoneMap.get(t.zoneId) : undefined;
            return (
              <tr key={t.id}>
                <td>{t.id}</td>
                <td>{t.name}</td>
                <td>{t.priority}</td>
                <td>
                  <StateBadge label={t.status} tone={getTaskStateTone(t.status)} />
                </td>
                <td>{r ? r.name : '-'}</td>
                <td>{z ? z.name : '-'}</td>
                <td>{t.expectedDurationMs}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
};

interface StrategyComparisonPanelProps {
  analysis: SystemState['schedulingAnalysis'];
  currentScheduler: SchedulerKind;
}

const StrategyComparisonPanel: React.FC<StrategyComparisonPanelProps> = ({
  analysis,
  currentScheduler,
}) => {
  const maxMakespan = Math.max(...analysis.strategies.map((summary) => summary.makespanMs), 1);
  const fifo = analysis.strategies.find((summary) => summary.scheduler === 'Fifo');
  const nonFifoStrategies = analysis.strategies.filter((summary) => summary.scheduler !== 'Fifo');

  return (
    <div className="panel comparison-panel">
      <div className="section-title">Scheduling strategy comparison</div>
      <div className="comparison-summary-line">
        Built-in long demo input: {analysis.inputTasks.length} tasks across {analysis.workerCount} robots.
        {fifo ? ` FIFO is the baseline for all improvement numbers below.` : ''}
      </div>
      <div className="comparison-improvements">
        {nonFifoStrategies.map((summary) => (
          <div key={`${summary.scheduler}-improvement`} className="comparison-improvement-card">
            <strong>{summary.scheduler}</strong>
            <span>Avg completion: {formatDelta(summary.avgCompletionImprovementVsFifoMs)}</span>
            <span>Urgent tasks: {formatDelta(summary.avgHighPriorityImprovementVsFifoMs)}</span>
            <span>
              Makespan: {summary.speedupVsFifoPct >= 0 ? '+' : ''}
              {summary.speedupVsFifoPct.toFixed(1)}% vs FIFO
            </span>
          </div>
        ))}
      </div>
      <div className="comparison-grid">
        {analysis.strategies.map((summary) => (
          <StrategyCard
            key={summary.scheduler}
            summary={summary}
            maxMakespan={maxMakespan}
            workerCount={analysis.workerCount}
            active={summary.scheduler === currentScheduler}
          />
        ))}
      </div>
    </div>
  );
};

interface StrategyCardProps {
  summary: StrategySummary;
  maxMakespan: number;
  workerCount: number;
  active: boolean;
}

const StrategyCard: React.FC<StrategyCardProps> = ({ summary, maxMakespan, workerCount, active }) => {
  return (
    <div className={`strategy-card${active ? ' strategy-card-active' : ''}`}>
      <div className="strategy-card-header">
        <div>
          <div className="strategy-name">{summary.scheduler}</div>
          <div className="strategy-subtitle">
            {active ? 'Current dashboard mode' : 'Projected comparison mode'}
          </div>
        </div>
        <div className="strategy-badge">
          {summary.speedupVsFifoPct > 0
            ? `${summary.speedupVsFifoPct.toFixed(1)}% faster than FIFO`
            : summary.scheduler === 'Fifo'
            ? 'Baseline'
            : `${Math.abs(summary.speedupVsFifoPct).toFixed(1)}% slower than FIFO`}
        </div>
      </div>

      <div className="strategy-stats-grid">
        <MetricPill label="Makespan" value={formatDuration(summary.makespanMs)} />
        <MetricPill label="Avg completion" value={formatDuration(summary.avgCompletionMs)} />
        <MetricPill label="Urgent avg finish" value={formatDuration(summary.avgHighPriorityCompletionMs)} />
        <MetricPill label="Avg wait" value={formatDuration(summary.avgWaitMs)} />
        <MetricPill
          label="Vs FIFO avg completion"
          value={formatDelta(summary.avgCompletionImprovementVsFifoMs)}
        />
        <MetricPill
          label="Vs FIFO urgent"
          value={formatDelta(summary.avgHighPriorityImprovementVsFifoMs)}
        />
      </div>

      <div className="timeline-chart">
        {Array.from({ length: workerCount }, (_, index) => {
          const workerId = index + 1;
          const tasks = summary.taskTimings.filter((timing) => timing.workerId === workerId);

          return (
            <div key={workerId} className="timeline-row">
              <div className="timeline-label">Robot {workerId}</div>
              <div className="timeline-track">
                {tasks.map((timing) => {
                  const leftPct = (timing.startMs / maxMakespan) * 100;
                  const widthPct = Math.max((timing.durationMs / maxMakespan) * 100, 4);
                  const color =
                    timing.priority === 'High'
                      ? '#ef4444'
                      : timing.priority === 'Normal'
                      ? '#3b82f6'
                      : '#9ca3af';

                  return (
                    <div
                      key={`${summary.scheduler}-${timing.taskId}`}
                      className="timeline-bar"
                      style={{ left: `${leftPct}%`, width: `${widthPct}%`, background: color }}
                      title={`${timing.taskName} · ${timing.priority} · start ${formatDuration(
                        timing.startMs,
                      )} · finish ${formatDuration(timing.finishMs)}`}
                    >
                      T{timing.taskId}
                    </div>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>

      <div className="worker-loads">
        {summary.workerBusyMs.map((busy, index) => (
          <div key={`${summary.scheduler}-load-${index}`} className="worker-load-row">
            <span>Robot {index + 1}</span>
            <div className="worker-load-bar">
              <div
                className="worker-load-bar-inner"
                style={{ width: `${summary.makespanMs ? (busy / summary.makespanMs) * 100 : 0}%` }}
              />
            </div>
            <span>{formatDuration(busy)}</span>
          </div>
        ))}
      </div>
    </div>
  );
};

const MetricPill: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="metric-pill">
    <span>{label}</span>
    <strong>{value}</strong>
  </div>
);

const StateBadge: React.FC<{ label: string; tone: 'blue' | 'grey' | 'red' }> = ({
  label,
  tone,
}) => <span className={`state-badge state-badge-${tone}`}>{label}</span>;

const DemoDatasetPanel: React.FC<{ tasks: DemoInputTask[] }> = ({ tasks }) => {
  return (
    <div className="panel">
      <div className="section-title">Explicit long demo input</div>
      <div className="config-description">
        This workload is seeded into the backend and also used for the scheduling comparison above.
      </div>
      <div className="dataset-table-wrap">
        <table className="tasks-table dataset-table">
          <thead>
            <tr>
              <th>#</th>
              <th>Task</th>
              <th>Priority</th>
              <th>Expected</th>
              <th>Description</th>
            </tr>
          </thead>
          <tbody>
            {tasks.map((task) => (
              <tr key={task.id}>
                <td>{task.id}</td>
                <td>{task.name}</td>
                <td>{task.priority}</td>
                <td>{formatDuration(task.expectedDurationMs)}</td>
                <td>{task.description}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};

interface RobotsPanelProps {
  robots: Robot[];
  tasks: Task[];
}

const RobotsPanel: React.FC<RobotsPanelProps> = ({ robots, tasks }) => {
  const taskMap = useMemo(() => new Map(tasks.map((t) => [t.id, t])), [tasks]);

  return (
    <div className="panel">
      <div className="section-title">Robots</div>
      <div className="state-legend">
        <StateBadge label="Working" tone="blue" />
        <StateBadge label="Idle" tone="grey" />
        <StateBadge label="Stopped" tone="red" />
        <span className="state-legend-note">
          Robot colors mirror task states to show the scheduler relationship clearly.
        </span>
      </div>
      {robots.map((r) => {
        const currentTask = r.currentTaskId != null ? taskMap.get(r.currentTaskId) : undefined;
        return (
          <div key={r.id} className="robot-card">
            <div className="robot-header">
              {r.name} (#{r.id}){' '}
              <StateBadge label={formatRobotState(r.state)} tone={getRobotStateTone(r.state)} />
            </div>
            <div className="robot-sub">
              Current task:{' '}
              {currentTask ? `${currentTask.name} (#${currentTask.id})` : 'idle'}
              {' · '}recent completed: {r.recentCompleted}
            </div>
          </div>
        );
      })}
    </div>
  );
};

interface ZonesPanelProps {
  zones: Zone[];
}

const ZonesPanel: React.FC<ZonesPanelProps> = ({ zones }) => {
  return (
    <div className="panel">
      <div className="section-title">Zones</div>
      <div className="zones-grid">
        {zones.map((z) => {
          const utilization = z.capacity ? Math.round((z.currentTasks / z.capacity) * 100) : 0;
          const color =
            z.health === 'Normal'
              ? '#52c41a'
              : z.health === 'HighLoad'
              ? '#faad14'
              : '#ff4d4f';
          return (
            <div key={z.id} className="zone-card">
              <div className="zone-name">{z.name}</div>
              <div className="zone-sub">
                Tasks: {z.currentTasks} / {z.capacity}
              </div>
              <div className="zone-sub">Robots: {z.activeRobots}</div>
              <div className="zone-bar">
                <div
                  className="zone-bar-inner"
                  style={{
                    width: `${Math.min(utilization, 100)}%`,
                    backgroundColor: color,
                  }}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

interface ConfigPanelProps {
  config: Config;
  busy: boolean;
  onChange: (c: Config) => void;
  onApply: () => void;
}

const ConfigPanel: React.FC<ConfigPanelProps> = ({ config, busy, onChange, onApply }) => {
  const handleSchedulerChange = (value: SchedulerKind) => {
    onChange({ ...config, scheduler: value });
  };

  return (
    <div className="panel">
      <div className="section-title">Config & Controls</div>
      <div className="config-description">
        Adjust configuration and apply changes to the backend.
      </div>
      <div className="config-form">
        <label>
          <span>Scheduler</span>
          <select
            value={config.scheduler}
            onChange={(e) => handleSchedulerChange(e.target.value as SchedulerKind)}
          >
            {SUPPORTED_SCHEDULERS.map((scheduler) => (
              <option key={scheduler} value={scheduler}>
                {scheduler}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>Worker count</span>
          <input
            type="number"
            min={1}
            value={config.workerCount}
            onChange={(e) => onChange({ ...config, workerCount: Number(e.target.value) })}
          />
        </label>
        <label>
          <span>Demo task count</span>
          <input
            type="number"
            min={1}
            value={config.demoTaskCount}
            onChange={(e) => onChange({ ...config, demoTaskCount: Number(e.target.value) })}
          />
        </label>
        <button type="button" disabled={busy} onClick={onApply}>
          Apply &amp; Restart Demo
        </button>
      </div>
    </div>
  );
};

export default App;

