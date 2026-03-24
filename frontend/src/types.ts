export type TaskStatus = 'Pending' | 'Running' | 'Finished' | 'Failed';
export type TaskPriority = 'Low' | 'Normal' | 'High';

export interface Task {
  id: number;
  name: string;
  priority: TaskPriority;
  status: TaskStatus;
  robotId?: number;
  zoneId?: number;
  expectedDurationMs: number;
  startedAt?: string;
  finishedAt?: string;
}

export type WorkerState = 'Idle' | 'Busy' | 'Stopped';

export interface Robot {
  id: number;
  name: string;
  state: WorkerState;
  currentTaskId?: number;
  recentCompleted: number;
}

export interface Zone {
  id: number;
  name: string;
  capacity: number;
  currentTasks: number;
  activeRobots: number;
  health: 'Normal' | 'HighLoad' | 'Error';
}

export type SchedulerKind = 'Fifo' | 'Priority' | 'RoundRobin' | 'Srt';

export interface Config {
  scheduler: SchedulerKind;
  workerCount: number;
  demoTaskCount: number;
}

export interface Metrics {
  throughput: number;
  avgLatencyMs: number;
}

export interface DemoInputTask {
  id: number;
  name: string;
  priority: TaskPriority;
  expectedDurationMs: number;
  description: string;
}

export interface StrategyTaskTiming {
  taskId: number;
  taskName: string;
  priority: TaskPriority;
  workerId: number;
  startMs: number;
  finishMs: number;
  durationMs: number;
}

export interface StrategySummary {
  scheduler: SchedulerKind;
  makespanMs: number;
  avgCompletionMs: number;
  avgWaitMs: number;
  avgHighPriorityCompletionMs: number;
  avgCompletionImprovementVsFifoMs: number;
  avgHighPriorityImprovementVsFifoMs: number;
  workerBusyMs: number[];
  speedupVsFifoPct: number;
  taskTimings: StrategyTaskTiming[];
}

export interface SchedulingAnalysis {
  inputTasks: DemoInputTask[];
  strategies: StrategySummary[];
  workerCount: number;
}

export interface SystemState {
  tasks: Task[];
  robots: Robot[];
  zones: Zone[];
  config: Config;
  metrics: Metrics;
  systemStatus: 'Running' | 'Paused' | 'Stopped' | 'Error';
  schedulingAnalysis: SchedulingAnalysis;
}

