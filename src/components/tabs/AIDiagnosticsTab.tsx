// CinaVault Premium — AI Diagnostics Tab
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import {
  LibraryEnrichmentResult,
  MediaItem,
  useAppStore,
} from "../../store/appStore";
import {
  buildLibraryPageRequest,
  hasMoreLibraryPages,
  LIBRARY_PAGE_SIZE,
} from "../../utils/libraryLoadPolicy";
import AIVisualizer from "../effects/AIVisualizer";
import {
  formatMetadataTaskProgress,
  metadataTaskPopupVisible,
  MetadataTaskProgress,
} from "../../utils/metadataTaskProgress";
import {
  Brain,
  Send,
  Settings,
  Key,
  Cpu,
  Network,
  FolderSearch,
  Database,
  Loader,
  Sparkles,
  ExternalLink,
  Tag,
  ShieldCheck,
  Trash2,
  Square,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

const DEFAULT_HF_MODEL = "katanemo/Arch-Router-1.5B:hf-inference";
const HF_FREE_MODELS = [
  { id: "Qwen/Qwen3-4B-Instruct-2507", name: "Qwen3 4B Instruct", reasoning: true, note: "Fast structured library automation" },
  { id: "HuggingFaceTB/SmolLM3-3B", name: "SmolLM3 3B", reasoning: true, note: "Compact open reasoning model" },
  { id: "deepseek-ai/DeepSeek-R1-Distill-Qwen-7B", name: "DeepSeek R1 Distill 7B", reasoning: true, note: "Deeper multi-step reasoning" },
  { id: "microsoft/Phi-3.5-mini-instruct", name: "Phi 3.5 Mini", reasoning: false, note: "Efficient instruction following" },
  { id: "katanemo/Arch-Router-1.5B:hf-inference", name: "Arch Router 1.5B", reasoning: true, note: "Routes tool and library tasks" },
] as const;
const BULK_METADATA_BATCH_LIMIT = 500;
const MAX_VISIBLE_PROVIDER_ERRORS = 40;

type QuickAction = {
  label: string;
  icon: LucideIcon;
  q: string;
  progressTask?: string;
  runNow?: () => Promise<any>;
};

type SourceDiscoveryResult = {
  type?: "source_discovery";
  status: string;
  roots_checked: number;
  discovered: number;
  added: number;
  existing: number;
  paths: string[];
  message: string;
};

type AiConfig = {
  model?: string;
  has_token?: boolean;
  default_model?: string;
  inference_url?: string;
};

type AdultMetadataGatherResult = {
  type: "adult_metadata_gather";
  status: string;
  configured_adult_providers?: string[];
  provider_count?: number;
  items_scanned?: number;
  items_reclassified_as_adult?: number;
  titles_refreshed_from_embedded?: number;
  metadata_items_enriched?: number;
  metadata_fields_updated?: number;
  sidecars_written?: number;
  posters_updated?: number;
  chapter_sets_generated?: number;
  chapter_images_generated?: number;
  items_needing_metadata?: number;
  skipped_missing_files?: number;
  skipped_non_video_items?: number;
  errors?: string[];
  note?: string;
};

type SingleItemMetadataCheckResult = {
  type: "single_item_metadata_check";
  status: string;
  item_id?: number;
  metadata_updated?: boolean;
  metadata_fields_updated?: number;
  provider_errors?: string[];
  message?: string;
  updated_item?: Partial<MediaItem>;
};

type BulkMetadataPostResult = {
  type: "bulk_metadata_post";
  status: string;
  items_scanned: number;
  candidates_considered: number;
  candidates_skipped_complete: number;
  batch_limit: number;
  metadata_items_enriched: number;
  metadata_fields_updated: number;
  posters_attached: number;
  provider_errors: string[];
  no_match: number;
  no_changes: number;
  failed: number;
  stopped_reason?: string;
};

function isLibraryEnrichmentResult(
  result: any,
): result is LibraryEnrichmentResult {
  return result?.type === "library_enrichment";
}

function isAdultMetadataGatherResult(
  result: any,
): result is AdultMetadataGatherResult {
  return result?.type === "adult_metadata_gather";
}

function isBulkMetadataPostResult(
  result: any,
): result is BulkMetadataPostResult {
  return result?.type === "bulk_metadata_post";
}

function itemNeedsMetadata(item: MediaItem): boolean {
  return (
    !item.overview?.trim() ||
    !item.poster_path?.trim() ||
    item.year == null ||
    item.rating == null ||
    !item.genre?.trim() ||
    !item.tmdb_id?.trim() ||
    !item.imdb_id?.trim()
  );
}

function trimProviderErrors(errors: string[]): string[] {
  if (errors.length <= MAX_VISIBLE_PROVIDER_ERRORS) return errors;
  return [
    ...errors.slice(0, MAX_VISIBLE_PROVIDER_ERRORS),
    `Additional provider messages hidden: ${errors.length - MAX_VISIBLE_PROVIDER_ERRORS}`,
  ];
}

function formatLibraryEnrichmentMessage(
  label: string,
  result: LibraryEnrichmentResult,
): string {
  return `${label}: scanned ${result.items_scanned || 0}, enriched ${result.metadata_items_enriched || 0}, updated ${result.metadata_fields_updated || 0} fields, renamed ${result.files_renamed || 0}`;
}

function formatAdultMetadataGatherMessage(
  label: string,
  result: AdultMetadataGatherResult,
): string {
  return `${label}: scanned ${result.items_scanned || 0}, enriched ${result.metadata_items_enriched || 0}, updated ${result.metadata_fields_updated || 0} fields, posters ${result.posters_updated || 0}, sidecars ${result.sidecars_written || 0}`;
}

function formatBulkMetadataPostMessage(
  label: string,
  result: BulkMetadataPostResult,
): string {
  const stop = result.stopped_reason ? ` (${result.stopped_reason})` : "";
  return `${label}: scanned ${result.items_scanned}, enriched ${result.metadata_items_enriched}, updated ${result.metadata_fields_updated} fields, posters attached ${result.posters_attached}${stop}`;
}

function formatResultSummary(result: any) {
  if (isBulkMetadataPostResult(result)) {
    return JSON.stringify(result, null, 2);
  }

  if (isAdultMetadataGatherResult(result)) {
    return JSON.stringify(
      {
        status: result.status,
        configured_adult_providers: result.configured_adult_providers,
        provider_count: result.provider_count,
        items_scanned: result.items_scanned,
        metadata_items_enriched: result.metadata_items_enriched,
        metadata_fields_updated: result.metadata_fields_updated,
        posters_updated: result.posters_updated,
        sidecars_written: result.sidecars_written,
        chapter_sets_generated: result.chapter_sets_generated,
        chapter_images_generated: result.chapter_images_generated,
        items_reclassified_as_adult: result.items_reclassified_as_adult,
        titles_refreshed_from_embedded: result.titles_refreshed_from_embedded,
        items_needing_metadata: result.items_needing_metadata,
        skipped_missing_files: result.skipped_missing_files,
        skipped_non_video_items: result.skipped_non_video_items,
        errors: trimProviderErrors(result.errors || []),
        note: result.note,
      },
      null,
      2,
    );
  }

  if (!isLibraryEnrichmentResult(result)) {
    return JSON.stringify(result, null, 2);
  }

  return JSON.stringify(
    {
      status: result.status,
      mode: result.mode,
      items_scanned: result.items_scanned,
      metadata_items_enriched: result.metadata_items_enriched,
      metadata_fields_updated: result.metadata_fields_updated,
      metadata_updated: result.metadata_updated,
      titles_improved: result.titles_improved,
      items_reclassified_as_adult: result.items_reclassified_as_adult,
      files_renamed: result.files_renamed,
      rename_collisions_skipped: result.rename_collisions_skipped,
      rename_failures: result.rename_failures,
      low_confidence_metadata_only: result.low_confidence_metadata_only,
      skipped_missing_files: result.skipped_missing_files,
      skipped_non_video_items: result.skipped_non_video_items,
      provider_errors: trimProviderErrors(result.provider_errors || []),
    },
    null,
    2,
  );
}

export default function AIDiagnosticsTab() {
  const {
    aiProcessing,
    setAiProcessing,
    aiResult,
    setAiResult,
    addStatusMessage,
    setMediaItems,
  } = useAppStore();
  const [prompt, setPrompt] = useState("");
  const [imageUrl, setImageUrl] = useState("");
  const [hfToken, setHfToken] = useState("");
  const [hasHfToken, setHasHfToken] = useState(false);
  const [model, setModel] = useState(DEFAULT_HF_MODEL);
  const [inferenceUrl, setInferenceUrl] = useState(
    "https://router.huggingface.co/v1/chat/completions",
  );
  const [showConfig, setShowConfig] = useState(false);
  const [showModelCatalog, setShowModelCatalog] = useState(false);
  const [history, setHistory] = useState<
    { query: string; result: any; time: string }[]
  >([]);
  const [metadataProgress, setMetadataProgress] =
    useState<MetadataTaskProgress | null>(null);

  const loadAiConfig = useCallback(async () => {
    try {
      // Recover a previously saved environment or Hugging Face CLI credential
      // before reading status. The backend persists any recovered token in the
      // app database so upgrades and reinstalls keep AI inference configured.
      await invoke("ensure_hf_token");
      const config = await invoke<AiConfig>("get_ai_config");
      setModel(config.model || config.default_model || DEFAULT_HF_MODEL);
      setHasHfToken(Boolean(config.has_token));
      setInferenceUrl(
        config.inference_url ||
          "https://router.huggingface.co/v1/chat/completions",
      );
    } catch (error) {
      addStatusMessage(`AI config load failed: ${error}`);
    }
  }, [addStatusMessage]);

  useEffect(() => {
    void loadAiConfig();
  }, [loadAiConfig]);

  useEffect(() => {
    if (showConfig) void loadAiConfig();
  }, [showConfig, loadAiConfig]);

  const refreshLoadedLibraryPage = useCallback(async () => {
    const items = await invoke<MediaItem[]>(
      "get_media_items",
      buildLibraryPageRequest({}),
    );
    setMediaItems(items);
  }, [setMediaItems]);

  const loadAllLibraryItems = useCallback(async () => {
    const allItems: MediaItem[] = [];
    let offset = 0;
    let guard = 0;

    while (guard < 1000) {
      const page = await invoke<MediaItem[]>(
        "get_media_items",
        buildLibraryPageRequest({ offset }),
      );
      allItems.push(...page);
      if (!hasMoreLibraryPages(page)) break;
      offset += LIBRARY_PAGE_SIZE;
      guard += 1;
    }

    return allItems;
  }, []);

  useEffect(() => {
    if (!aiProcessing) return;

    let cancelled = false;
    const pollProgress = async () => {
      try {
        const progress = await invoke<MetadataTaskProgress>(
          "get_metadata_task_progress",
        );
        if (!cancelled && metadataTaskPopupVisible(progress)) {
          setMetadataProgress(progress);
        }
      } catch {}
    };

    pollProgress();
    const timer = window.setInterval(pollProgress, 500);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [aiProcessing]);

  const formattedProgress = useMemo(
    () => formatMetadataTaskProgress(metadataProgress, "Metadata Task"),
    [metadataProgress],
  );
  const showMetadataProgress = metadataTaskPopupVisible(metadataProgress);

  const showStartingProgress = (
    label: string,
    task = "metadata_task",
    total = 1,
  ) => {
    setMetadataProgress({
      active: true,
      task,
      label,
      current: 0,
      total,
      percent: 0,
      message: `Starting ${label}...`,
    });
  };

  const stopAiAgent = async () => {
    try {
      await invoke("stop_ai_agent");
      addStatusMessage("AI agent stop requested; the current item will finish safely.");
    } catch (error) {
      addStatusMessage(`Unable to stop AI agent: ${error}`);
    }
  };

  const updateLocalProgress = (
    label: string,
    task: string,
    current: number,
    total: number,
    message: string,
  ) => {
    const percent =
      total > 0 ? Math.min(99, Math.floor((current * 100) / total)) : 0;
    setMetadataProgress({
      active: true,
      task,
      label,
      current,
      total,
      percent,
      message,
    });
  };

  const showFinishedProgress = (
    label: string,
    message = `${label} complete`,
  ) => {
    setMetadataProgress((prev) => ({
      ...prev,
      active: false,
      task: prev?.task || "metadata_task",
      label: prev?.label || label,
      current: prev?.total || 1,
      total: prev?.total || 1,
      percent: 100,
      message,
    }));
    window.setTimeout(() => {
      setMetadataProgress((current) => (current?.active ? current : null));
    }, 3500);
  };

  const runBulkMetadataPost =
    useCallback(async (): Promise<BulkMetadataPostResult> => {
      const label = "Post Metadata & Posters";
      const task = "bulk_metadata_post";
      const items = await loadAllLibraryItems();
      const eligible = items.filter((item) => typeof item.id === "number");
      const candidates = eligible
        .filter(itemNeedsMetadata)
        .slice(0, BULK_METADATA_BATCH_LIMIT);
      const skippedComplete =
        eligible.length - eligible.filter(itemNeedsMetadata).length;
      const result: BulkMetadataPostResult = {
        type: "bulk_metadata_post",
        status: "success",
        items_scanned: 0,
        candidates_considered: eligible.length,
        candidates_skipped_complete: skippedComplete,
        batch_limit: BULK_METADATA_BATCH_LIMIT,
        metadata_items_enriched: 0,
        metadata_fields_updated: 0,
        posters_attached: 0,
        provider_errors: [],
        no_match: 0,
        no_changes: 0,
        failed: 0,
        stopped_reason:
          candidates.length === 0
            ? "No incomplete metadata candidates found"
            : undefined,
      };

      if (candidates.length === 0) {
        return result;
      }

      showStartingProgress(label, task, candidates.length);
      for (let index = 0; index < candidates.length; index += 1) {
        const item = candidates[index];
        updateLocalProgress(
          label,
          task,
          index + 1,
          candidates.length,
          `Posting metadata for ${index + 1} of ${candidates.length}`,
        );

        try {
          const check = await invoke<SingleItemMetadataCheckResult>(
            "check_media_item_metadata",
            { id: item.id },
          );
          result.items_scanned += 1;
          const changed = check.metadata_fields_updated || 0;
          if (check.metadata_updated || changed > 0) {
            result.metadata_items_enriched += 1;
            result.metadata_fields_updated += changed;
          }
          if (
            !item.poster_path?.trim() &&
            check.updated_item?.poster_path?.trim()
          ) {
            result.posters_attached += 1;
          }
          if (check.status === "no_match") {
            result.no_match += 1;
          } else if (check.status === "no_changes") {
            result.no_changes += 1;
          }
          if (check.provider_errors?.length) {
            result.provider_errors.push(...check.provider_errors);
            result.provider_errors = trimProviderErrors(result.provider_errors);
          }
        } catch (error) {
          result.items_scanned += 1;
          result.failed += 1;
          result.provider_errors.push(
            `${item.title || item.file_path}: ${String(error)}`,
          );
          result.provider_errors = trimProviderErrors(result.provider_errors);
        }
      }

      if (
        eligible.filter(itemNeedsMetadata).length > BULK_METADATA_BATCH_LIMIT
      ) {
        result.stopped_reason = `Batch limit reached; run again to continue remaining items`;
      }
      if (result.failed > 0) {
        result.status = "partial";
      }
      return result;
    }, [loadAllLibraryItems]);

  const handleTrackedResult = async (
    label: string,
    query: string,
    result: any,
  ) => {
    setAiResult(result);
    setHistory((prev) => [
      { query, result, time: new Date().toLocaleTimeString() },
      ...prev.slice(0, 19),
    ]);
    let message = `${label} complete`;
    if (isBulkMetadataPostResult(result)) {
      message = formatBulkMetadataPostMessage(label, result);
      await refreshLoadedLibraryPage();
    } else if (isLibraryEnrichmentResult(result)) {
      message = formatLibraryEnrichmentMessage(label, result);
      await refreshLoadedLibraryPage();
    } else if (isAdultMetadataGatherResult(result)) {
      message = formatAdultMetadataGatherMessage(label, result);
      await refreshLoadedLibraryPage();
    } else if (result?.type === "source_discovery" || result?.paths) {
      const discovery = result as SourceDiscoveryResult;
      message = `${label}: found ${discovery.discovered || 0}, added ${discovery.added || 0}, already configured ${discovery.existing || 0}`;
      await refreshLoadedLibraryPage();
    } else if (result?.type === "ai_library_manage") {
      message = `${label}: ${result.status || "complete"}, ${result.total_updated || 0} real updates`;
      await refreshLoadedLibraryPage();
    } else if (result?.type === "purge_photo_items") {
      message =
        result.message ||
        `Removed ${result.rows_removed ?? 0} photo/poster items from library`;
      await refreshLoadedLibraryPage();
    }
    addStatusMessage(message);
    showFinishedProgress(label, message);
  };

  const runQuery = async () => {
    const cleanPrompt = prompt.trim();
    if (!cleanPrompt) return;

    const wantsBulkMetadata =
      /(gather metadata|enrich metadata|post metadata|attach posters|poster artwork|metadata posters)/i.test(
        cleanPrompt,
      ) && !/adult metadata|adult providers|chapter images/i.test(cleanPrompt);

    if (wantsBulkMetadata) {
      const label = "Post Metadata & Posters";
      showStartingProgress(label, "bulk_metadata_post");
      setAiProcessing(true);
      addStatusMessage(`Running: ${label}...`);
      try {
        const result = await runBulkMetadataPost();
        await handleTrackedResult(label, cleanPrompt, result);
      } catch (e) {
        const errResult = { status: "error", message: String(e) };
        setAiResult(errResult);
        setHistory((prev) => [
          {
            query: cleanPrompt,
            result: errResult,
            time: new Date().toLocaleTimeString(),
          },
          ...prev.slice(0, 19),
        ]);
        addStatusMessage(`${label} failed: ${e}`);
        showFinishedProgress(label, `${label} failed: ${e}`);
      } finally {
        setAiProcessing(false);
        setPrompt("");
      }
      return;
    }

    const tracksAdultGather =
      /adult metadata|chapter images|adult providers/i.test(cleanPrompt);
    if (tracksAdultGather) {
      const label = "Adult Metadata Gather";
      showStartingProgress(label, "adult_metadata_gather");
      setAiProcessing(true);
      addStatusMessage(`Running: ${label}...`);
      try {
        const result = await invoke<AdultMetadataGatherResult>("ai_query", {
          prompt: cleanPrompt,
        });
        await handleTrackedResult(label, cleanPrompt, result);
      } catch (e) {
        const errResult = { status: "error", message: String(e) };
        setAiResult(errResult);
        setHistory((prev) => [
          {
            query: cleanPrompt,
            result: errResult,
            time: new Date().toLocaleTimeString(),
          },
          ...prev.slice(0, 19),
        ]);
        addStatusMessage(`${label} failed: ${e}`);
        showFinishedProgress(label, `${label} failed: ${e}`);
      } finally {
        setAiProcessing(false);
        setPrompt("");
      }
      return;
    }

    setAiProcessing(true);
    addStatusMessage(`AI processing: ${cleanPrompt.substring(0, 50)}...`);
    try {
      const result = await invoke<any>("ai_query", { prompt: cleanPrompt });
      setAiResult(result);
      setHistory((prev) => [
        { query: cleanPrompt, result, time: new Date().toLocaleTimeString() },
        ...prev.slice(0, 19),
      ]);
      addStatusMessage("AI query complete");
    } catch (e) {
      const errResult = { status: "error", message: String(e) };
      setAiResult(errResult);
      setHistory((prev) => [
        {
          query: cleanPrompt,
          result: errResult,
          time: new Date().toLocaleTimeString(),
        },
        ...prev.slice(0, 19),
      ]);
      addStatusMessage(`AI error: ${e}`);
    }
    setAiProcessing(false);
    setPrompt("");
  };

  const runInference = async () => {
    if (!prompt.trim()) return;
    setAiProcessing(true);
    try {
      const result = await invoke<any>("ai_inference", {
        input: prompt,
        model,
        imageUrl: imageUrl.trim() || null,
      });
      setAiResult(result);
      setHistory((prev) => [
        {
          query: `[Inference] ${prompt}`,
          result,
          time: new Date().toLocaleTimeString(),
        },
        ...prev.slice(0, 19),
      ]);
      addStatusMessage("AI inference complete");
      await loadAiConfig();
    } catch (e) {
      addStatusMessage(`Inference failed: ${e}`);
    }
    setAiProcessing(false);
  };

  const saveToken = async () => {
    try {
      await invoke("set_hf_token", { token: hfToken });
      setHasHfToken(Boolean(hfToken.trim()));
      setHfToken("");
      addStatusMessage(
        "Hugging Face token saved and retained for AI inference",
      );
      await loadAiConfig();
    } catch (e) {
      addStatusMessage(`Failed: ${e}`);
    }
  };

  const saveModel = async () => {
    try {
      await invoke("set_ai_model", { model });
      addStatusMessage(`AI model set to: ${model}`);
      await loadAiConfig();
    } catch (e) {
      addStatusMessage(`Failed: ${e}`);
    }
  };

  const openLink = (url: string) => {
    window.open(url, "_blank", "noopener,noreferrer");
  };

  const runQuickAction = async (action: QuickAction) => {
    if (aiProcessing) return;
    setPrompt(action.q);
    if (!action.runNow) return;

    if (action.progressTask) {
      showStartingProgress(action.label, action.progressTask);
    }

    setAiProcessing(true);
    addStatusMessage(`Running: ${action.label}...`);
    try {
      const result = await action.runNow();
      await handleTrackedResult(action.label, action.q, result);
    } catch (e) {
      addStatusMessage(`${action.label} failed: ${e}`);
      if (action.progressTask) {
        showFinishedProgress(action.label, `${action.label} failed: ${e}`);
      }
    } finally {
      setAiProcessing(false);
    }
  };

  const quickActions: QuickAction[] = [
    {
      label: "Network Diagnostics",
      icon: Network,
      q: "Run network diagnostics",
      runNow: () => invoke("ai_query", { prompt: "Run network diagnostics" }),
    },
    {
      label: "Check Sources",
      icon: FolderSearch,
      q: "Check all media sources",
      runNow: () => invoke("ai_query", { prompt: "Check all media sources" }),
    },
    {
      label: "Discover Media Sources",
      icon: FolderSearch,
      q: "Discover media sources",
      runNow: () => invoke("discover_media_sources"),
    },
    {
      label: "Check Providers",
      icon: Database,
      q: "Check metadata providers",
      runNow: () => invoke("ai_query", { prompt: "Check metadata providers" }),
    },
    {
      label: "Run Full AI Library Management",
      icon: Brain,
      q: "Scan, enrich metadata, normalize titles and filenames, attach posters, write NFO files, and analyze duplicates",
      progressTask: "ai_library_manage",
      runNow: () => invoke("ai_library_manage", { tasks: null }),
    },
    {
      label: "Post Metadata & Posters",
      icon: Sparkles,
      q: "Post metadata and attach posters to all incomplete media files",
      progressTask: "bulk_metadata_post",
      runNow: runBulkMetadataPost,
    },
    {
      label: "Enrich Library Metadata",
      icon: Sparkles,
      q: "Enrich Library Metadata",
      progressTask: "library_enrichment",
      runNow: () => invoke("run_library_enrichment", { renameFiles: false }),
    },
    {
      label: "Apply Embedded Titles",
      icon: Tag,
      q: "Apply embedded titles to existing library",
      runNow: () => invoke("apply_embedded_titles"),
    },
    {
      label: "Enrich + Normalize Filenames",
      icon: Tag,
      q: "Enrich + Normalize Filenames",
      progressTask: "library_enrichment",
      runNow: () => invoke("run_library_enrichment", { renameFiles: true }),
    },
    {
      label: "Adult Metadata Gather",
      icon: Sparkles,
      q: "Run adult metadata gather for installed providers and generate posters and chapter images",
      progressTask: "adult_metadata_gather",
      runNow: () =>
        invoke("ai_query", {
          prompt:
            "Run adult metadata gather for installed providers and generate posters and chapter images",
        }),
    },
    {
      label: "Purge Photo Items",
      icon: Trash2,
      q: "Purge all photo/poster items incorrectly listed as standalone media",
      runNow: () => invoke("purge_photo_items"),
    },
  ];

  return (
    <div className="space-y-5">
      {showMetadataProgress && (
        <motion.div
          initial={{ opacity: 0, y: 16, scale: 0.98 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 16, scale: 0.98 }}
          className="fixed right-5 bottom-5 z-[90] w-[min(360px,calc(100vw-2rem))] glass-panel p-4 border border-cv-accent/25 shadow-2xl"
        >
          <div className="flex items-start justify-between gap-4">
            <div className="min-w-0">
              <div className="text-sm font-bold text-cv-text truncate">
                {formattedProgress.label}
              </div>
              <div className="text-[11px] text-cv-subtext mt-1 truncate">
                {formattedProgress.message}
              </div>
            </div>
            <div className="text-xl font-bold text-cv-accent tabular-nums">
              {formattedProgress.percent}%
            </div>
          </div>
          <div className="mt-3 h-2 rounded-full bg-white/10 overflow-hidden">
            <motion.div
              className="h-full rounded-full"
              style={{
                background:
                  "linear-gradient(90deg, var(--cv-accent), var(--cv-neon-1))",
              }}
              animate={{ width: `${formattedProgress.percent}%` }}
              transition={{ duration: 0.25 }}
            />
          </div>
          {formattedProgress.total > 0 && (
            <div className="mt-2 text-[10px] text-cv-subtext tabular-nums">
              {formattedProgress.current} / {formattedProgress.total} items
            </div>
          )}
        </motion.div>
      )}

      <div
        className="glass-panel p-5 relative overflow-hidden"
        style={{ minHeight: 280 }}
      >
        <div className="absolute inset-0 z-0">
          <AIVisualizer active={aiProcessing} />
        </div>
        <div className="relative z-10">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-bold flex items-center gap-2">
              <Brain size={16} className="text-cv-accent" /> AI Agent
            </h3>
            <button
              onClick={() => setShowConfig(!showConfig)}
              className="cv-btn cv-btn-secondary text-xs"
            >
              <Settings size={12} /> Configure
            </button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3 mb-4">
            <div className="glass-panel-2 p-3 rounded-xl">
              <div className="text-[10px] uppercase tracking-[0.2em] text-cv-subtext">
                HF Token
              </div>
              <div className="mt-1 flex items-center gap-2 text-sm font-bold">
                <ShieldCheck
                  size={14}
                  className={hasHfToken ? "text-emerald-300" : "text-amber-300"}
                />
                {hasHfToken ? "Configured" : "Missing"}
              </div>
            </div>
            <div className="glass-panel-2 p-3 rounded-xl md:col-span-2">
              <div className="text-[10px] uppercase tracking-[0.2em] text-cv-subtext">
                Model
              </div>
              <div className="mt-1 truncate text-sm font-bold text-cv-text">
                {model}
              </div>
              <button onClick={() => setShowModelCatalog(true)} className="cv-btn cv-btn-gold mt-3 text-xs">
                <Sparkles size={12} /> Select Free HF Model
              </button>
            </div>
          </div>

          <div className="flex gap-3 mt-auto pt-20">
            <input
              type="text"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && runQuery()}
              placeholder="Ask AI anything... or type: gather metadata / attach posters"
              className="cv-input flex-1 bg-black/40"
            />
            <button
              onClick={runQuery}
              disabled={aiProcessing}
              className="cv-btn cv-btn-primary"
            >
              {aiProcessing ? (
                <Loader size={14} className="animate-spin" />
              ) : (
                <Send size={14} />
              )}
              Query
            </button>
            <button
              onClick={runInference}
              disabled={aiProcessing || !hasHfToken}
              className="cv-btn cv-btn-gold disabled:opacity-45 disabled:cursor-not-allowed"
              title={
                !hasHfToken
                  ? "Save a Hugging Face token before inference"
                  : undefined
              }
            >
              <Sparkles size={14} /> Inference
            </button>
            {aiProcessing && (
              <button
                onClick={() => void stopAiAgent()}
                className="cv-btn cv-btn-danger"
                title="Stop the active AI or metadata operation"
              >
                <Square size={14} /> Stop AI Agent
              </button>
            )}
          </div>
          <div className="mt-2">
            <input
              type="text"
              value={imageUrl}
              onChange={(e) => setImageUrl(e.target.value)}
              placeholder="Optional image URL for multimodal query (jpg/png/webp)"
              className="cv-input w-full bg-black/30 text-xs"
            />
          </div>

          <div className="flex flex-wrap gap-2 mt-3">
            {quickActions.map((action) => (
              <button
                key={action.label}
                disabled={aiProcessing}
                onClick={() => runQuickAction(action)}
                className="cv-btn cv-btn-secondary text-[10px] py-1 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <action.icon size={10} /> {action.label}
              </button>
            ))}
          </div>
          <button
            disabled={aiProcessing}
            onClick={async () => {
              if (!window.confirm("Mark every item currently indexed in CinaVault as adult? Existing poster and backdrop references will be preserved. Future imports will continue to use normal classification.")) return;
              setAiProcessing(true);
              try {
                const result = await invoke<any>("convert_entire_library_to_adult");
                await handleTrackedResult("Convert Entire Library to Adult", "Convert entire library to adult", result);
                await refreshLoadedLibraryPage();
              } catch (error) {
                addStatusMessage(`Adult library conversion failed: ${error}`);
              } finally {
                setAiProcessing(false);
              }
            }}
            className="cv-btn cv-btn-danger mt-3 text-xs disabled:opacity-50"
          >
            Mark Current Inventory Adult (CinaVault Only)
          </button>
        </div>
      </div>

      {showConfig && (
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          className="glass-panel p-5"
        >
          <h3 className="text-sm font-bold mb-4 flex items-center gap-2">
            <Cpu size={16} className="text-cv-accent" /> AI Model Configuration
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="section-label">Hugging Face Token</label>
              <div className="flex gap-2">
                <input
                  type="password"
                  value={hfToken}
                  onChange={(e) => setHfToken(e.target.value)}
                  className="cv-input flex-1"
                  placeholder={
                    hasHfToken ? "Token already configured" : "hf_..."
                  }
                />
                <button
                  onClick={saveToken}
                  className="cv-btn cv-btn-primary text-xs"
                >
                  <Key size={12} /> Save
                </button>
                <button
                  onClick={() =>
                    openLink("https://huggingface.co/settings/tokens")
                  }
                  className="cv-btn cv-btn-secondary text-xs"
                >
                  <ExternalLink size={12} /> Get API Key
                </button>
              </div>
              <div className="text-[10px] text-cv-subtext mt-1">
                Status: {hasHfToken ? "configured and retained" : "missing"}.
                Existing tokens are never displayed.
              </div>
            </div>
            <div>
              <label className="section-label">AI Model</label>
              <div className="flex gap-2">
                <input
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  className="cv-input flex-1"
                  placeholder={DEFAULT_HF_MODEL}
                />
                <button
                  onClick={saveModel}
                  className="cv-btn cv-btn-primary text-xs"
                >
                  <Cpu size={12} /> Set
                </button>
                <button
                  onClick={() => setShowModelCatalog(true)}
                  className="cv-btn cv-btn-gold text-xs"
                >
                  <Sparkles size={12} /> Browse Free Models
                </button>
              </div>
              <div className="text-[10px] text-cv-subtext mt-1">
                Default: {DEFAULT_HF_MODEL}
              </div>
            </div>
          </div>
          <div className="mt-3 text-[10px] text-cv-subtext">
            Inference URL: {inferenceUrl}
          </div>
        </motion.div>
      )}

      {showModelCatalog && (
        <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/80 p-4" role="dialog" aria-modal="true" aria-label="Hugging Face model selection">
          <motion.div initial={{ opacity: 0, y: 16 }} animate={{ opacity: 1, y: 0 }} className="glass-panel w-full max-w-3xl p-5">
            <div className="mb-4 flex items-start justify-between gap-4">
              <div><h3 className="text-base font-bold">Hugging Face Free Model Catalog</h3><p className="mt-1 text-xs text-cv-subtext">Public, ungated choices only. Reasoning-capable models are labeled.</p></div>
              <button onClick={() => setShowModelCatalog(false)} className="cv-btn cv-btn-secondary text-xs">Close</button>
            </div>
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              {HF_FREE_MODELS.map((candidate) => (
                <button key={candidate.id} onClick={() => { setModel(candidate.id); setShowModelCatalog(false); }} className={`glass-panel-2 rounded-xl border p-4 text-left transition-colors ${model === candidate.id ? "border-cv-accent" : "border-white/10 hover:border-white/25"}`}>
                  <div className="flex items-center justify-between gap-2"><span className="text-sm font-bold">{candidate.name}</span>{candidate.reasoning && <span className="rounded bg-cv-accent/15 px-2 py-1 text-[9px] font-bold text-cv-accent">REASONING</span>}</div>
                  <div className="mt-2 break-all font-mono text-[10px] text-cv-subtext">{candidate.id}</div>
                  <div className="mt-2 text-xs text-cv-subtext">{candidate.note}</div>
                </button>
              ))}
            </div>
            <div className="mt-4 text-[10px] text-cv-subtext">Selecting a card fills the model field. Press Set to save it permanently.</div>
          </motion.div>
        </div>
      )}

      {(aiResult || history.length > 0) && (
        <div className="glass-panel p-5">
          <h3 className="text-sm font-bold mb-3">AI Activity Log</h3>

          {aiResult && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="glass-panel-2 p-4 rounded-lg mb-3"
            >
              <div className="text-xs font-semibold mb-2 text-cv-accent">
                Latest Result
              </div>
              <pre className="text-xs text-cv-subtext whitespace-pre-wrap font-mono max-h-48 overflow-y-auto">
                {formatResultSummary(aiResult)}
              </pre>
            </motion.div>
          )}

          {history.length > 0 && (
            <div className="space-y-1 max-h-60 overflow-y-auto">
              {history.map((entry, i) => (
                <div
                  key={i}
                  className="flex items-start gap-3 py-2 px-3 rounded hover:bg-white/[0.02] text-xs"
                >
                  <span className="text-cv-subtext shrink-0">{entry.time}</span>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium truncate">{entry.query}</div>
                    <div className="text-cv-subtext text-[10px] truncate">
                      {entry.result?.status || "completed"} —{" "}
                      {entry.result?.type || "inference"}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
