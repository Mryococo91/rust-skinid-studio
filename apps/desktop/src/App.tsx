import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import {
  appPaths,
  bytesToBase64,
  HistoryEntry,
  listHistory,
  listProjects,
  prepareAsset,
  processImageBytes,
  processImagePath,
  ProcessedImage,
  ProjectRecord,
  publishAsset,
  PublishResult,
  steamDoctor,
  SteamStatus,
  wipeLocalData,
} from "./lib/api";

type Tab = "studio" | "history" | "settings";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

export default function App() {
  const [tab, setTab] = useState<Tab>("studio");
  const [dragOver, setDragOver] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sourcePreview, setSourcePreview] = useState<string | null>(null);
  const [sourceBase64, setSourceBase64] = useState<string | null>(null);
  const [processed, setProcessed] = useState<ProcessedImage | null>(null);
  const [rotate, setRotate] = useState(0);
  const [targetSize, setTargetSize] = useState(512);
  const [title, setTitle] = useState("Custom Icon");
  const [description, setDescription] = useState(
    "Custom icon-only Rust Workshop item — unique SkinID for plugins / custom items.",
  );
  const [itemType, setItemType] = useState("Grenade");
  const [projectId, setProjectId] = useState<string | null>(null);
  const [assetDir, setAssetDir] = useState<string | null>(null);
  /** Live Steam by default — requires Steam client logged in + Rust owned. */
  const [dryRun, setDryRun] = useState(false);
  const [steam, setSteam] = useState<SteamStatus | null>(null);
  const [publishResult, setPublishResult] = useState<PublishResult | null>(null);
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [paths, setPaths] = useState<{ data_dir: string; projects_dir: string } | null>(
    null,
  );
  const [showAdvancedUpdate, setShowAdvancedUpdate] = useState(false);
  const [updateSkinId, setUpdateSkinId] = useState("");

  const validation = processed?.validation;
  const compatible = validation?.compatible ?? false;

  const refreshLists = useCallback(async () => {
    try {
      const [p, h, ap] = await Promise.all([
        listProjects(),
        listHistory(),
        appPaths(),
      ]);
      setProjects(p);
      setHistory(h);
      setPaths(ap);
    } catch {
      // ignore outside Tauri
    }
  }, []);

  useEffect(() => {
    void refreshLists();
    void steamDoctor(false)
      .then(setSteam)
      .catch(() => {
        /* Steam may be offline at launch */
      });
  }, [refreshLists]);

  const runProcessPath = async (path: string) => {
    setBusy(true);
    setError(null);
    setPublishResult(null);
    try {
      const result = await processImagePath(path, {
        target_size: targetSize,
        rotate,
      });
      setProcessed(result);
      setSourcePreview(result.preview_data_url);
      setSourceBase64(result.png_base64);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const runProcessFile = async (file: File) => {
    setBusy(true);
    setError(null);
    setPublishResult(null);
    try {
      const buf = await file.arrayBuffer();
      const b64 = bytesToBase64(buf);
      setSourcePreview(URL.createObjectURL(file));
      setSourceBase64(b64);
      const result = await processImageBytes(b64, {
        target_size: targetSize,
        rotate,
      });
      setProcessed(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const onPickFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Images",
            extensions: ["png", "jpg", "jpeg", "webp", "bmp"],
          },
        ],
      });
      if (typeof selected === "string") {
        await runProcessPath(selected);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const reprocess = async () => {
    if (!sourceBase64 && !processed) return;
    setBusy(true);
    setError(null);
    try {
      const result = await processImageBytes(sourceBase64 ?? processed!.png_base64, {
        target_size: targetSize,
        rotate,
      });
      setProcessed(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const prepareFresh = async () => {
    if (!processed) throw new Error("No processed image");
    // Always a new local project so we never reuse an old SkinID by accident.
    const res = await prepareAsset({
      png_base64: processed.png_base64,
      title,
      description,
      item_type: itemType || "Grenade",
    });
    setProjectId(res.project_id);
    setAssetDir(res.asset_dir);
    return res;
  };

  const onPrepare = async () => {
    if (!processed) return;
    setBusy(true);
    setError(null);
    try {
      await prepareFresh();
      await refreshLists();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const onDoctor = async () => {
    setBusy(true);
    setError(null);
    try {
      setSteam(await steamDoctor(dryRun));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const onCreateSkinId = async () => {
    if (!processed) return;
    setBusy(true);
    setError(null);
    setPublishResult(null);
    try {
      // Fresh pack + CreateItem → unique SkinID every time.
      const prepared = await prepareFresh();
      const result = await publishAsset({
        asset_dir: prepared.asset_dir,
        project_id: prepared.project_id,
        title,
        description,
        dry_run: dryRun,
        create_new: true,
      });
      setPublishResult(result);
      await refreshLists();
      if (!dryRun) {
        setSteam(await steamDoctor(false).catch(() => null));
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const onUpdateSkinId = async () => {
    if (!processed || !updateSkinId.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const prepared = await prepareFresh();
      const result = await publishAsset({
        asset_dir: prepared.asset_dir,
        project_id: prepared.project_id,
        title,
        description,
        dry_run: dryRun,
        create_new: false,
        published_file_id: Number(updateSkinId.trim()),
      });
      setPublishResult(result);
      await refreshLists();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const copySkinId = async () => {
    if (!publishResult) return;
    await navigator.clipboard.writeText(String(publishResult.published_file_id));
  };

  const loadProject = (p: ProjectRecord) => {
    setProjectId(p.id);
    setTitle(p.title);
    setDescription(p.description);
    setItemType(p.item_type);
    setAssetDir(p.asset_dir);
    setTab("studio");
  };

  const statusBadge = useMemo(() => {
    if (!validation) return null;
    if (validation.compatible) {
      return <span className="badge ok">Compatible</span>;
    }
    return <span className="badge bad">Not compatible</span>;
  }, [validation]);

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <h1>Rust SkinID Studio</h1>
          <p>
            Create a custom icon and publish a <strong>new unique Workshop
            SkinID</strong> with your Steam account — for custom items / plugins,
            with no required link to a vanilla item.
          </p>
        </div>
        <nav className="nav">
          <button
            className={tab === "studio" ? "active" : ""}
            onClick={() => setTab("studio")}
            type="button"
          >
            Studio
          </button>
          <button
            className={tab === "history" ? "active" : ""}
            onClick={() => setTab("history")}
            type="button"
          >
            History
          </button>
          <button
            className={tab === "settings" ? "active" : ""}
            onClick={() => setTab("settings")}
            type="button"
          >
            Settings
          </button>
        </nav>
      </header>

      {tab === "studio" && (
        <>
          <div className="notice">
            Main action: <strong>Create a unique SkinID</strong> (new Workshop
            item). No automatic assignment to an in-game item — you then use this
            SkinID in your plugins however you like. Steam must be running and
            logged in (account that owns Rust).
          </div>

          <div className="grid-2">
            <section className="panel">
              <div
                className={`dropzone ${dragOver ? "dragover" : ""}`}
                onDragOver={(e) => {
                  e.preventDefault();
                  setDragOver(true);
                }}
                onDragLeave={() => setDragOver(false)}
                onDrop={(e) => {
                  e.preventDefault();
                  setDragOver(false);
                  const file = e.dataTransfer.files?.[0];
                  if (file) void runProcessFile(file);
                }}
                onClick={() => void onPickFile()}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") void onPickFile();
                }}
              >
                <div>
                  <strong>Drop an image here</strong>
                  <span>PNG, JPEG, WebP, BMP — or click to browse</span>
                </div>
              </div>

              <div className="controls">
                <label>
                  Size{" "}
                  <select
                    value={targetSize}
                    onChange={(e) => setTargetSize(Number(e.target.value))}
                  >
                    <option value={256}>256</option>
                    <option value={512}>512</option>
                    <option value={1024}>1024</option>
                    <option value={2048}>2048</option>
                  </select>
                </label>
                <label>
                  Rotation{" "}
                  <select
                    value={rotate}
                    onChange={(e) => setRotate(Number(e.target.value))}
                  >
                    <option value={0}>0°</option>
                    <option value={90}>90°</option>
                    <option value={180}>180°</option>
                    <option value={270}>270°</option>
                  </select>
                </label>
                <button
                  className="btn ghost"
                  type="button"
                  disabled={!processed || busy}
                  onClick={() => void reprocess()}
                >
                  Reapply
                </button>
              </div>

              <div className="field" style={{ marginTop: 16 }}>
                <label htmlFor="title">Workshop title</label>
                <input
                  id="title"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                />
              </div>
              <div className="field">
                <label htmlFor="description">Description</label>
                <textarea
                  id="description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                />
              </div>
              <div className="field">
                <label htmlFor="itemType">
                  Workshop category (technical — not an in-game shortname)
                </label>
                <input
                  id="itemType"
                  value={itemType}
                  onChange={(e) => setItemType(e.target.value)}
                  placeholder="Grenade"
                />
              </div>

              <label style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <input
                  type="checkbox"
                  checked={dryRun}
                  onChange={(e) => setDryRun(e.target.checked)}
                />
                Dry-run only (does not contact Steam — for testing)
              </label>

              <div className="actions">
                <button
                  className="btn"
                  type="button"
                  disabled={busy}
                  onClick={() => void onDoctor()}
                >
                  Connect / check Steam
                </button>
                <button
                  className="btn"
                  type="button"
                  disabled={!compatible || busy}
                  onClick={() => void onPrepare()}
                >
                  Prepare locally
                </button>
                <button
                  className="btn primary"
                  type="button"
                  disabled={!compatible || busy}
                  onClick={() => void onCreateSkinId()}
                >
                  Create a unique SkinID
                </button>
              </div>

              <details
                style={{ marginTop: 16 }}
                open={showAdvancedUpdate}
                onToggle={(e) =>
                  setShowAdvancedUpdate((e.target as HTMLDetailsElement).open)
                }
              >
                <summary style={{ cursor: "pointer", color: "var(--muted)" }}>
                  Advanced — update an existing SkinID
                </summary>
                <div className="field" style={{ marginTop: 12 }}>
                  <label htmlFor="updateSkinId">SkinID to update</label>
                  <input
                    id="updateSkinId"
                    value={updateSkinId}
                    onChange={(e) => setUpdateSkinId(e.target.value)}
                    placeholder="e.g. 3248306153"
                  />
                </div>
                <button
                  className="btn"
                  type="button"
                  disabled={!compatible || busy || !updateSkinId.trim()}
                  onClick={() => void onUpdateSkinId()}
                >
                  Update this SkinID
                </button>
              </details>
            </section>

            <section className="panel preview-wrap">
              <div className="before-after">
                <div>
                  <div className="label">Icon preview</div>
                  <div className="preview-frame">
                    {(processed?.preview_data_url || sourcePreview) && (
                      <img
                        src={processed?.preview_data_url || sourcePreview || ""}
                        alt="Custom icon preview"
                      />
                    )}
                  </div>
                </div>
                <div>
                  <div className="label">Inventory preview</div>
                  <div className="preview-frame" style={{ background: "#0f1318" }}>
                    {processed && (
                      <div style={{ textAlign: "center" }}>
                        <img
                          src={processed.preview_data_url}
                          alt="Inventory icon"
                          style={{ width: "64%", height: "64%" }}
                        />
                        <div
                          style={{
                            marginTop: 8,
                            color: "var(--muted)",
                            fontSize: "0.75rem",
                          }}
                        >
                          custom item
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              </div>

              <div className="meta">
                <div className="row">
                  <span>Dimensions</span>
                  <b>
                    {validation
                      ? `${validation.width} × ${validation.height}`
                      : "—"}
                  </b>
                </div>
                <div className="row">
                  <span>Format</span>
                  <b>{validation?.format ?? "—"}</b>
                </div>
                <div className="row">
                  <span>Size</span>
                  <b>{validation ? formatBytes(validation.bytes) : "—"}</b>
                </div>
                <div className="row">
                  <span>Status</span>
                  <b>{statusBadge ?? "—"}</b>
                </div>
              </div>

              {validation && !validation.compatible && (
                <ul style={{ color: "var(--bad)", margin: 0, paddingLeft: 18 }}>
                  {validation.messages.map((m) => (
                    <li key={m}>{m}</li>
                  ))}
                </ul>
              )}

              {assetDir && (
                <div className="notice" style={{ marginBottom: 0 }}>
                  Local pack{projectId ? ` (${projectId.slice(0, 8)}…)` : ""}:{" "}
                  <code>{assetDir}</code>
                </div>
              )}

              {steam && (
                <div className={steam.ready_to_publish ? "success" : "error"}>
                  <div>
                    Steam ({steam.mode}) — {steam.persona_name ?? "not connected"}{" "}
                    {steam.ready_to_publish
                      ? "— ready to publish"
                      : "— not ready"}
                  </div>
                  <ul style={{ margin: "8px 0 0", paddingLeft: 18 }}>
                    {steam.messages.map((m) => (
                      <li key={m}>{m}</li>
                    ))}
                  </ul>
                </div>
              )}

              {publishResult && (
                <div className="success">
                  <div>
                    New SkinID:{" "}
                    <code>{publishResult.published_file_id}</code>{" "}
                    {publishResult.dry_run && (
                      <span className="badge warn">dry-run</span>
                    )}
                  </div>
                  <div style={{ marginTop: 8 }}>
                    <a
                      href={publishResult.url}
                      onClick={(e) => {
                        e.preventDefault();
                        void openUrl(publishResult.url);
                      }}
                    >
                      {publishResult.url}
                    </a>
                  </div>
                  <div className="actions">
                    <button
                      className="btn"
                      type="button"
                      onClick={() => void copySkinId()}
                    >
                      Copy SkinID
                    </button>
                  </div>
                  {publishResult.needs_workshop_agreement && (
                    <p>
                      Accept the Steam Workshop Legal Agreement in your browser,
                      then create / republish again.
                    </p>
                  )}
                </div>
              )}
            </section>
          </div>
        </>
      )}

      {tab === "history" && (
        <section className="panel">
          <h2 style={{ marginTop: 0 }}>Projects</h2>
          <div className="list">
            {projects.length === 0 && <p>No local projects.</p>}
            {projects.map((p) => (
              <div className="list-item" key={p.id}>
                <div>
                  <h3>{p.title}</h3>
                  <p>
                    {p.published_file_id
                      ? `SkinID ${p.published_file_id}`
                      : "Not published"}{" "}
                    · {p.item_type}
                  </p>
                </div>
                <button className="btn" type="button" onClick={() => loadProject(p)}>
                  Open
                </button>
              </div>
            ))}
          </div>

          <h2>Publish history</h2>
          <div className="list">
            {history.length === 0 && <p>No history yet.</p>}
            {history.map((h) => (
              <div className="list-item" key={h.id}>
                <div>
                  <h3>{h.published_file_id}</h3>
                  <p>
                    {h.dry_run ? "dry-run" : "live"} · {h.message}
                  </p>
                </div>
                <button
                  className="btn ghost"
                  type="button"
                  onClick={() => void openUrl(h.url)}
                >
                  Open
                </button>
              </div>
            ))}
          </div>
        </section>
      )}

      {tab === "settings" && (
        <section className="panel">
          <h2 style={{ marginTop: 0 }}>Settings</h2>
          <p className="notice">
            Steam sign-in uses your local Steam client (already logged in — no
            password in the app). Each click on &quot;Create a unique SkinID&quot;
            calls CreateItem and generates a new Workshop ID.
          </p>
          <div className="meta">
            <div className="row">
              <span>Version</span>
              <b>0.1.0</b>
            </div>
            <div className="row">
              <span>Data dir</span>
              <b>{paths?.data_dir ?? "—"}</b>
            </div>
            <div className="row">
              <span>Projects dir</span>
              <b>{paths?.projects_dir ?? "—"}</b>
            </div>
          </div>
          <div className="actions">
            <button className="btn" type="button" onClick={() => void refreshLists()}>
              Refresh
            </button>
            <button className="btn" type="button" onClick={() => void onDoctor()}>
              Check Steam
            </button>
            <button
              className="btn ghost"
              type="button"
              disabled={busy}
              onClick={() => {
                if (
                  !window.confirm(
                    "Delete all local projects and publish history on this PC?",
                  )
                ) {
                  return;
                }
                setBusy(true);
                setError(null);
                void wipeLocalData()
                  .then(async () => {
                    setProjectId(null);
                    setAssetDir(null);
                    setPublishResult(null);
                    await refreshLists();
                  })
                  .catch((e) => setError(String(e)))
                  .finally(() => setBusy(false));
              }}
            >
              Wipe local data
            </button>
          </div>
        </section>
      )}

      {error && <div className="error">{error}</div>}
      {busy && (
        <p style={{ color: "var(--muted)", marginTop: 12 }}>Working…</p>
      )}
    </div>
  );
}
