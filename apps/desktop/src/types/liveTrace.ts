export interface TracePoint {
  elapsed_s: number;
  bx_mv: number;
  by_mv: number;
  freq_hz: number;
}

export interface TraceSnapshot {
  points: TracePoint[];
  frames_total: number;
  frames_unique: number;
  dup_rate: number;
  avg_read_us: number;
}

export interface CollectorStatus {
  frames_captured: number;
  frames_duplicated: number;
  frames_parse_error: number;
  total_reads_attempted: number;
  avg_read_time_us: number;
  running: boolean;
}
