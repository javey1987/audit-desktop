export interface ColumnSummary {
  name: string;
  sensitive_type: string;
  sensitive_label: string;
  sample_values: string[];
}

export interface FileInfo {
  sheet_name: string;
  total_rows: number;
  columns: ColumnSummary[];
}

export interface DesensitizeResult {
  columns: ColumnSummary[];
  headers: string[];
  rows: string[][];
  matched_count: number;
}
