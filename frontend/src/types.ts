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

export type SchedulerKind = 'Fifo' | 'Priority' | 'RoundRobin';

export interface Config {
  scheduler: SchedulerKind;
  workerCount: number;
  demoTaskCount: number;
}

export interface Metrics {
  throughput: number;
  avgLatencyMs: number;
}

export interface SystemState {
  tasks: Task[];
  robots: Robot[];
  zones: Zone[];
  config: Config;
  metrics: Metrics;
  systemStatus: 'Running' | 'Paused' | 'Stopped' | 'Error';
}

