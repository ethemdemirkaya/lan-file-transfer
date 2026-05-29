import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  FluentProvider,
  webLightTheme,
  webDarkTheme,
  Theme,
  Title2,
  Subtitle1,
  Body1,
  Body1Strong,
  Caption1,
  Button,
  Input,
  Field,
  ProgressBar,
  MessageBar,
  MessageBarBody,
  MessageBarTitle,
  Spinner,
  Dialog,
  DialogTrigger,
  DialogSurface,
  DialogTitle,
  DialogBody,
  DialogContent,
  DialogActions,
  Badge,
  Divider,
  Tooltip,
  Menu,
  MenuTrigger,
  MenuPopover,
  MenuList,
  MenuItem,
  Drawer,
  DrawerHeader,
  DrawerHeaderTitle,
  DrawerBody,
  Switch,
  Checkbox,
  RadioGroup,
  Radio,
  makeStyles,
  shorthands,
  tokens,
} from "@fluentui/react-components";
import {
  ArrowSync20Regular,
  Folder20Regular,
  Document20Regular,
  Send20Filled,
  Dismiss20Regular,
  CloudArrowUp48Regular,
  Edit20Regular,
  Wifi120Regular,
  LocalLanguage20Regular,
  Settings20Regular,
  Delete20Regular,
  Pause20Regular,
  Play20Regular,
  Subtract20Regular,
  Square20Regular,
} from "@fluentui/react-icons";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { getVersion as tauriGetVersion } from "@tauri-apps/api/app";
import {
  cancelTransfer,
  checkForUpdate,
  clearHistory,
  ensureReceiver,
  getAccentColor,
  getHistory,
  getSession,
  HistoryRecord,
  openExternalUrl,
  UpdateInfo,
  IncomingRequest,
  onIncomingRequest,
  onPeerAdded,
  onPeerRemoved,
  onReceiverReady,
  onTransferCompleted,
  onTransferProgress,
  onTransferStarted,
  pauseTransfer,
  Peer,
  refreshDiscovery,
  regenerateCode,
  respondIncoming,
  resumeTransfer,
  saveSettings,
  sendPaths,
  Session,
  setAutoStart,
  setCloseToTray,
  setNotificationsEnabled,
  setSoundEnabled,
  setTaskbarProgress,
  setTheme,
  ThemePref,
  TransferCompleted,
  TransferProgress,
  TransferStarted,
  trustDevice,
  untrustDevice,
  Unlisten,
  UserSettings,
} from "./lib/tauri";
import { SUPPORTED_LANGS, setLang } from "./i18n";

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const useStyles = makeStyles({
  app: { minHeight: "100vh", backgroundColor: "transparent", color: tokens.colorNeutralForeground1, display: "flex", flexDirection: "column" },
  titleBar: {
    height: "32px",
    display: "flex",
    alignItems: "stretch",
    flexShrink: 0,
    backgroundColor: "transparent",
    userSelect: "none",
  },
  titleBarDrag: {
    flex: 1,
    display: "flex",
    alignItems: "center",
    paddingLeft: tokens.spacingHorizontalM,
    fontSize: "12px",
    color: tokens.colorNeutralForeground2,
    fontWeight: 600,
    letterSpacing: "0.02em",
  },
  titleBarControls: { display: "flex", alignItems: "stretch" },
  winBtn: {
    width: "46px",
    height: "32px",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "transparent",
    borderTopStyle: "none",
    borderRightStyle: "none",
    borderBottomStyle: "none",
    borderLeftStyle: "none",
    cursor: "default",
    color: tokens.colorNeutralForeground1,
    transitionDuration: "100ms",
    transitionProperty: "background-color",
    ":hover": { backgroundColor: tokens.colorNeutralBackground2Hover },
  },
  winBtnClose: {
    ":hover": {
      backgroundColor: "#c42b1c",
      color: "#ffffff",
    },
  },
  topbar: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    ...shorthands.padding(tokens.spacingVerticalL, tokens.spacingHorizontalXXXL),
    borderBottomWidth: "1px", borderBottomStyle: "solid", borderBottomColor: tokens.colorNeutralStroke3,
  },
  topbarLeft: { display: "flex", flexDirection: "column", gap: "2px" },
  topbarRight: { display: "flex", alignItems: "center", gap: tokens.spacingHorizontalL },
  brand: { fontWeight: 600, fontSize: "14px", color: tokens.colorNeutralForeground2, letterSpacing: "0.04em" },
  device: { fontSize: "20px", fontWeight: 600 },
  statusRow: { display: "flex", alignItems: "center", gap: "6px", fontVariantNumeric: "tabular-nums" },
  codeBlock: { display: "flex", flexDirection: "column", alignItems: "flex-end", gap: "2px" },
  codeLabel: { color: tokens.colorNeutralForeground3, fontSize: "11px", letterSpacing: "0.06em", textTransform: "uppercase" },
  codeValueRow: { display: "flex", alignItems: "center", gap: tokens.spacingHorizontalS },
  codeValue: { fontSize: "28px", fontWeight: 600, letterSpacing: "0.22em", fontVariantNumeric: "tabular-nums", color: tokens.colorNeutralForeground1 },
  main: {
    flex: 1, display: "flex", flexDirection: "column", gap: tokens.spacingVerticalXL,
    ...shorthands.padding(tokens.spacingVerticalXXL, tokens.spacingHorizontalXXXL),
    maxWidth: "920px", width: "100%", marginLeft: "auto", marginRight: "auto", boxSizing: "border-box",
  },
  sectionHead: { display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: tokens.spacingVerticalXS },
  sectionLabel: { color: tokens.colorNeutralForeground2, fontSize: "12px", fontWeight: 600, letterSpacing: "0.06em", textTransform: "uppercase" },
  dropZone: {
    borderRadius: tokens.borderRadiusXLarge, backgroundColor: tokens.colorNeutralBackground2,
    borderTopWidth: "1px", borderRightWidth: "1px", borderBottomWidth: "1px", borderLeftWidth: "1px",
    borderTopStyle: "dashed", borderRightStyle: "dashed", borderBottomStyle: "dashed", borderLeftStyle: "dashed",
    borderTopColor: tokens.colorNeutralStroke2, borderRightColor: tokens.colorNeutralStroke2,
    borderBottomColor: tokens.colorNeutralStroke2, borderLeftColor: tokens.colorNeutralStroke2,
    ...shorthands.padding("44px", tokens.spacingHorizontalXXL),
    display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center",
    gap: tokens.spacingVerticalS,
    transitionDuration: "180ms", transitionTimingFunction: "ease-out",
    transitionProperty: "background-color, border-color, transform",
    minHeight: "180px",
  },
  dropZoneActive: {
    backgroundColor: tokens.colorBrandBackground2,
    borderTopColor: tokens.colorBrandStroke2, borderRightColor: tokens.colorBrandStroke2,
    borderBottomColor: tokens.colorBrandStroke2, borderLeftColor: tokens.colorBrandStroke2,
  },
  dropZoneFilled: {
    borderTopStyle: "solid", borderRightStyle: "solid", borderBottomStyle: "solid", borderLeftStyle: "solid",
    backgroundColor: tokens.colorNeutralBackground1,
  },
  dropIcon: { color: tokens.colorNeutralForeground3, fontSize: "44px" },
  dropTitle: { fontSize: "16px", fontWeight: 500, color: tokens.colorNeutralForeground1 },
  dropHint: { color: tokens.colorNeutralForeground3 },
  dropActions: { marginTop: tokens.spacingVerticalS, display: "flex", gap: tokens.spacingHorizontalS },
  fileList: { display: "flex", flexDirection: "column", gap: "2px", width: "100%", maxWidth: "560px" },
  fileRow: { display: "flex", alignItems: "center", gap: tokens.spacingHorizontalS, ...shorthands.padding("6px", tokens.spacingHorizontalS), borderRadius: tokens.borderRadiusMedium },
  filePath: { flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: "13px", color: tokens.colorNeutralForeground2 },
  peerGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))", gap: tokens.spacingHorizontalM },
  peerTile: {
    ...shorthands.padding(tokens.spacingVerticalM, tokens.spacingHorizontalM),
    borderRadius: tokens.borderRadiusLarge, backgroundColor: tokens.colorNeutralBackground2,
    borderTopWidth: "1px", borderRightWidth: "1px", borderBottomWidth: "1px", borderLeftWidth: "1px",
    borderTopStyle: "solid", borderRightStyle: "solid", borderBottomStyle: "solid", borderLeftStyle: "solid",
    borderTopColor: tokens.colorTransparentStroke, borderRightColor: tokens.colorTransparentStroke,
    borderBottomColor: tokens.colorTransparentStroke, borderLeftColor: tokens.colorTransparentStroke,
    cursor: "pointer", transitionDuration: "120ms", transitionTimingFunction: "ease-out",
    transitionProperty: "background-color, border-color",
    display: "flex", flexDirection: "column", gap: "4px",
  },
  peerTileActive: {
    borderTopColor: tokens.colorBrandStroke1, borderRightColor: tokens.colorBrandStroke1,
    borderBottomColor: tokens.colorBrandStroke1, borderLeftColor: tokens.colorBrandStroke1,
    backgroundColor: tokens.colorBrandBackground2,
  },
  peerName: { fontWeight: 600, fontSize: "14px" },
  peerMeta: { color: tokens.colorNeutralForeground3, fontSize: "12px", fontVariantNumeric: "tabular-nums" },
  emptyHint: { color: tokens.colorNeutralForeground3, fontSize: "13px" },
  composeRow: { display: "grid", gridTemplateColumns: "1fr auto", gap: tokens.spacingHorizontalM, alignItems: "end" },
  codeInputWrap: { display: "flex", flexDirection: "column", gap: "4px" },
  sendButton: { alignSelf: "stretch" },
  transferRow: { display: "flex", flexDirection: "column", gap: "6px", ...shorthands.padding(tokens.spacingVerticalS, "0") },
  transferHeader: { display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "13px" },
  metricsRow: {
    display: "flex",
    gap: tokens.spacingHorizontalXL,
    flexWrap: "wrap",
    alignItems: "flex-end",
    fontVariantNumeric: "tabular-nums",
    marginTop: "4px",
  },
  metric: { display: "flex", flexDirection: "column", gap: "2px", minWidth: "0" },
  metricLabel: {
    fontSize: "10px",
    fontWeight: 600,
    letterSpacing: "0.06em",
    textTransform: "uppercase",
    color: tokens.colorNeutralForeground3,
    lineHeight: 1,
  },
  metricValue: { fontSize: "14px", fontWeight: 600, lineHeight: 1.2 },
  metricValueMuted: { fontSize: "13px", fontWeight: 400, lineHeight: 1.2, color: tokens.colorNeutralForeground2 },
  metricSecondary: {
    fontSize: "12px",
    color: tokens.colorNeutralForeground3,
    fontVariantNumeric: "tabular-nums",
    marginTop: "2px",
  },
  historyRow: { display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "12px", color: tokens.colorNeutralForeground3, ...shorthands.padding("2px", "0") },
  setupRoot: { flex: 1, display: "flex", alignItems: "center", justifyContent: "center", ...shorthands.padding(tokens.spacingVerticalXXL) },
  setupCard: { width: "100%", maxWidth: "440px", display: "flex", flexDirection: "column", gap: tokens.spacingVerticalL, ...shorthands.padding(tokens.spacingVerticalXXL, tokens.spacingHorizontalXXXL), backgroundColor: tokens.colorNeutralBackground1, borderRadius: tokens.borderRadiusXLarge },
  manualIp: { display: "flex", gap: tokens.spacingHorizontalS, alignItems: "center" },
  settingsSection: { display: "flex", flexDirection: "column", gap: tokens.spacingVerticalM, marginTop: tokens.spacingVerticalL },
  settingsRow: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: tokens.spacingHorizontalM },
  trustedItem: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    ...shorthands.padding(tokens.spacingVerticalXS, tokens.spacingHorizontalS),
    borderRadius: tokens.borderRadiusMedium, backgroundColor: tokens.colorNeutralBackground2,
  },
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function shadeHex(hex: string, factor: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  const adjust = (c: number) =>
    Math.round(factor > 0 ? c + (255 - c) * factor : c * (1 + factor));
  const clamp = (c: number) => Math.max(0, Math.min(255, c));
  const hh = (n: number) => clamp(n).toString(16).padStart(2, "0");
  return `#${hh(adjust(r))}${hh(adjust(g))}${hh(adjust(b))}`;
}

/// Override Fluent's brand colors with the supplied accent. Approximate
/// the brand ramp by lighten/darken — Fluent's real BrandVariants would
/// need a full 16-step HSL ramp; this is good enough for buttons,
/// badges and progress.
function applyAccentToTheme(base: Theme, accent: string | null): Theme {
  if (!accent || !/^#[0-9A-Fa-f]{6}$/.test(accent)) return base;
  return {
    ...base,
    colorBrandBackground: accent,
    colorBrandBackgroundHover: shadeHex(accent, 0.12),
    colorBrandBackgroundPressed: shadeHex(accent, -0.12),
    colorBrandBackgroundSelected: shadeHex(accent, -0.05),
    colorBrandForeground1: accent,
    colorBrandForeground2: shadeHex(accent, -0.05),
    colorBrandStroke1: accent,
    colorBrandStroke2: shadeHex(accent, 0.3),
    colorCompoundBrandBackground: accent,
    colorCompoundBrandBackgroundHover: shadeHex(accent, 0.1),
    colorCompoundBrandBackgroundPressed: shadeHex(accent, -0.1),
    colorCompoundBrandForeground1: accent,
    colorCompoundBrandStroke: accent,
  };
}

function useEffectiveTheme(pref: ThemePref | undefined, accent: string | null): Theme {
  const compute = (): Theme => {
    const sysIsDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    const want = pref === "system" || pref === undefined ? (sysIsDark ? "dark" : "light") : pref;
    const base = want === "dark" ? webDarkTheme : webLightTheme;
    return applyAccentToTheme(base, accent);
  };
  const [theme, setTheme] = useState<Theme>(compute);
  useEffect(() => {
    setTheme(compute());
    if (pref !== "system" && pref !== undefined) return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => setTheme(compute());
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pref, accent]);
  return theme;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function basename(p: string): string {
  const parts = p.split(/[\\/]/);
  return parts[parts.length - 1] || p;
}

/// Format an ETA in seconds as a compact human string: "27s" / "2m 15s"
/// / "1h 30m" / "2d 4h". Returns "—" when the rate is too unstable to
/// estimate (zero / non-finite).
function formatEta(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) return "—";
  if (seconds < 1) return "<1s";
  if (seconds < 60) return `${Math.floor(seconds)}s`;
  if (seconds < 3600) {
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}m ${s}s`;
  }
  if (seconds < 86400) {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return `${h}h ${m}m`;
  }
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  return `${d}d ${h}h`;
}

function playChime(kind: "incoming" | "done"): void {
  try {
    const Ctor = (window.AudioContext ||
      (window as unknown as { webkitAudioContext: typeof AudioContext })
        .webkitAudioContext) as typeof AudioContext;
    const ctx = new Ctor();
    const now = ctx.currentTime;
    const tones = kind === "incoming" ? [660, 880] : [880, 1320];
    const dur = 0.14;
    tones.forEach((freq, i) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      gain.gain.setValueAtTime(0.0001, now + i * dur);
      gain.gain.exponentialRampToValueAtTime(0.08, now + i * dur + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + i * dur + dur);
      osc.connect(gain).connect(ctx.destination);
      osc.start(now + i * dur);
      osc.stop(now + i * dur + dur + 0.02);
    });
    setTimeout(() => ctx.close().catch(() => {}), 600);
  } catch {
    /* ignore */
  }
}

async function notify(title: string, body: string): Promise<void> {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const res = await requestPermission();
      granted = res === "granted";
    }
    if (granted) sendNotification({ title, body });
  } catch {
    /* ignore */
  }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ActiveTransfer {
  id: string;
  direction: "send" | "recv";
  peer: string;
  filesTotal: number;
  filesDone: number;
  totalBytes: number;
  totalBytesDone: number;
  currentFile: string;
  startedAt: number;
  instantNetwork: number;
  instantDisk: number;
  paused: boolean;
}

interface IncomingPending {
  request: IncomingRequest;
  overrideDir: string | null;
  trust: boolean;
}

// ---------------------------------------------------------------------------
// Language picker
// ---------------------------------------------------------------------------

function TitleBar() {
  const styles = useStyles();
  const onMin = () => getCurrentWebviewWindow().minimize().catch(() => {});
  const onMax = () => getCurrentWebviewWindow().toggleMaximize().catch(() => {});
  const onClose = () => getCurrentWebviewWindow().close().catch(() => {});
  return (
    <div className={styles.titleBar}>
      <div className={styles.titleBarDrag} data-tauri-drag-region>
        LanBlaze
      </div>
      <div className={styles.titleBarControls}>
        <button className={styles.winBtn} onClick={onMin} aria-label="Minimize">
          <Subtract20Regular />
        </button>
        <button className={styles.winBtn} onClick={onMax} aria-label="Maximize">
          <Square20Regular />
        </button>
        <button
          className={`${styles.winBtn} ${styles.winBtnClose}`}
          onClick={onClose}
          aria-label="Close"
        >
          <Dismiss20Regular />
        </button>
      </div>
    </div>
  );
}

function LanguagePicker() {
  const { i18n } = useTranslation();
  const current = SUPPORTED_LANGS.find((l) => l.code === i18n.language)?.name ?? "English";
  return (
    <Menu>
      <MenuTrigger disableButtonEnhancement>
        <Button appearance="subtle" size="small" icon={<LocalLanguage20Regular />}>
          {current}
        </Button>
      </MenuTrigger>
      <MenuPopover>
        <MenuList>
          {SUPPORTED_LANGS.map((l) => (
            <MenuItem key={l.code} onClick={() => setLang(l.code)}>{l.name}</MenuItem>
          ))}
        </MenuList>
      </MenuPopover>
    </Menu>
  );
}

// ---------------------------------------------------------------------------
// Setup wizard
// ---------------------------------------------------------------------------

function SetupWizard({ initial, onDone }: { initial: Session; onDone: (s: Session) => void }) {
  const styles = useStyles();
  const theme = useEffectiveTheme(initial.settings.theme, null);
  const { t } = useTranslation();
  const [name, setName] = useState(initial.settings.deviceName);
  const [dir, setDir] = useState(initial.settings.saveDir);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const pickDir = async () => {
    const r = await open({ directory: true, multiple: false });
    if (typeof r === "string") setDir(r);
  };
  const handleSave = async () => {
    setError(null);
    setBusy(true);
    try {
      await saveSettings(name, dir);
      try { await ensureReceiver(); } catch (e) { console.warn(e); }
      const fresh = await getSession();
      onDone(fresh);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <FluentProvider theme={theme} style={{ backgroundColor: "transparent" }}>
      <div className={styles.app}>
        <TitleBar />
        <div style={{ display: "flex", justifyContent: "flex-end", padding: 12 }}>
          <LanguagePicker />
        </div>
        <div className={styles.setupRoot}>
          <div className={styles.setupCard}>
            <div>
              <div className={styles.brand}>{t("setup.brand")}</div>
              <Title2>{t("setup.title")}</Title2>
            </div>
            <Body1 style={{ color: tokens.colorNeutralForeground3 }}>{t("setup.desc")}</Body1>
            <Field label={t("setup.deviceName")}>
              <Input value={name} onChange={(_, d) => setName(d.value)} />
            </Field>
            <Field label={t("setup.saveDir")} hint={t("setup.saveDirHint")}>
              <div className={styles.manualIp}>
                <Input value={dir} onChange={(_, d) => setDir(d.value)} style={{ flex: 1 }} readOnly />
                <Button icon={<Folder20Regular />} onClick={pickDir}>{t("common.select")}</Button>
              </div>
            </Field>
            {error && (
              <MessageBar intent="error">
                <MessageBarBody>{error}</MessageBarBody>
              </MessageBar>
            )}
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <Button appearance="primary" onClick={handleSave} disabled={busy}>
                {busy ? t("setup.preparing") : t("setup.start")}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </FluentProvider>
  );
}

// ---------------------------------------------------------------------------
// DropZone
// ---------------------------------------------------------------------------

function DropZone({
  paths, dragOver, onPickFiles, onPickFolder, onClear, onRemoveOne,
}: {
  paths: string[]; dragOver: boolean;
  onPickFiles: () => void; onPickFolder: () => void; onClear: () => void; onRemoveOne: (p: string) => void;
}) {
  const styles = useStyles();
  const { t } = useTranslation();
  const isFilled = paths.length > 0;
  const classes = [styles.dropZone, dragOver ? styles.dropZoneActive : "", isFilled ? styles.dropZoneFilled : ""].filter(Boolean).join(" ");

  return (
    <div className={classes}>
      {!isFilled ? (
        <>
          <CloudArrowUp48Regular className={styles.dropIcon} />
          <div className={styles.dropTitle}>{t("step1.title")}</div>
          <div className={styles.dropHint}>{t("step1.sub")}</div>
          <div className={styles.dropActions}>
            <Button icon={<Document20Regular />} onClick={onPickFiles}>{t("step1.file")}</Button>
            <Button icon={<Folder20Regular />} onClick={onPickFolder}>{t("step1.folder")}</Button>
          </div>
        </>
      ) : (
        <>
          <div className={styles.fileList}>
            {paths.slice(0, 6).map((p) => (
              <div key={p} className={styles.fileRow}>
                <Document20Regular />
                <Tooltip content={p} relationship="label">
                  <span className={styles.filePath}>{basename(p)}</span>
                </Tooltip>
                <Button size="small" appearance="subtle" icon={<Dismiss20Regular />} onClick={() => onRemoveOne(p)} aria-label={t("step1.remove")} />
              </div>
            ))}
            {paths.length > 6 && (
              <Caption1 style={{ paddingLeft: 8 }}>{t("step1.more", { count: paths.length - 6 })}</Caption1>
            )}
          </div>
          <div className={styles.dropActions}>
            <Button icon={<Document20Regular />} onClick={onPickFiles}>{t("step1.addFile")}</Button>
            <Button icon={<Folder20Regular />} onClick={onPickFolder}>{t("step1.addFolder")}</Button>
            <Button appearance="subtle" onClick={onClear}>{t("common.clear")}</Button>
          </div>
        </>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// PeerTile
// ---------------------------------------------------------------------------

function PeerTile({ peer, active, onSelect }: { peer: Peer; active: boolean; onSelect: () => void }) {
  const styles = useStyles();
  return (
    <div
      role="button"
      tabIndex={0}
      className={[styles.peerTile, active ? styles.peerTileActive : ""].join(" ")}
      onClick={onSelect}
      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect(); } }}
      aria-pressed={active}
      aria-label={`${peer.deviceName} ${peer.addresses[0] ?? ""}:${peer.port}`}
    >
      <div className={styles.peerName}>{peer.deviceName}</div>
      <div className={styles.peerMeta}>{peer.addresses[0] ?? "?"}:{peer.port}</div>
      <div className={styles.peerMeta}>{peer.os}</div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Settings drawer
// ---------------------------------------------------------------------------

function SettingsDrawer({
  open: isOpen, onOpenChange, session, onSettingsUpdated,
  useAccent, onToggleAccent, accentHex,
}: {
  open: boolean; onOpenChange: (v: boolean) => void;
  session: Session; onSettingsUpdated: (s: UserSettings) => void;
  useAccent: boolean; onToggleAccent: (v: boolean) => void; accentHex: string | null;
}) {
  const styles = useStyles();
  const { t } = useTranslation();
  const [history, setHistory] = useState<HistoryRecord[]>([]);
  const [updateBusy, setUpdateBusy] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [updateAvailable, setUpdateAvailable] = useState<UpdateInfo | null>(null);

  const onCheckUpdate = async () => {
    setUpdateBusy(true);
    setUpdateStatus(t("update.checking"));
    setUpdateAvailable(null);
    try {
      const current = await tauriGetVersion();
      const info = await checkForUpdate(current);
      localStorage.setItem("update.lastCheck", Date.now().toString());
      if (info) {
        setUpdateAvailable(info);
        setUpdateStatus(null);
      } else {
        setUpdateStatus(t("update.upToDate"));
      }
    } catch (e) {
      setUpdateStatus(String(e));
    } finally {
      setUpdateBusy(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      getHistory().then((h) => setHistory(h.records)).catch(() => {});
    }
  }, [isOpen]);

  const onTheme = async (theme: ThemePref) => {
    try { onSettingsUpdated(await setTheme(theme)); } catch (e) { console.error(e); }
  };
  const onSound = async (v: boolean) => {
    try { onSettingsUpdated(await setSoundEnabled(v)); } catch (e) { console.error(e); }
  };
  const onNotif = async (v: boolean) => {
    try { onSettingsUpdated(await setNotificationsEnabled(v)); } catch (e) { console.error(e); }
  };
  const onAuto = async (v: boolean) => {
    try { onSettingsUpdated(await setAutoStart(v)); } catch (e) { console.error(e); }
  };
  const onTray = async (v: boolean) => {
    try { onSettingsUpdated(await setCloseToTray(v)); } catch (e) { console.error(e); }
  };
  const onUntrust = async (name: string) => {
    try { onSettingsUpdated(await untrustDevice(name)); } catch (e) { console.error(e); }
  };
  const onClearHistory = async () => {
    try { await clearHistory(); setHistory([]); } catch (e) { console.error(e); }
  };

  const trusted = Object.values(session.settings.trustedDevices ?? {});

  return (
    <Drawer
      open={isOpen}
      onOpenChange={(_, d) => onOpenChange(d.open)}
      position="end"
      size="medium"
    >
      <DrawerHeader>
        <DrawerHeaderTitle
          action={
            <Button
              appearance="subtle"
              aria-label={t("settings.close")}
              icon={<Dismiss20Regular />}
              onClick={() => onOpenChange(false)}
            />
          }
        >
          {t("settings.title")}
        </DrawerHeaderTitle>
      </DrawerHeader>
      <DrawerBody>
        <div className={styles.settingsSection}>
          <span className={styles.sectionLabel}>{t("settings.appearance")}</span>
          <Field label={t("settings.theme")}>
            <RadioGroup
              value={session.settings.theme ?? "system"}
              onChange={(_, d) => onTheme(d.value as ThemePref)}
              layout="horizontal"
            >
              <Radio value="system" label={t("settings.themeSystem")} />
              <Radio value="light" label={t("settings.themeLight")} />
              <Radio value="dark" label={t("settings.themeDark")} />
            </RadioGroup>
          </Field>
          <div className={styles.settingsRow}>
            <span>
              {t("settings.useAccent")}
              {accentHex && (
                <span style={{
                  display: "inline-block", width: 12, height: 12,
                  marginLeft: 8, borderRadius: 3, verticalAlign: "middle",
                  backgroundColor: accentHex,
                  border: `1px solid ${tokens.colorNeutralStroke2}`,
                }} />
              )}
            </span>
            <Switch checked={useAccent} onChange={(_, d) => onToggleAccent(d.checked)} />
          </div>
        </div>

        <div className={styles.settingsSection}>
          <span className={styles.sectionLabel}>{t("settings.notifications")}</span>
          <div className={styles.settingsRow}>
            <span>{t("settings.soundEnabled")}</span>
            <Switch checked={session.settings.soundEnabled ?? true} onChange={(_, d) => onSound(d.checked)} />
          </div>
          <div className={styles.settingsRow}>
            <span>{t("settings.notificationsEnabled")}</span>
            <Switch checked={session.settings.notificationsEnabled ?? true} onChange={(_, d) => onNotif(d.checked)} />
          </div>
        </div>

        <div className={styles.settingsSection}>
          <span className={styles.sectionLabel}>{t("settings.behaviour")}</span>
          <div className={styles.settingsRow}>
            <span>{t("settings.autoStart")}</span>
            <Switch checked={session.settings.autoStart ?? false} onChange={(_, d) => onAuto(d.checked)} />
          </div>
          <div className={styles.settingsRow}>
            <span>{t("settings.closeToTray")}</span>
            <Switch checked={session.settings.closeToTray ?? false} onChange={(_, d) => onTray(d.checked)} />
          </div>
        </div>

        <div className={styles.settingsSection}>
          <span className={styles.sectionLabel}>{t("settings.trustedDevices")}</span>
          {trusted.length === 0 ? (
            <Caption1>{t("settings.trustedEmpty")}</Caption1>
          ) : (
            trusted.map((d) => (
              <div key={d.name} className={styles.trustedItem}>
                <span>{d.name}</span>
                <Button
                  size="small"
                  appearance="subtle"
                  icon={<Delete20Regular />}
                  onClick={() => onUntrust(d.name)}
                  aria-label={t("settings.untrust")}
                >
                  {t("settings.untrust")}
                </Button>
              </div>
            ))
          )}
        </div>

        <div className={styles.settingsSection}>
          <span className={styles.sectionLabel}>{t("update.sectionLabel")}</span>
          <div className={styles.settingsRow}>
            <Button
              icon={<ArrowSync20Regular />}
              onClick={onCheckUpdate}
              disabled={updateBusy}
            >
              {updateBusy ? t("update.checking") : t("update.checkNow")}
            </Button>
            {updateStatus && (
              <Caption1 style={{ color: tokens.colorNeutralForeground3 }}>
                {updateStatus}
              </Caption1>
            )}
          </div>
          {updateAvailable && (
            <MessageBar intent="info">
              <MessageBarBody>
                <MessageBarTitle>
                  {t("update.available", { version: updateAvailable.version })}
                </MessageBarTitle>
              </MessageBarBody>
              <Button
                appearance="primary"
                size="small"
                onClick={() => openExternalUrl(updateAvailable.htmlUrl).catch(console.error)}
              >
                {t("update.download")}
              </Button>
            </MessageBar>
          )}
        </div>

        <div className={styles.settingsSection}>
          <div className={styles.settingsRow}>
            <span className={styles.sectionLabel}>{t("settings.recent")}</span>
            {history.length > 0 && (
              <Button size="small" appearance="subtle" onClick={onClearHistory}>{t("settings.clear")}</Button>
            )}
          </div>
          {history.length === 0 ? (
            <Caption1>{t("settings.recentEmpty")}</Caption1>
          ) : (
            history.slice(0, 50).map((r) => {
              const label = r.success
                ? "✓"
                : r.error === "canceled_by_user"
                  ? t("history.canceledByUser")
                  : r.error === "canceled_by_peer"
                    ? t("history.canceledByPeer")
                    : "✗";
              return (
                <div key={r.id} className={styles.historyRow}>
                  <span>
                    {r.direction === "send" ? "↑" : "↓"} {formatBytes(r.bytes)} · {r.peer}
                  </span>
                  <span style={{
                    color: r.success ? tokens.colorPaletteGreenForeground1 : tokens.colorPaletteRedForeground1,
                  }}>
                    {label}
                  </span>
                </div>
              );
            })
          )}
        </div>
      </DrawerBody>
    </Drawer>
  );
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

function App() {
  const styles = useStyles();
  const { t } = useTranslation();

  const [session, setSession] = useState<Session | null>(null);
  const [peers, setPeers] = useState<Record<string, Peer>>({});
  const [active, setActive] = useState<Record<string, ActiveTransfer>>({});
  const [incoming, setIncoming] = useState<IncomingPending | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [accent, setAccent] = useState<string | null>(null);
  const [useAccent, setUseAccent] = useState<boolean>(() => {
    return localStorage.getItem("ui.useAccent") !== "false";
  });

  const theme = useEffectiveTheme(session?.settings.theme, useAccent ? accent : null);

  const [selectedPeer, setSelectedPeer] = useState<Peer | null>(null);
  const [manualIp, setManualIp] = useState("");
  const [peerCode, setPeerCode] = useState("");
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [dragOver, setDragOver] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);
  const [busySend, setBusySend] = useState(false);
  const [showManualIp, setShowManualIp] = useState(false);

  const incomingRef = useRef<IncomingPending | null>(null);
  incomingRef.current = incoming;
  const sessionRef = useRef<Session | null>(null);
  sessionRef.current = session;

  // Initial session load.
  useEffect(() => {
    (async () => {
      try {
        let s = await getSession();
        if (s.settings.configured && !s.receiverRunning) {
          try { await ensureReceiver(); s = await getSession(); } catch (e) { console.warn(e); }
        }
        setSession(s);
      } catch (e) { console.error(e); }
    })();
  }, []);

  // Update check on startup, throttled to once per 12 hours.
  useEffect(() => {
    (async () => {
      const TWELVE_HOURS = 12 * 60 * 60 * 1000;
      const last = Number(localStorage.getItem("update.lastCheck") ?? "0");
      if (Date.now() - last < TWELVE_HOURS) return;
      try {
        const current = await tauriGetVersion();
        const info = await checkForUpdate(current);
        localStorage.setItem("update.lastCheck", Date.now().toString());
        if (info) {
          const dismissed = localStorage.getItem("update.dismissed");
          if (dismissed !== info.version) setUpdate(info);
        }
      } catch (e) {
        console.warn("update check failed:", e);
      }
    })();
  }, []);

  // Receiver-ready → refresh session.
  useEffect(() => {
    let stop: Unlisten | undefined;
    onReceiverReady(async () => {
      try { setSession(await getSession()); } catch {}
    }).then((u) => { stop = u; });
    return () => { stop?.(); };
  }, []);

  // Read Windows accent color on mount + every time the settings drawer
  // closes (cheap, lets the UI pick up colour changes without polling).
  useEffect(() => {
    getAccentColor().then((c) => c && setAccent(c)).catch(() => {});
  }, [showSettings]);

  // Receive shell-context-menu / drag-onto-icon file arguments.
  useEffect(() => {
    let unlisten: Unlisten | undefined;
    listen<string[]>("shell://send-paths", (e) => {
      if (Array.isArray(e.payload) && e.payload.length > 0) {
        setSelectedPaths((prev) => Array.from(new Set([...prev, ...e.payload])));
      }
    }).then((u) => { unlisten = u; });
    return () => { unlisten?.(); };
  }, []);

  // Subscriptions.
  useEffect(() => {
    const unsubs: Promise<Unlisten>[] = [];
    unsubs.push(onTransferStarted((e: TransferStarted) => {
      setActive((prev) => ({
        ...prev,
        [e.id]: {
          id: e.id, direction: e.direction, peer: e.peer,
          filesTotal: e.fileCount, filesDone: 0,
          totalBytes: e.totalBytes, totalBytesDone: 0,
          currentFile: "", startedAt: performance.now(),
          instantNetwork: 0, instantDisk: 0, paused: false,
        },
      }));
    }));
    unsubs.push(onTransferProgress((e: TransferProgress) => {
      setActive((prev) => {
        const a = prev[e.id]; if (!a) return prev;
        return { ...prev, [e.id]: { ...a,
          filesDone: e.filesDone, filesTotal: e.filesTotal,
          totalBytes: e.totalBytes, totalBytesDone: e.totalBytesDone,
          currentFile: e.currentFile,
          instantNetwork: e.instantMbpsNetwork,
          instantDisk: e.instantMbpsDisk,
        } };
      });
    }));
    unsubs.push(onTransferCompleted((e: TransferCompleted) => {
      setActive((prev) => {
        const { [e.id]: _g, ...rest } = prev; return rest;
      });
      const s = sessionRef.current?.settings;
      if (s?.soundEnabled !== false) playChime("done");
      if (s?.notificationsEnabled !== false) {
        let title: string;
        let body: string;
        if (e.success) {
          title = e.direction === "send" ? t("active.send") : t("active.recv");
          body = `${formatBytes(e.totalBytes)} · ${(e.elapsedMs / 1000).toFixed(1)}s`;
        } else if (e.error === "canceled_by_user") {
          title = t("history.canceledByUser");
          body = e.direction === "send" ? t("active.send") : t("active.recv");
        } else if (e.error === "canceled_by_peer") {
          title = t("history.canceledByPeer");
          body = e.direction === "send" ? t("active.send") : t("active.recv");
        } else {
          title = t("step3.errorTitle");
          body = e.error ?? "—";
        }
        notify(`LanBlaze · ${title}`, body);
      }
    }));
    unsubs.push(onPeerAdded((p: Peer) => setPeers((prev) => ({ ...prev, [p.instance]: p }))));
    unsubs.push(onPeerRemoved((instance: string) => setPeers((prev) => {
      const { [instance]: _g, ...rest } = prev; return rest;
    })));
    unsubs.push(onIncomingRequest((r: IncomingRequest) => {
      setIncoming({ request: r, overrideDir: null, trust: false });
      const s = sessionRef.current?.settings;
      if (s?.soundEnabled !== false) playChime("incoming");
      if (s?.notificationsEnabled !== false) {
        notify(`LanBlaze · ${t("incoming.title")}`, `${r.deviceName} · ${r.fileCount} · ${formatBytes(r.totalBytes)}`);
      }
    }));
    return () => { unsubs.forEach((u) => u.then((fn) => fn()).catch(() => {})); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const win = getCurrentWebviewWindow();
    const p = win.onDragDropEvent((event) => {
      const e: any = event.payload;
      if (e.type === "enter" || e.type === "over") setDragOver(true);
      else if (e.type === "leave") setDragOver(false);
      else if (e.type === "drop") {
        setDragOver(false);
        const paths: string[] = e.paths ?? [];
        if (paths.length > 0) {
          setSelectedPaths((prev) => Array.from(new Set([...prev, ...paths])));
        }
      }
    });
    return () => { p.then((u) => u()).catch(() => {}); };
  }, []);

  const peerList = useMemo(() => Object.values(peers), [peers]);
  const activeList = useMemo(() => Object.values(active), [active]);

  // Drive the Windows taskbar progress overlay from whichever transfer
  // is currently most advanced (or all of them combined).
  useEffect(() => {
    if (activeList.length === 0) {
      setTaskbarProgress(0, 1, "none").catch(() => {});
      return;
    }
    const totalBytes = activeList.reduce((s, a) => s + a.totalBytes, 0);
    const doneBytes = activeList.reduce((s, a) => s + a.totalBytesDone, 0);
    const anyPaused = activeList.some((a) => a.paused);
    setTaskbarProgress(doneBytes, totalBytes, anyPaused ? "paused" : "normal").catch(() => {});
  }, [activeList]);

  const handleRegenerate = async () => {
    const code = await regenerateCode();
    setSession((s) => (s ? { ...s, authCode: code } : s));
  };
  const handleChangeSaveDir = async () => {
    const d = await open({ directory: true, multiple: false });
    if (typeof d === "string" && session) {
      try {
        const updated = await saveSettings(session.settings.deviceName, d);
        setSession({ ...session, settings: updated });
      } catch (e) { console.error(e); }
    }
  };
  const handleRenameDevice = async () => {
    if (!session) return;
    const next = window.prompt(t("topbar.rename"), session.settings.deviceName);
    if (next && next.trim() && next !== session.settings.deviceName) {
      try {
        const updated = await saveSettings(next.trim(), session.settings.saveDir);
        setSession({ ...session, settings: updated });
      } catch (e) { console.error(e); }
    }
  };
  const pickFiles = async () => {
    const f = await open({ directory: false, multiple: true });
    if (Array.isArray(f)) setSelectedPaths((prev) => Array.from(new Set([...prev, ...f])));
    else if (typeof f === "string") setSelectedPaths((prev) => Array.from(new Set([...prev, f])));
  };
  const pickFolder = async () => {
    const f = await open({ directory: true, multiple: false });
    if (typeof f === "string") setSelectedPaths((prev) => Array.from(new Set([...prev, f])));
  };
  const removeOne = (p: string) => setSelectedPaths((prev) => prev.filter((x) => x !== p));
  const clearPaths = () => setSelectedPaths([]);

  const startReceiverNow = async () => {
    try { await ensureReceiver(); setSession(await getSession()); }
    catch (e) { console.error(e); }
  };

  const togglePauseTransfer = async (a: ActiveTransfer) => {
    try {
      if (a.paused) {
        await resumeTransfer(a.id);
      } else {
        await pauseTransfer(a.id);
      }
      setActive((prev) => {
        const cur = prev[a.id]; if (!cur) return prev;
        return { ...prev, [a.id]: { ...cur, paused: !cur.paused } };
      });
    } catch (e) {
      console.error(e);
    }
  };

  const handleToggleAccent = (v: boolean) => {
    setUseAccent(v);
    try { localStorage.setItem("ui.useAccent", v ? "true" : "false"); } catch {}
  };

  const handleSend = async () => {
    setSendError(null);
    const ip = selectedPeer?.addresses[0] ?? manualIp.trim();
    const port = selectedPeer?.port ?? session?.defaultPort;
    if (!ip) return setSendError(t("send.needDevice"));
    if (selectedPaths.length === 0) return setSendError(t("send.needPaths"));
    const code = peerCode.replace(/\D/g, "");
    if (code.length !== 6) return setSendError(t("send.needCode"));
    if (ip === "127.0.0.1" || ip === "localhost" || (session?.localIp && ip === session.localIp)) {
      try { await ensureReceiver(); }
      catch (e) { return setSendError(t("send.receiverFailed", { error: String(e) })); }
    }
    setBusySend(true);
    try {
      await sendPaths(ip, selectedPaths, code, port);
      setSelectedPaths([]);
      setPeerCode("");
    } catch (e) {
      setSendError(String(e));
    } finally {
      setBusySend(false);
    }
  };

  const respondAndClose = async (accept: boolean) => {
    const pending = incomingRef.current;
    if (!pending) return;
    try {
      if (accept && pending.trust) {
        const updated = await trustDevice(pending.request.deviceName);
        setSession((s) => (s ? { ...s, settings: updated } : s));
      }
      await respondIncoming(pending.request.id, accept, pending.overrideDir ?? undefined);
    } catch (e) { console.error(e); }
    finally { setIncoming(null); }
  };
  const pickIncomingDir = async () => {
    const d = await open({ directory: true, multiple: false });
    if (typeof d === "string" && incoming) {
      setIncoming({ ...incoming, overrideDir: d });
    }
  };

  if (!session) {
    return (
      <FluentProvider theme={theme} style={{ backgroundColor: "transparent" }}>
        <div className={styles.app}>
          <TitleBar />
          <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
            <Spinner label={t("common.busy")} />
          </div>
        </div>
      </FluentProvider>
    );
  }
  if (!session.settings.configured) {
    return <SetupWizard initial={session} onDone={setSession} />;
  }

  const target = selectedPeer
    ? `${selectedPeer.deviceName} · ${selectedPeer.addresses[0]}:${selectedPeer.port}`
    : manualIp ? `${manualIp}:${session.defaultPort}` : null;
  const canSend = !busySend && selectedPaths.length > 0
    && peerCode.replace(/\D/g, "").length === 6
    && (!!selectedPeer || manualIp.trim().length > 0);

  return (
    <FluentProvider theme={theme} style={{ backgroundColor: "transparent" }}>
      <div className={styles.app}>
        <TitleBar />
        <div className={styles.topbar}>
          <div className={styles.topbarLeft}>
            <div className={styles.brand}>{t("setup.brand")}</div>
            <div className={styles.device}>
              {session.settings.deviceName}
              <Tooltip content={t("topbar.renameDevice")} relationship="label">
                <Button size="small" appearance="subtle" icon={<Edit20Regular />}
                  onClick={handleRenameDevice}
                  aria-label={t("topbar.renameDevice")}
                  style={{ marginLeft: 8 }} />
              </Tooltip>
            </div>
            <div className={styles.statusRow}>
              <Wifi120Regular />
              <Caption1>{session.localIp ?? "—"}</Caption1>
              {session.receiverRunning ? (
                <Badge appearance="tint" color="success">{t("topbar.listening")} · :{session.receiverPort}</Badge>
              ) : (
                <>
                  <Badge appearance="tint" color="danger">{t("topbar.notListening")}</Badge>
                  <Button size="small" appearance="primary" onClick={startReceiverNow}>{t("topbar.startListening")}</Button>
                </>
              )}
            </div>
          </div>
          <div className={styles.topbarRight}>
            <LanguagePicker />
            <Tooltip content={t("topbar.openSettings")} relationship="label">
              <Button appearance="subtle" icon={<Settings20Regular />}
                onClick={() => setShowSettings(true)}
                aria-label={t("topbar.openSettings")} />
            </Tooltip>
            <div className={styles.codeBlock}>
              <div className={styles.codeLabel}>{t("topbar.pairingCode")}</div>
              <div className={styles.codeValueRow}>
                <span className={styles.codeValue}>{session.authCode}</span>
                <Tooltip content={t("topbar.newCode")} relationship="label">
                  <Button size="small" appearance="subtle" icon={<ArrowSync20Regular />}
                    onClick={handleRegenerate} aria-label={t("topbar.newCode")} />
                </Tooltip>
              </div>
            </div>
          </div>
        </div>

        <div className={styles.main}>
          {update && (
            <MessageBar intent="info">
              <MessageBarBody>
                <MessageBarTitle>
                  {t("update.available", { version: update.version })}
                </MessageBarTitle>
              </MessageBarBody>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <Button
                  appearance="primary"
                  size="small"
                  onClick={() => openExternalUrl(update.htmlUrl).catch(console.error)}
                >
                  {t("update.download")}
                </Button>
                <Button
                  appearance="subtle"
                  size="small"
                  onClick={() => openExternalUrl(update.htmlUrl).catch(console.error)}
                >
                  {t("update.viewNotes")}
                </Button>
                <Button
                  appearance="transparent"
                  size="small"
                  icon={<Dismiss20Regular />}
                  aria-label={t("update.dismiss")}
                  onClick={() => {
                    localStorage.setItem("update.dismissed", update.version);
                    setUpdate(null);
                  }}
                />
              </div>
            </MessageBar>
          )}

          <div style={{ display: "flex", alignItems: "center", gap: 8, color: tokens.colorNeutralForeground3, fontSize: 13 }}>
            <Folder20Regular />
            <span>{t("receiveFolder.label")}: {session.settings.saveDir}</span>
            <Button size="small" appearance="subtle" onClick={handleChangeSaveDir}>{t("common.change")}</Button>
          </div>

          <section aria-labelledby="step1-label">
            <div className={styles.sectionHead}>
              <span id="step1-label" className={styles.sectionLabel}>{t("step1.label")}</span>
              <span className={styles.emptyHint}>
                {selectedPaths.length > 0 ? t("step1.hintCount", { count: selectedPaths.length }) : t("step1.hintEmpty")}
              </span>
            </div>
            <DropZone paths={selectedPaths} dragOver={dragOver}
              onPickFiles={pickFiles} onPickFolder={pickFolder}
              onClear={clearPaths} onRemoveOne={removeOne} />
          </section>

          <section aria-labelledby="step2-label">
            <div className={styles.sectionHead}>
              <span id="step2-label" className={styles.sectionLabel}>{t("step2.label")}</span>
              <div style={{ display: "flex", gap: 8 }}>
                <Button
                  size="small"
                  appearance="subtle"
                  icon={<ArrowSync20Regular />}
                  onClick={() => refreshDiscovery().catch(console.error)}
                  aria-label={t("step2.refresh")}
                >
                  {t("step2.refresh")}
                </Button>
                <Button size="small" appearance="subtle" onClick={() => setShowManualIp((v) => !v)}>
                  {showManualIp ? t("step2.toggleList") : t("step2.toggleManual")}
                </Button>
              </div>
            </div>
            {showManualIp ? (
              <Field hint={t("step2.manualHint")}>
                <Input value={manualIp} onChange={(_, d) => { setManualIp(d.value); setSelectedPeer(null); }}
                  placeholder={t("step2.manualPlaceholder")} />
              </Field>
            ) : peerList.length === 0 ? (
              <div className={styles.emptyHint}>{t("step2.empty")}</div>
            ) : (
              <div className={styles.peerGrid}>
                {peerList.map((p) => (
                  <PeerTile key={p.instance} peer={p}
                    active={selectedPeer?.instance === p.instance}
                    onSelect={() => { setSelectedPeer(p); setManualIp(""); }} />
                ))}
              </div>
            )}
          </section>

          <section aria-labelledby="step3-label">
            <div className={styles.sectionHead}>
              <span id="step3-label" className={styles.sectionLabel}>{t("step3.label")}</span>
              {target && <span className={styles.emptyHint}>{t("step3.targetPrefix")}: {target}</span>}
            </div>
            <div className={styles.composeRow}>
              <div className={styles.codeInputWrap}>
                <Input value={peerCode}
                  onChange={(_, d) => setPeerCode(d.value.replace(/\D/g, "").slice(0, 6))}
                  placeholder="000000" size="large"
                  aria-label={t("step3.label")}
                  style={{ letterSpacing: "0.18em", fontVariantNumeric: "tabular-nums" }} />
              </div>
              <Button appearance="primary" size="large" icon={<Send20Filled />}
                onClick={handleSend} disabled={!canSend} className={styles.sendButton}>
                {busySend ? t("step3.sending") : t("step3.send")}
              </Button>
            </div>
            {sendError && (
              <MessageBar intent="error" style={{ marginTop: tokens.spacingVerticalS }}>
                <MessageBarBody>
                  <MessageBarTitle>{t("step3.errorTitle")}</MessageBarTitle>
                  {sendError}
                </MessageBarBody>
              </MessageBar>
            )}
          </section>

          {activeList.length > 0 && (
            <section aria-labelledby="active-label">
              <div className={styles.sectionHead}>
                <span id="active-label" className={styles.sectionLabel}>{t("active.label")}</span>
              </div>
              {activeList.map((a) => {
                const ratio = a.totalBytes > 0 ? a.totalBytesDone / a.totalBytes : 0;
                const elapsedS = Math.max(0.001, (performance.now() - a.startedAt) / 1000);
                const avgMbps = a.totalBytesDone / elapsedS / (1024 * 1024);
                const remainingFiles = Math.max(0, a.filesTotal - a.filesDone);
                const remainingBytes = Math.max(0, a.totalBytes - a.totalBytesDone);
                // Use the slower side as the ETA pacer: if disk can't keep up
                // with the network, the network MB/s is misleading.
                const pacerMbps = a.direction === "recv" && a.instantDisk > 0
                  ? Math.min(a.instantNetwork, a.instantDisk)
                  : a.instantNetwork;
                const etaSec = pacerMbps > 0.01
                  ? remainingBytes / (pacerMbps * 1024 * 1024)
                  : NaN;
                const eta = formatEta(etaSec);
                return (
                  <div key={a.id} className={styles.transferRow}>
                    <div className={styles.transferHeader}>
                      <Body1Strong>
                        {a.direction === "send" ? t("active.send") : t("active.recv")} — {a.peer}
                        {a.paused && (
                          <span style={{ marginLeft: 8, color: tokens.colorPaletteYellowForeground1, fontSize: 12 }}>
                            · {t("active.paused")}
                          </span>
                        )}
                      </Body1Strong>
                      <span style={{ display: "flex", gap: 4 }}>
                        <Tooltip
                          content={a.paused ? t("active.resume") : t("active.pause")}
                          relationship="label"
                        >
                          <Button
                            size="small"
                            appearance="subtle"
                            icon={a.paused ? <Play20Regular /> : <Pause20Regular />}
                            onClick={() => togglePauseTransfer(a)}
                            aria-label={a.paused ? t("active.resume") : t("active.pause")}
                          />
                        </Tooltip>
                        <Tooltip content={t("active.cancel")} relationship="label">
                          <Button
                            size="small"
                            appearance="subtle"
                            icon={<Dismiss20Regular />}
                            onClick={() => cancelTransfer(a.id).catch(console.error)}
                            aria-label={t("active.cancel")}
                          />
                        </Tooltip>
                      </span>
                    </div>
                    <ProgressBar value={ratio} thickness="medium" />
                    <div className={styles.metricsRow}>
                      {a.direction === "send" ? (
                        <div className={styles.metric}>
                          <span className={styles.metricLabel}>{t("active.now")}</span>
                          <span className={styles.metricValue}>
                            {a.instantNetwork.toFixed(1)} MB/s
                          </span>
                        </div>
                      ) : (
                        <>
                          <div className={styles.metric}>
                            <span className={styles.metricLabel}>{t("active.network")}</span>
                            <span className={styles.metricValue}>
                              {a.instantNetwork.toFixed(1)} MB/s
                            </span>
                          </div>
                          <div className={styles.metric}>
                            <span className={styles.metricLabel}>{t("active.disk")}</span>
                            <span className={styles.metricValue}>
                              {a.instantDisk.toFixed(1)} MB/s
                            </span>
                          </div>
                        </>
                      )}
                      <div className={styles.metric}>
                        <span className={styles.metricLabel}>{t("active.average")}</span>
                        <span className={styles.metricValueMuted}>
                          {avgMbps.toFixed(1)} MB/s
                        </span>
                      </div>
                      <div className={styles.metric}>
                        <span className={styles.metricLabel}>{t("active.eta")}</span>
                        <span className={styles.metricValueMuted}>{eta}</span>
                      </div>
                    </div>
                    <div className={styles.metricSecondary}>
                      {formatBytes(a.totalBytesDone)} / {formatBytes(a.totalBytes)}
                      {" · "}
                      {a.filesDone}/{a.filesTotal} {t("active.files")}
                      {" · "}
                      {t("active.remaining", { count: remainingFiles })}
                    </div>
                    {a.currentFile && (
                      <span className={styles.peerMeta} style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {a.currentFile}
                      </span>
                    )}
                  </div>
                );
              })}
            </section>
          )}
        </div>

        <Dialog open={incoming !== null} modalType="alert">
          <DialogSurface>
            <DialogBody>
              <DialogTitle>{t("incoming.title")}</DialogTitle>
              <DialogContent>
                {incoming && (
                  <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                    <div>
                      <Subtitle1>{incoming.request.deviceName}</Subtitle1>
                      <Caption1 style={{ color: tokens.colorNeutralForeground3, display: "block" }}>
                        {incoming.request.peer} · {incoming.request.os}
                      </Caption1>
                    </div>
                    <Body1>
                      {t("incoming.summary", {
                        count: incoming.request.fileCount,
                        size: formatBytes(incoming.request.totalBytes),
                      })}
                    </Body1>
                    <Divider />
                    <div>
                      <div className={styles.codeLabel} style={{ marginBottom: 4 }}>{t("incoming.folder")}</div>
                      <div style={{ fontSize: 13, color: tokens.colorNeutralForeground2 }}>
                        {incoming.overrideDir ?? session.settings.saveDir}
                      </div>
                      <Button size="small" appearance="subtle" icon={<Folder20Regular />}
                        onClick={pickIncomingDir} style={{ marginTop: 4 }}>
                        {t("incoming.overrideButton")}
                      </Button>
                    </div>
                    <Checkbox
                      label={t("incoming.trustCheck")}
                      checked={incoming.trust}
                      onChange={(_, d) => setIncoming({ ...incoming, trust: !!d.checked })}
                    />
                  </div>
                )}
              </DialogContent>
              <DialogActions>
                <DialogTrigger disableButtonEnhancement>
                  <Button appearance="secondary" onClick={() => respondAndClose(false)}>
                    {t("incoming.reject")}
                  </Button>
                </DialogTrigger>
                <DialogTrigger disableButtonEnhancement>
                  <Button appearance="primary" onClick={() => respondAndClose(true)}>
                    {t("incoming.accept")}
                  </Button>
                </DialogTrigger>
              </DialogActions>
            </DialogBody>
          </DialogSurface>
        </Dialog>

        <SettingsDrawer
          open={showSettings}
          onOpenChange={setShowSettings}
          session={session}
          onSettingsUpdated={(s) => setSession({ ...session, settings: s })}
          useAccent={useAccent}
          onToggleAccent={handleToggleAccent}
          accentHex={accent}
        />
      </div>
    </FluentProvider>
  );
}

export default App;
