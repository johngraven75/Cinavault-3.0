// Build 140 Futuristic Sidebar Navigation compatibility retained by the current manifest-driven shell.
import type { JSX } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { BUILD_INFO } from "../buildInfo";
import { useAppStore, type TabId } from "../store/appStore";
import {
  BrainCircuit,
  Cast,
  ChevronLeft,
  ChevronRight,
  Cloud,
  Download,
  Film,
  FolderOpen,
  Home,
  Puzzle,
  Router,
  Server,
  Settings,
  Shield,
  Sliders,
  Sparkles,
  Tv,
  type LucideIcon,
} from "lucide-react";

interface NavItem {
  id: TabId;
  label: string;
  icon: LucideIcon;
  zone: "Experience" | "Infrastructure" | "Intelligence";
  detail: string;
  accent: string;
}

const NAV_ITEMS: NavItem[] = [
  {
    id: "home",
    label: "Library",
    icon: Home,
    zone: "Experience",
    detail: "Browse & play",
    accent: "from-cyan-300/34 to-blue-500/10",
  },
  {
    id: "livetv",
    label: "Live TV",
    icon: Tv,
    zone: "Experience",
    detail: "Channels & guide",
    accent: "from-fuchsia-400/32 to-purple-500/10",
  },
  {
    id: "downloads",
    label: "Downloads",
    icon: Download,
    zone: "Experience",
    detail: "Acquisition queue",
    accent: "from-amber-300/30 to-orange-500/10",
  },
  {
    id: "sources",
    label: "Sources",
    icon: FolderOpen,
    zone: "Infrastructure",
    detail: "Ingest libraries",
    accent: "from-emerald-300/30 to-cyan-500/10",
  },
  {
    id: "server",
    label: "Server",
    icon: Server,
    zone: "Infrastructure",
    detail: "Core services",
    accent: "from-blue-300/32 to-indigo-500/10",
  },
  {
    id: "remote",
    label: "Remote",
    icon: Router,
    zone: "Infrastructure",
    detail: "Reachability & relay",
    accent: "from-cyan-300/30 to-violet-500/10",
  },
  {
    id: "security",
    label: "Security",
    icon: Shield,
    zone: "Infrastructure",
    detail: "Identity & privacy",
    accent: "from-emerald-300/28 to-teal-500/10",
  },
  {
    id: "ai",
    label: "AI Autopilot",
    icon: BrainCircuit,
    zone: "Intelligence",
    detail: "Automate media",
    accent: "from-fuchsia-400/34 to-cyan-400/10",
  },
  {
    id: "hf-models",
    label: "HF Models",
    icon: Sparkles,
    zone: "Intelligence",
    detail: "Free model catalog",
    accent: "from-amber-300/30 to-fuchsia-500/10",
  },
  {
    id: "plugins",
    label: "Extensions",
    icon: Puzzle,
    zone: "Intelligence",
    detail: "Providers & tools",
    accent: "from-violet-400/30 to-pink-500/10",
  },
  {
    id: "cloud",
    label: "Cloud & NAS",
    icon: Cloud,
    zone: "Intelligence",
    detail: "Storage mesh",
    accent: "from-sky-300/30 to-blue-500/10",
  },
  {
    id: "advanced",
    label: "Advanced",
    icon: Sliders,
    zone: "Intelligence",
    detail: "Diagnostics & tuning",
    accent: "from-orange-300/28 to-fuchsia-500/10",
  },
  {
    id: "settings",
    label: "Settings",
    icon: Settings,
    zone: "Intelligence",
    detail: "Experience control",
    accent: "from-slate-200/24 to-cyan-500/10",
  },
];

const ZONES: NavItem["zone"][] = [
  "Experience",
  "Infrastructure",
  "Intelligence",
];

function NavItemButton({
  item,
  collapsed,
}: {
  item: NavItem;
  collapsed: boolean;
}): JSX.Element {
  const { activeTab, setActiveTab } = useAppStore();
  const active = activeTab === item.id;
  const Icon = item.icon;

  return (
    <motion.button
      type="button"
      onClick={() => setActiveTab(item.id)}
      title={collapsed ? item.label : undefined}
      whileHover={{ x: collapsed ? 0 : 5, scale: collapsed ? 1.035 : 1 }}
      whileTap={{ scale: 0.975 }}
      className={`group relative flex w-full items-center gap-3 overflow-hidden rounded-[18px] border px-2.5 py-2.5 text-left transition-colors ${
        active
          ? "border-white/22 text-white"
          : "border-transparent text-slate-400 hover:border-white/10 hover:text-white"
      }`}
    >
      {active && (
        <motion.div
          layoutId="cv-orbital-nav-active"
          className={`sidebar-active-panel absolute inset-0 rounded-[18px] bg-gradient-to-r ${item.accent} shadow-[inset_0_1px_0_rgba(255,255,255,0.12),0_0_30px_rgba(105,247,255,0.12)]`}
          transition={{ type: "spring", stiffness: 420, damping: 34 }}
        />
      )}
      {active && (
        <motion.div
          layoutId="cv-orbital-nav-rail"
          className="sidebar-active-rail absolute left-0 top-1/2 h-8 w-[3px] -translate-y-1/2 rounded-r-full bg-cyan-200 shadow-[0_0_14px_rgba(105,247,255,0.95)]"
        />
      )}

      <span className="relative z-10 grid h-10 w-10 shrink-0 place-items-center rounded-[14px] border border-white/10 bg-black/30 shadow-[inset_0_1px_0_rgba(255,255,255,0.08)] transition group-hover:border-cyan-200/30 group-hover:bg-cyan-200/10">
        <Icon size={18} />
      </span>

      <AnimatePresence initial={false}>
        {!collapsed && (
          <motion.span
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -8 }}
            className="relative z-10 min-w-0 flex-1"
          >
            <span className="block truncate text-[13px] font-extrabold tracking-tight">
              {item.label}
            </span>
            <span className="block truncate text-[9px] font-semibold uppercase tracking-[0.19em] text-slate-400/75">
              {item.detail}
            </span>
          </motion.span>
        )}
      </AnimatePresence>
    </motion.button>
  );
}

export default function Sidebar(): JSX.Element {
  const {
    sidebarCollapsed,
    toggleSidebar,
    statusMessages,
    settings,
  } = useAppStore();
  const autopilotEnabled = settings.ai_media_autopilot_enabled !== "false";

  return (
    <motion.aside
      animate={{ width: sidebarCollapsed ? 78 : 258 }}
      transition={{ duration: 0.34, ease: [0.16, 1, 0.3, 1] }}
      className="relative z-20 h-full shrink-0 overflow-hidden rounded-[28px] border border-white/12 bg-[linear-gradient(180deg,rgba(12,15,34,0.94),rgba(4,7,18,0.82))] shadow-[0_28px_90px_rgba(0,0,0,0.58),inset_0_1px_0_rgba(255,255,255,0.10)] backdrop-blur-3xl"
    >
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_24%_0%,rgba(78,124,255,0.24),transparent_26%),radial-gradient(circle_at_100%_35%,rgba(255,79,207,0.12),transparent_28%)]" />
      <div className="pointer-events-none absolute inset-y-10 right-0 w-px bg-gradient-to-b from-transparent via-cyan-200/25 to-transparent" />

      <div className="relative z-10 flex h-full flex-col">
        <div className="flex h-[92px] shrink-0 items-center gap-3 border-b border-white/[0.08] px-3">
          <motion.div
            className="relative grid h-13 w-13 shrink-0 place-items-center rounded-[18px] border border-cyan-200/20 bg-[radial-gradient(circle_at_30%_20%,rgba(255,255,255,0.22),rgba(105,247,255,0.08)_45%,rgba(0,0,0,0.35))] shadow-[0_0_28px_rgba(105,247,255,0.18)]"
            animate={{ rotate: [0, 1.2, 0, -1.2, 0] }}
            transition={{ duration: 9, repeat: Infinity, ease: "easeInOut" }}
          >
            <img
              src="/branding/cinavault-logo.png"
              alt="CinaVault Premium"
              className="h-10 w-10 rounded-[14px] object-cover"
            />
            <span className="absolute -right-1 -top-1 h-3 w-3 rounded-full border-2 border-[#07101f] bg-emerald-300 shadow-[0_0_12px_rgba(98,255,194,0.9)]" />
          </motion.div>

          {!sidebarCollapsed && (
            <motion.div
              initial={{ opacity: 0, x: -8 }}
              animate={{ opacity: 1, x: 0 }}
              className="min-w-0"
            >
              <div className="flex items-center gap-1.5 text-[9px] font-black uppercase tracking-[0.28em] text-cyan-200">
                <Film size={11} /> Spatial Media OS
              </div>
              <div className="truncate text-xl font-black tracking-[-0.03em] text-white">
                CinaVault
              </div>
              <div className="text-[9px] font-bold uppercase tracking-[0.18em] text-slate-400">
                {BUILD_INFO.displayName} · Online
              </div>
            </motion.div>
          )}
        </div>

        <nav className="min-h-0 flex-1 overflow-y-auto px-2 py-3">
          <div className="mb-3">
            {!sidebarCollapsed && (
              <div className="mb-1 px-2 text-[9px] font-black uppercase tracking-[0.24em] text-slate-500">
                Connected experience
              </div>
            )}
            <motion.button
              type="button"
              onClick={() =>
                window.dispatchEvent(new Event("cinavault:open-casting"))
              }
              whileHover={{ x: sidebarCollapsed ? 0 : 5 }}
              whileTap={{ scale: 0.98 }}
              className="group relative flex w-full items-center gap-3 overflow-hidden rounded-[18px] border border-fuchsia-300/18 bg-[linear-gradient(90deg,rgba(255,79,207,0.15),rgba(105,247,255,0.07))] px-2.5 py-2.5 text-left text-white"
              title={sidebarCollapsed ? "Casting" : undefined}
            >
              <span className="absolute inset-y-0 right-0 w-20 bg-[radial-gradient(circle,rgba(255,255,255,0.16),transparent_65%)] opacity-70" />
              <span className="relative z-10 grid h-10 w-10 shrink-0 place-items-center rounded-[14px] border border-fuchsia-200/20 bg-black/28 text-fuchsia-100 shadow-[0_0_18px_rgba(255,79,207,0.12)]">
                <Cast size={18} />
              </span>
              {!sidebarCollapsed && (
                <span className="relative z-10 min-w-0 flex-1">
                  <span className="block truncate text-[13px] font-extrabold">
                    Casting Center
                  </span>
                  <span className="block text-[9px] font-semibold uppercase tracking-[0.19em] text-fuchsia-100/60">
                    Discover & beam
                  </span>
                </span>
              )}
            </motion.button>
          </div>

          {ZONES.map((zone) => (
            <div key={zone} className="mb-3 last:mb-0">
              {!sidebarCollapsed && (
                <div className="mb-1 px-2 text-[9px] font-black uppercase tracking-[0.24em] text-slate-500">
                  {zone}
                </div>
              )}
              <div className="space-y-0.5">
                {NAV_ITEMS.filter((item) => item.zone === zone).map((item) => (
                  <NavItemButton
                    key={item.id}
                    item={item}
                    collapsed={sidebarCollapsed}
                  />
                ))}
              </div>
            </div>
          ))}
        </nav>

        <div className="border-t border-white/[0.08] p-2">
          <div className="mb-2 rounded-[18px] border border-white/[0.08] bg-white/[0.035] p-2.5">
            <div className="flex items-center gap-2">
              <span className="cv-status-orb shrink-0" />
              {!sidebarCollapsed && (
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1 text-[10px] font-extrabold text-white">
                    <Sparkles size={11} className="text-fuchsia-300" /> AI Autopilot
                  </div>
                  <div className="truncate text-[9px] uppercase tracking-[0.16em] text-slate-500">
                    {autopilotEnabled
                      ? "Scanning · enriching · repairing"
                      : "Manual mode"}
                  </div>
                </div>
              )}
              {!sidebarCollapsed && statusMessages.length > 0 && (
                <span className="rounded-full bg-cyan-300/10 px-2 py-0.5 text-[9px] font-black text-cyan-200">
                  {statusMessages.length}
                </span>
              )}
            </div>
          </div>

          <button
            type="button"
            onClick={toggleSidebar}
            className="flex h-11 w-full items-center justify-center rounded-[16px] border border-white/[0.08] bg-white/[0.035] text-slate-400 transition hover:border-cyan-200/20 hover:bg-cyan-200/[0.07] hover:text-white"
            title={sidebarCollapsed ? "Expand navigation" : "Collapse navigation"}
          >
            {sidebarCollapsed ? (
              <ChevronRight size={16} />
            ) : (
              <ChevronLeft size={16} />
            )}
          </button>
        </div>
      </div>
    </motion.aside>
  );
}
