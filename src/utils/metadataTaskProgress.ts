export interface MetadataTaskProgress {
  active?: boolean;
  task?: string;
  label?: string;
  current?: number;
  total?: number;
  percent?: number;
  message?: string;
}

export interface FormattedMetadataTaskProgress {
  active: boolean;
  task: string;
  label: string;
  current: number;
  total: number;
  percent: number;
  message: string;
}

export function formatMetadataTaskProgress(
  progress: MetadataTaskProgress | null | undefined,
  fallbackLabel = "Metadata Task",
): FormattedMetadataTaskProgress {
  const active = Boolean(progress?.active);
  const percent = clampPercent(progress?.percent ?? 0);
  const current = Math.max(0, Math.floor(progress?.current ?? 0));
  const total = Math.max(0, Math.floor(progress?.total ?? 0));
  const label = progress?.label?.trim() || fallbackLabel;
  const message =
    progress?.message?.trim() || (active ? "Working..." : "Complete");

  return {
    active,
    task: progress?.task?.trim() || "metadata_task",
    label,
    current,
    total,
    percent,
    message,
  };
}

export function metadataTaskPopupVisible(
  progress: MetadataTaskProgress | null | undefined,
): boolean {
  if (!progress) return false;
  return (
    Boolean(progress.active) || clampPercent(progress.percent ?? 0) === 100
  );
}

function clampPercent(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(100, Math.round(value)));
}
