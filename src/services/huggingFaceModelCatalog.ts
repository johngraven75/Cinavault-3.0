export type HuggingFaceModel = {
  id: string;
  pipelineTag?: string;
  libraryName?: string;
  downloads: number;
  likes: number;
  lastModified?: string;
  private: boolean;
  gated: boolean | string;
  disabled?: boolean;
  tags: string[];
};

export type HuggingFaceCatalogueQuery = {
  search?: string;
  task?: string;
  limit?: number;
};

const HUGGING_FACE_MODELS_API = "https://huggingface.co/api/models";
const DEFAULT_TASK = "text-generation";
const DEFAULT_LIMIT = 50;

const REASONING_MARKERS = [
  "reasoning",
  "reasoner",
  "thinking",
  "instruct",
  "instruction",
  "chat",
  "assistant",
  "cot",
  "chain-of-thought",
  "distill",
  "r1",
  "math",
  "logic",
  "tool-use",
  "function-calling",
];

const BASE_MODEL_MARKERS = [
  "base",
  "pretrain",
  "pretrained",
  "foundation",
  "raw",
];

function searchableModelText(model: HuggingFaceModel): string {
  return [model.id, model.pipelineTag, model.libraryName, ...(model.tags || [])]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
}

export function isPublicUngatedModel(model: HuggingFaceModel): boolean {
  return (
    model.private !== true &&
    model.disabled !== true &&
    model.gated !== true &&
    model.gated !== "auto" &&
    model.gated !== "manual"
  );
}

export function reasoningCapabilityScore(model: HuggingFaceModel): number {
  const text = searchableModelText(model);
  let score = 0;

  for (const marker of REASONING_MARKERS) {
    if (text.includes(marker)) score += 2;
  }

  if (model.pipelineTag === "text-generation") score += 1;
  if (text.includes("conversational")) score += 2;
  if (text.includes("text-generation-inference")) score += 1;

  for (const marker of BASE_MODEL_MARKERS) {
    if (text.includes(marker)) score -= 4;
  }

  return score;
}

export function isReasoningCapableModel(model: HuggingFaceModel): boolean {
  const text = searchableModelText(model);
  const explicitlyBase = BASE_MODEL_MARKERS.some((marker) =>
    text.includes(marker),
  );

  return !explicitlyBase && reasoningCapabilityScore(model) >= 3;
}

export function normalizeCatalogueModels(
  models: HuggingFaceModel[],
): HuggingFaceModel[] {
  return models
    .filter(isPublicUngatedModel)
    .filter(isReasoningCapableModel)
    .sort((a, b) => {
      const reasoningDelta =
        reasoningCapabilityScore(b) - reasoningCapabilityScore(a);
      if (reasoningDelta !== 0) return reasoningDelta;

      const popularityA = (a.downloads || 0) + (a.likes || 0) * 100;
      const popularityB = (b.downloads || 0) + (b.likes || 0) * 100;
      return popularityB - popularityA;
    });
}

export async function fetchPublicHuggingFaceModels(
  query: HuggingFaceCatalogueQuery = {},
  signal?: AbortSignal,
): Promise<HuggingFaceModel[]> {
  const params = new URLSearchParams({
    pipeline_tag: query.task || DEFAULT_TASK,
    sort: "downloads",
    direction: "-1",
    limit: String(Math.min(Math.max(query.limit || DEFAULT_LIMIT, 1), 100)),
    full: "true",
  });

  if (query.search?.trim()) {
    params.set("search", query.search.trim());
  }

  const response = await fetch(`${HUGGING_FACE_MODELS_API}?${params}`, {
    method: "GET",
    headers: { Accept: "application/json" },
    signal,
  });

  if (!response.ok) {
    throw new Error(`Hugging Face catalogue request failed (${response.status})`);
  }

  const payload = (await response.json()) as HuggingFaceModel[];
  return normalizeCatalogueModels(Array.isArray(payload) ? payload : []);
}
