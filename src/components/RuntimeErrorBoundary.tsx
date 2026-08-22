import { Component, type ErrorInfo, type ReactNode } from "react";
import { AlertTriangle, Home, RefreshCw } from "lucide-react";
import { useAppStore } from "../store/appStore";

interface RuntimeErrorBoundaryProps {
  children: ReactNode;
  scope?: string;
}

interface RuntimeErrorBoundaryState {
  error: Error | null;
  diagnosticId: string | null;
}

interface UiCrashRecord {
  diagnosticId: string;
  scope: string;
  message: string;
  stack: string | null;
  componentStack: string | null;
  occurredAt: string;
}

const CRASH_STORAGE_KEY = "cinavault_last_ui_error";

function createDiagnosticId(): string {
  const randomPart = Math.random().toString(36).slice(2, 8).toUpperCase();
  return `CV-${Date.now().toString(36).toUpperCase()}-${randomPart}`;
}

function persistCrash(record: UiCrashRecord): void {
  try {
    localStorage.setItem(CRASH_STORAGE_KEY, JSON.stringify(record));
  } catch (error) {
    console.warn("Unable to persist CinaVault UI diagnostic:", error);
  }
}

export default class RuntimeErrorBoundary extends Component<
  RuntimeErrorBoundaryProps,
  RuntimeErrorBoundaryState
> {
  state: RuntimeErrorBoundaryState = {
    error: null,
    diagnosticId: null,
  };

  static getDerivedStateFromError(error: Error): RuntimeErrorBoundaryState {
    return {
      error,
      diagnosticId: createDiagnosticId(),
    };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    const diagnosticId = this.state.diagnosticId || createDiagnosticId();
    const record: UiCrashRecord = {
      diagnosticId,
      scope: this.props.scope || "application",
      message: error.message || "Unknown interface error",
      stack: error.stack || null,
      componentStack: info.componentStack || null,
      occurredAt: new Date().toISOString(),
    };

    console.error("CinaVault interface recovery boundary caught an error:", record);
    persistCrash(record);
    useAppStore
      .getState()
      .addStatusMessage(`Interface recovered from error ${diagnosticId}`);
  }

  private recoverToLibrary = (): void => {
    const store = useAppStore.getState();
    store.setActiveTab("home");
    store.setSearchQuery("");
    store.addStatusMessage("Returned to Library after interface recovery");
    this.setState({ error: null, diagnosticId: null }, () => {
      window.requestAnimationFrame(() => {
        window.dispatchEvent(new Event("resize"));
      });
    });
  };

  private reloadApplication = (): void => {
    window.location.reload();
  };

  render(): ReactNode {
    const { error, diagnosticId } = this.state;
    if (!error) return this.props.children;

    return (
      <main className="cv-runtime-fallback" role="alert" aria-live="assertive">
        <section className="cv-runtime-fallback-card">
          <div className="cv-runtime-fallback-icon" aria-hidden="true">
            <AlertTriangle size={30} />
          </div>
          <div className="cv-runtime-fallback-kicker">v2 Build 1.02 recovery</div>
          <h1>CinaVault stopped a screen from crashing the app</h1>
          <p>
            A menu or workspace failed to render. The media server and library data
            remain intact. Return to Library to continue, or reload the interface.
          </p>

          <div className="cv-runtime-fallback-diagnostic">
            <span>Diagnostic</span>
            <strong>{diagnosticId || "Unavailable"}</strong>
            <code>{error.message || "Unknown interface error"}</code>
          </div>

          <div className="cv-runtime-fallback-actions">
            <button type="button" onClick={this.recoverToLibrary}>
              <Home size={17} /> Return to Library
            </button>
            <button type="button" onClick={this.reloadApplication}>
              <RefreshCw size={17} /> Reload interface
            </button>
          </div>
        </section>
      </main>
    );
  }
}
