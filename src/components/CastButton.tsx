import { useEffect, useState } from "react";
import { Cast, X } from "lucide-react";
import CastingTab from "./tabs/CastingTab";

const OPEN_CASTING_EVENT = "cinavault:open-casting";

export default function CastButton() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const openCasting = () => setOpen(true);
    window.addEventListener(OPEN_CASTING_EVENT, openCasting);
    return () => window.removeEventListener(OPEN_CASTING_EVENT, openCasting);
  }, []);

  return (
    <>
      <button
        type="button"
        className="fixed right-7 top-7 z-40 flex h-12 items-center gap-2 rounded-2xl border border-cyan-100/25 bg-[#071421]/90 px-4 font-bold text-cyan-100 shadow-[0_18px_55px_rgba(0,0,0,0.45)] backdrop-blur-xl hover:bg-[#0b1e30]"
        data-testid="cinavault-cast-button"
        aria-label="Open CinaVault Casting"
        title="Open Casting Center"
        onClick={() => setOpen(true)}
      >
        <Cast size={18} /> Cast
      </button>

      {open && (
        <div
          className="fixed inset-0 z-[100] overflow-y-auto bg-[#020711]/92 p-4 backdrop-blur-2xl md:p-8"
          data-testid="cinavault-cast-tab"
          role="dialog"
          aria-modal="true"
          aria-label="CinaVault Casting Center"
        >
          <div className="mx-auto min-h-full max-w-[1500px] rounded-[32px] border border-white/10 bg-[radial-gradient(circle_at_15%_0%,rgba(0,234,255,0.18),transparent_32%),linear-gradient(180deg,#071421,#030813)] p-5 shadow-[0_30px_120px_rgba(0,0,0,0.75)] md:p-7">
            <header className="mb-6 flex flex-wrap items-center justify-between gap-4 border-b border-white/10 pb-5">
              <div>
                <div className="text-[11px] font-bold uppercase tracking-[0.34em] text-cyan-200">
                  Future Horizon
                </div>
                <h1 className="mt-2 text-3xl font-black tracking-tight md:text-5xl">
                  CinaVault Casting
                </h1>
                <p className="mt-2 text-sm text-cv-subtext md:text-base">
                  Discover, connect, and control Chromecast / Google Cast, AirPlay, Smart View, and DLNA receivers.
                </p>
              </div>
              <button
                type="button"
                onClick={() => setOpen(false)}
                className="grid h-12 w-12 place-items-center rounded-2xl border border-white/10 bg-white/[0.06] hover:bg-white/[0.10]"
                aria-label="Close Casting Center"
              >
                <X size={20} />
              </button>
            </header>
            <CastingTab />
          </div>
        </div>
      )}
    </>
  );
}
