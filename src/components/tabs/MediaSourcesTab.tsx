import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { motion } from "framer-motion";
import { useAppStore, type LibraryEnrichmentResult } from "../../store/appStore";
import { ExternalLink, File, FolderOpen, HardDrive, Link, Plus, RefreshCw, Scan, Sparkles, Trash2 } from "lucide-react";

type ScanResult = { status?: string; total_found?: number | string; total_added?: number | string; total_updated?: number | string; sources_scanned?: number | string; sources_failed?: number | string; errors?: string[] };
type SourceLike = { id?: number; path: string; source_type: string; name: string; enabled: boolean; last_scanned?: string; item_count: number };
const DEFAULT_METADATA_AFTER_SCAN = true;

function safeNumber(value: unknown): number {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.trim()) { const parsed = Number(value); return Number.isFinite(parsed) ? parsed : 0; }
  return 0;
}
function formatMetadataSummary(result: LibraryEnrichmentResult): string {
  const enriched = result.metadata_items_enriched || result.metadata_updated || 0;
  const fields = result.metadata_fields_updated || 0;
  const posters = result.posters_downloaded || 0;
  const warnings = result.provider_errors?.length || 0;
  return [`${enriched} items enriched`, `${fields} metadata fields updated`, `${posters} posters downloaded`, warnings ? `${warnings} provider warnings` : "no provider warnings"].join(", ");
}
function sourceIcon(sourceType: string) {
  if (sourceType === "drive") return <HardDrive size={18} className="text-cv-accent" />;
  if (sourceType === "file") return <File size={18} className="text-cv-accent" />;
  return <FolderOpen size={18} className="text-cv-accent" />;
}

export default function MediaSourcesTab() {
  const { sources, setSources, scanning, setScanning, scanProgress, addStatusMessage, settings, setSetting, scheduledTasks } = useAppStore();
  const [newSourcePath, setNewSourcePath] = useState("");
  const [newSourceName, setNewSourceName] = useState("");
  const [newSourceType, setNewSourceType] = useState("folder");
  const [webLink, setWebLink] = useState("");
  const [savingOption, setSavingOption] = useState<string | null>(null);
  const [discovering, setDiscovering] = useState(false);
  const [addingSource, setAddingSource] = useState(false);

  const browseForSource = async () => {
    try {
      const selected = await open({
        directory: newSourceType !== "file",
        multiple: false,
        title: newSourceType === "file" ? "Select media file" : "Select media folder or external drive",
      });
      if (typeof selected === "string" && selected.trim()) {
        setNewSourcePath(selected);
        if (!newSourceName.trim()) {
          setNewSourceName(selected.split(/[\\/]/).filter(Boolean).pop() || selected);
        }
      }
    } catch (error) {
      addStatusMessage(`Source picker failed: ${error}`);
    }
  };

  const loadSources = async (): Promise<SourceLike[]> => {
    try { const loaded = await invoke<SourceLike[]>("get_sources"); setSources(loaded); return loaded; }
    catch (error) { setSources([]); addStatusMessage(`Configured sources unavailable: ${error}`); return []; }
  };
  const refreshLibrary = (reason: string) => {
    window.dispatchEvent(new CustomEvent("cinavault:library-refresh", { detail: { reason } }));
    addStatusMessage("Library refresh queued; media cards and posters will reload in pages");
  };
  useEffect(() => { void loadSources(); }, []);
  const isEnabled = (key: string, defaultOn = false) => (settings[key] ?? (defaultOn ? "true" : "false")) === "true";
  const shouldPullMetadataAfterScan = (result: ScanResult) => safeNumber(result.total_found) > 0 && (isEnabled("library_auto_scan", DEFAULT_METADATA_AFTER_SCAN) || scheduledTasks.metadata_check === "on_scan");
  const pullMetadataAfterScan = async (result: ScanResult) => {
    if (!shouldPullMetadataAfterScan(result)) { addStatusMessage("Metadata pull skipped: automatic metadata after scan is disabled or no media was found"); return; }
    addStatusMessage("AI is identifying media and retrieving posters...");
    const enrichment = await invoke<LibraryEnrichmentResult>("run_library_enrichment", { renameFiles: false });
    addStatusMessage(`AI enrichment complete: ${formatMetadataSummary(enrichment)}`);
  };
  const finishPipeline = async (result: ScanResult, reason: string) => {
    if (result.errors?.length) addStatusMessage(`Scan warnings: ${result.errors.slice(0, 3).join("; ")}`);
    await pullMetadataAfterScan(result);
    await loadSources();
    refreshLibrary(reason);
  };
  const runSourcePipeline = async (sourceId: number, sourceName: string) => {
    setScanning(true); addStatusMessage(`Scanning source: ${sourceName}`);
    try {
      const result = await invoke<ScanResult>("scan_single_source", { sourceId });
      addStatusMessage(`Source scan complete: ${safeNumber(result.total_found)} found, ${safeNumber(result.total_added)} added, ${safeNumber(result.total_updated)} refreshed`);
      await finishPipeline(result, "single-source-scan");
      window.dispatchEvent(new Event("cinavault:source-added"));
    } catch (error) { addStatusMessage(`Source pipeline failed: ${error}`); }
    finally { setScanning(false); }
  };
  const addSource = async () => {
    const path = newSourcePath.trim(); if (!path || addingSource) return;
    const name = newSourceName.trim() || path.split(/[\\/]/).filter(Boolean).pop() || (newSourceType === "drive" ? path : "New Source");
    setAddingSource(true);
    try {
      const health = await invoke<{ readable: boolean; message: string }>("validate_source_path", { path, sourceType: newSourceType });
      if (!health.readable) throw new Error(health.message);
      const sourceId = await invoke<number>("add_source", { path, sourceType: newSourceType, name });
      setNewSourcePath(""); setNewSourceName(""); addStatusMessage(`Source added: ${name}`); await loadSources(); await runSourcePipeline(sourceId, name);
    } catch (error) { addStatusMessage(`Failed to add source: ${error}`); }
    finally { setAddingSource(false); }
  };
  const removeSource = async (id: number) => {
    try { await invoke("remove_source", { id }); addStatusMessage("Source removed"); await loadSources(); refreshLibrary("source-removed"); }
    catch (error) { addStatusMessage(`Failed to remove source: ${error}`); }
  };
  const exploreSource = async (source: SourceLike) => {
    try {
      await invoke("explore_source_path", { path: source.path });
      addStatusMessage(`Opened source: ${source.name}`);
    } catch (error) {
      addStatusMessage(`Failed to explore ${source.name}: ${error}`);
    }
  };
  const scanAll = async () => {
    if (scanning) return; setScanning(true); addStatusMessage("Scanning all configured sources...");
    try {
      const result = await invoke<ScanResult>("scan_sources");
      addStatusMessage(`Scan complete: ${safeNumber(result.total_added)} new, ${safeNumber(result.total_updated)} refreshed from ${safeNumber(result.sources_scanned)} sources${safeNumber(result.sources_failed) ? `; ${safeNumber(result.sources_failed)} failed` : ""}`);
      await finishPipeline(result, "all-sources-scan");
    } catch (error) { addStatusMessage(`Scan failed: ${error}`); }
    finally { setScanning(false); }
  };
  const aiDiscover = async () => {
    if (discovering || scanning) return; setDiscovering(true); addStatusMessage("AI Source Discovery is analyzing available drives...");
    try {
      const result = await invoke<{ discovered: number; added: number; existing: number }>("discover_media_sources");
      addStatusMessage(`Source discovery complete: ${result.discovered} found, ${result.added} added, ${result.existing} already configured`);
      await loadSources(); if (result.added > 0) await scanAll();
    } catch (error) { addStatusMessage(`AI Source Discovery failed: ${error}`); }
    finally { setDiscovering(false); }
  };
  const saveLibraryOption = async (key: string, enabled: boolean) => {
    const value = enabled ? "true" : "false"; setSetting(key, value); setSavingOption(key);
    try {
      await invoke("set_setting", { key, value });
      if (key === "prefer_embedded_titles" && enabled) {
        const result = await invoke<{ checked: number; updated: number }>("apply_embedded_titles");
        addStatusMessage(`Embedded titles applied: ${result.updated}/${result.checked} updated`); refreshLibrary("embedded-titles");
      }
    } catch (error) { addStatusMessage(`Failed to save option ${key}: ${error}`); }
    finally { setSavingOption(null); }
  };

  return <div className="space-y-5">
    <section className="glass-panel p-5">
      <div className="mb-4 flex flex-wrap items-start justify-between gap-3"><div><h3 className="flex items-center gap-2 text-sm font-bold"><Plus size={16} className="text-cv-accent" /> Add, Scan, and Enrich</h3><p className="mt-1 text-xs text-cv-subtext">Add a folder or external drive. Scans index quickly, attach local artwork, then enrich metadata.</p></div><button type="button" onClick={() => window.dispatchEvent(new Event("cinavault:ai-autopilot-run"))} className="cv-btn cv-btn-gold text-xs"><Sparkles size={13} /> Run AI Autopilot</button></div>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-4"><div className="md:col-span-2"><label className="section-label">Local path</label><div className="flex gap-2"><input value={newSourcePath} onChange={(e) => setNewSourcePath(e.target.value)} placeholder="E:\\ or E:\\Movies" className="cv-input flex-1" /><button type="button" onClick={() => void browseForSource()} className="cv-btn cv-btn-secondary shrink-0"><FolderOpen size={14} /> Browse</button></div></div><div><label className="section-label">Display name</label><input value={newSourceName} onChange={(e) => setNewSourceName(e.target.value)} placeholder="External Movies" className="cv-input" /></div><div><label className="section-label">Source type</label><select value={newSourceType} onChange={(e) => setNewSourceType(e.target.value)} className="cv-select w-full"><option value="folder">Folder</option><option value="drive">External Drive</option><option value="adult">Adult Media</option><option value="file">File</option></select></div></div>
      <div className="mt-2 text-[10px] text-cv-subtext">Choose <b>Adult Media</b> for an adult library. Every video in that source is labeled adult and routed only to adult metadata/poster providers.</div>
      <div className="mt-4 flex flex-wrap gap-2"><button onClick={addSource} disabled={addingSource || scanning || !newSourcePath.trim()} className="cv-btn cv-btn-primary disabled:opacity-50"><FolderOpen size={14} />{addingSource ? "Adding and scanning..." : "Add Source"}</button><button onClick={aiDiscover} disabled={discovering || scanning} className="cv-btn cv-btn-gold disabled:opacity-50"><Sparkles size={14} className={discovering ? "animate-spin" : ""} />{discovering ? "Discovering..." : "Discover Drives"}</button><button onClick={scanAll} disabled={scanning} className="cv-btn cv-btn-secondary disabled:opacity-50"><Scan size={14} className={scanning ? "animate-spin" : ""} />{scanning ? "Scanning and enriching..." : "Scan Everything"}</button></div>
    </section>
    <section className="glass-panel p-5"><h3 className="mb-4 flex items-center gap-2 text-sm font-bold"><Sparkles size={16} className="text-cv-accent" /> AI Library Policy</h3><div className="grid grid-cols-1 gap-3 md:grid-cols-2">{[["library_auto_scan","Pull metadata and posters after scans","Automatically identify new media and refresh visible cards.",true],["prefer_embedded_titles","Prefer embedded titles","Apply container title tags as a separate post-scan pass.",false],["library_partial_scan_on_changes","Rescan changed paths","Use targeted refreshes when source contents change.",true],["library_empty_trash_after_scan","Remove missing records","Clean database entries for files no longer available.",false]].map(([key,label,description,defaultOn]) => <label key={String(key)} className="glass-panel-2 flex items-start justify-between gap-3 rounded-lg p-3"><span><span className="block text-xs font-semibold">{String(label)}</span><span className="mt-1 block text-[10px] text-cv-subtext">{String(description)}</span></span><input type="checkbox" checked={isEnabled(String(key),Boolean(defaultOn))} disabled={savingOption === key} onChange={(e) => void saveLibraryOption(String(key),e.target.checked)} /></label>)}</div></section>
    <section className="glass-panel p-5"><h3 className="mb-3 flex items-center gap-2 text-sm font-bold"><Link size={16} className="text-cv-accent" /> Web or Playlist Link</h3><div className="flex gap-3"><input value={webLink} onChange={(e) => setWebLink(e.target.value)} placeholder="Paste a media or playlist URL" className="cv-input flex-1" /><button type="button" onClick={() => addStatusMessage(`Download link staged: ${webLink}`)} disabled={!webLink.trim()} className="cv-btn cv-btn-primary shrink-0 disabled:opacity-50"><ExternalLink size={14} /> Send to Downloads</button></div></section>
    <section className="glass-panel overflow-hidden rounded-xl"><div className="flex items-center justify-between border-b border-white/5 px-5 py-3"><h3 className="text-sm font-bold">Configured Sources ({sources.length})</h3><button onClick={() => void loadSources()} className="cv-btn cv-btn-secondary py-1 text-xs"><RefreshCw size={12} /> Refresh</button></div>{sources.length === 0 ? <div className="p-8 text-center"><FolderOpen size={40} className="mx-auto mb-3 text-cv-subtext/20" /><p className="text-sm text-cv-subtext">No real sources are configured. Add a folder or drive above.</p></div> : <div className="divide-y divide-white/5">{sources.map((source,index) => <motion.div key={source.id || `${source.path}-${index}`} initial={{opacity:0,x:-12}} animate={{opacity:1,x:0}} transition={{delay:Math.min(index * .035,.25)}} className="flex items-center gap-4 px-5 py-3 transition-colors hover:bg-white/[0.03]"><div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-cv-accent/10">{sourceIcon(source.source_type)}</div><div className="min-w-0 flex-1"><div className="truncate text-sm font-semibold">{source.name}</div><div className="truncate text-xs text-cv-subtext">{source.path}</div></div><div className="shrink-0 text-right text-xs text-cv-subtext"><div>{source.item_count} items</div><div>{source.last_scanned ? new Date(source.last_scanned).toLocaleString() : "Never scanned"}</div></div><span className={`status-dot ${source.enabled ? "online" : "offline"}`} /><button type="button" onClick={() => void exploreSource(source)} className="cv-btn cv-btn-secondary px-2 py-1 text-xs" title={`Explore ${source.name}`}><FolderOpen size={12} /> Explore Source</button><button type="button" onClick={() => source.id && void removeSource(source.id)} className="cv-btn cv-btn-danger px-2 py-1 text-xs" title={`Remove ${source.name}`}><Trash2 size={12} /></button></motion.div>)}</div>}</section>
    {scanning && <motion.section initial={{opacity:0,y:12}} animate={{opacity:1,y:0}} className="glass-panel p-4"><div className="mb-2 flex items-center justify-between"><span className="text-sm font-semibold">Scanning external sources and posting local artwork...</span><span className="text-xs text-cv-subtext">{scanProgress.current} / {scanProgress.total}</span></div><div className="h-2 w-full overflow-hidden rounded-full bg-white/10"><motion.div className="h-full rounded-full" style={{background:"linear-gradient(90deg, var(--cv-accent), var(--cv-neon-1))"}} animate={{width:scanProgress.total ? `${(scanProgress.current / scanProgress.total) * 100}%` : "15%"}} transition={{duration:.25}} /></div></motion.section>}
  </div>;
}
