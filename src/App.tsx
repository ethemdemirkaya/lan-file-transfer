import { useEffect, useMemo, useRef, useState } from "react";
import {
  FluentProvider,
  webLightTheme,
  webDarkTheme,
  Theme,
  Title2,
  Title3,
  Subtitle2,
  Subtitle1,
  Body1,
  Caption1,
  Button,
  Card,
  Input,
  Field,
  ProgressBar,
  Divider,
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
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import {
  ArrowSync24Regular,
  Folder24Regular,
  Document24Regular,
  Send24Regular,
  Delete24Regular,
} from "@fluentui/react-icons";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  ensureReceiver,
  getSession,
  IncomingRequest,
  onIncomingRequest,
  onPeerAdded,
  onPeerRemoved,
  onTransferCompleted,
  onTransferProgress,
  onTransferStarted,
  Peer,
  regenerateCode,
  respondIncoming,
  saveSettings,
  sendPaths,
  Session,
  TransferCompleted,
  TransferProgress,
  TransferStarted,
  Unlisten,
} from "./lib/tauri";

const useStyles = makeStyles({
  root: {
    minHeight: "100vh",
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalL,
    padding: tokens.spacingHorizontalXXL,
    backgroundColor: "transparent",
  },
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "flex-start",
    gap: tokens.spacingHorizontalL,
    flexWrap: "wrap",
  },
  identity: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalXS,
  },
  codeCard: {
    padding: tokens.spacingHorizontalL,
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: tokens.spacingVerticalXS,
    minWidth: "220px",
  },
  codeValue: {
    fontSize: "44px",
    fontWeight: 600,
    letterSpacing: "0.18em",
    fontVariantNumeric: "tabular-nums",
    color: tokens.colorBrandForeground1,
  },
  card: {
    padding: tokens.spacingHorizontalXXL,
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalM,
  },
  row: {
    display: "flex",
    gap: tokens.spacingHorizontalM,
    alignItems: "center",
  },
  meta: {
    color: tokens.colorNeutralForeground3,
    fontVariantNumeric: "tabular-nums",
  },
  peerGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))",
    gap: tokens.spacingHorizontalM,
  },
  peerCard: {
    padding: tokens.spacingHorizontalM,
    cursor: "pointer",
    transition: "all 120ms ease",
  },
  peerCardActive: {
    boxShadow: tokens.shadow8,
    outlineWidth: "2px",
    outlineStyle: "solid",
    outlineColor: tokens.colorBrandForeground1,
  },
  dropZone: {
    border: `2px dashed ${tokens.colorNeutralStroke2}`,
    borderRadius: tokens.borderRadiusLarge,
    padding: tokens.spacingVerticalXXL,
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: tokens.spacingVerticalS,
    color: tokens.colorNeutralForeground3,
    transition: "all 120ms ease",
  },
  dropZoneActive: {
    borderTopColor: tokens.colorBrandForeground1,
    borderRightColor: tokens.colorBrandForeground1,
    borderBottomColor: tokens.colorBrandForeground1,
    borderLeftColor: tokens.colorBrandForeground1,
    backgroundColor: tokens.colorSubtleBackgroundHover,
    color: tokens.colorBrandForeground1,
  },
});

function useSystemTheme(): Theme {
  const get = () =>
    window.matchMedia("(prefers-color-scheme: dark)").matches
      ? webDarkTheme
      : webLightTheme;
  const [theme, setTheme] = useState<Theme>(get);
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => setTheme(get());
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);
  return theme;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

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
}

interface CompletedRow {
  id: string;
  direction: "send" | "recv";
  ok: boolean;
  msg: string;
  mbps: number;
  bytes: number;
}

interface IncomingPending {
  request: IncomingRequest;
  overrideDir: string | null;
}

function SetupWizard({
  initial,
  onDone,
}: {
  initial: Session;
  onDone: (s: Session) => void;
}) {
  const styles = useStyles();
  const theme = useSystemTheme();
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
      <div className={styles.root} style={{ alignItems: "center", justifyContent: "center" }}>
        <Card className={styles.card} style={{ maxWidth: 480, width: "100%" }}>
          <Title2>LanBlaze'e hoş geldin</Title2>
          <Body1 className={styles.meta}>
            Aynı yerel ağdaki cihazlarla bulutsuz, hızlı dosya aktarımı. Önce
            birkaç şey ayarlayalım.
          </Body1>
          <Field label="Cihaz adı">
            <Input value={name} onChange={(_, d) => setName(d.value)} />
          </Field>
          <Field label="Varsayılan kayıt klasörü" hint="Gelen dosyalar buraya düşer; her aktarımda değiştirebilirsin.">
            <div className={styles.row}>
              <Input value={dir} onChange={(_, d) => setDir(d.value)} style={{ flex: 1 }} readOnly />
              <Button onClick={pickDir}>Seç</Button>
            </div>
          </Field>
          {error && (
            <MessageBar intent="error">
              <MessageBarBody>{error}</MessageBarBody>
            </MessageBar>
          )}
          <div className={styles.row} style={{ justifyContent: "flex-end" }}>
            <Button appearance="primary" onClick={handleSave} disabled={busy}>
              {busy ? "Kaydediliyor..." : "Devam et"}
            </Button>
          </div>
        </Card>
      </div>
    </FluentProvider>
  );
}

function App() {
  const styles = useStyles();
  const theme = useSystemTheme();

  const [session, setSession] = useState<Session | null>(null);
  const [peers, setPeers] = useState<Record<string, Peer>>({});
  const [active, setActive] = useState<Record<string, ActiveTransfer>>({});
  const [completed, setCompleted] = useState<CompletedRow[]>([]);
  const [incoming, setIncoming] = useState<IncomingPending | null>(null);

  const [selectedPeer, setSelectedPeer] = useState<Peer | null>(null);
  const [manualIp, setManualIp] = useState("");
  const [peerCode, setPeerCode] = useState("");
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [dragOver, setDragOver] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);
  const [busySend, setBusySend] = useState(false);
  const incomingRef = useRef<IncomingPending | null>(null);
  incomingRef.current = incoming;

  // Initial session load.
  useEffect(() => {
    (async () => {
      try {
        const s = await getSession();
        setSession(s);
        if (s.settings.configured && !s.receiverRunning) {
          try { await ensureReceiver(); } catch {}
        }
      } catch (e) {
        console.error(e);
      }
    })();
  }, []);

  // Event subscriptions.
  useEffect(() => {
    const unsubs: Promise<Unlisten>[] = [];
    unsubs.push(onTransferStarted((e: TransferStarted) => {
      setActive((prev) => ({
        ...prev,
        [e.id]: {
          id: e.id,
          direction: e.direction,
          peer: e.peer,
          filesTotal: e.fileCount,
          filesDone: 0,
          totalBytes: e.totalBytes,
          totalBytesDone: 0,
          currentFile: "",
          startedAt: performance.now(),
        },
      }));
    }));
    unsubs.push(onTransferProgress((e: TransferProgress) => {
      setActive((prev) => {
        const a = prev[e.id]; if (!a) return prev;
        return { ...prev, [e.id]: { ...a, filesDone: e.filesDone, filesTotal: e.filesTotal, totalBytes: e.totalBytes, totalBytesDone: e.totalBytesDone, currentFile: e.currentFile } };
      });
    }));
    unsubs.push(onTransferCompleted((e: TransferCompleted) => {
      setActive((prev) => {
        const a = prev[e.id];
        const elapsedMs = e.elapsedMs || (a ? performance.now() - a.startedAt : 1);
        const mbps = elapsedMs > 0 ? (e.totalBytes / (elapsedMs / 1000)) / (1024 * 1024) : 0;
        setCompleted((cs) => [
          { id: e.id, direction: e.direction, ok: e.success, msg: e.success ? "Tamam" : e.error ?? "Hata", mbps, bytes: e.totalBytes },
          ...cs.slice(0, 19),
        ]);
        const { [e.id]: _gone, ...rest } = prev;
        return rest;
      });
    }));
    unsubs.push(onPeerAdded((p: Peer) => setPeers((prev) => ({ ...prev, [p.instance]: p }))));
    unsubs.push(onPeerRemoved((instance: string) => setPeers((prev) => {
      const { [instance]: _g, ...rest } = prev; return rest;
    })));
    unsubs.push(onIncomingRequest((r: IncomingRequest) => {
      setIncoming({ request: r, overrideDir: null });
    }));
    return () => { unsubs.forEach((u) => u.then((fn) => fn()).catch(() => {})); };
  }, []);

  // Tauri drag-drop on window.
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    const unlistenPromise = win.onDragDropEvent((event) => {
      const p: any = event.payload;
      if (p.type === "enter" || p.type === "over") setDragOver(true);
      else if (p.type === "leave") setDragOver(false);
      else if (p.type === "drop") {
        setDragOver(false);
        const paths: string[] = p.paths ?? [];
        if (paths.length > 0) setSelectedPaths(paths);
      }
    });
    return () => { unlistenPromise.then((u) => u()).catch(() => {}); };
  }, []);

  const peerList = useMemo(() => Object.values(peers), [peers]);
  const activeList = useMemo(() => Object.values(active), [active]);

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
      } catch (e) {
        console.error(e);
      }
    }
  };
  const pickFiles = async () => {
    const f = await open({ directory: false, multiple: true });
    if (Array.isArray(f)) setSelectedPaths(f);
    else if (typeof f === "string") setSelectedPaths([f]);
  };
  const pickFolder = async () => {
    const f = await open({ directory: true, multiple: false });
    if (typeof f === "string") setSelectedPaths([f]);
  };
  const handleSend = async () => {
    setSendError(null);
    const ip = selectedPeer?.addresses[0] ?? manualIp.trim();
    const port = selectedPeer?.port ?? session?.defaultPort;
    if (!ip) return setSendError("Bir cihaz seç veya IP yaz.");
    if (selectedPaths.length === 0) return setSendError("Dosya veya klasör seç.");
    if (peerCode.replace(/\D/g, "").length !== 6) return setSendError("6 haneli eşleştirme kodunu gir.");
    setBusySend(true);
    try {
      await sendPaths(ip, selectedPaths, peerCode.replace(/\D/g, ""), port);
      setSelectedPaths([]);
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
      await respondIncoming(pending.request.id, accept, pending.overrideDir ?? undefined);
    } catch (e) {
      console.error(e);
    } finally {
      setIncoming(null);
    }
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
        <div className={styles.root} style={{ alignItems: "center", justifyContent: "center" }}>
          <Spinner label="Yükleniyor..." />
        </div>
      </FluentProvider>
    );
  }
  if (!session.settings.configured) {
    return <SetupWizard initial={session} onDone={setSession} />;
  }

  const effectiveSaveDir = incoming?.overrideDir ?? session.settings.saveDir;

  return (
    <FluentProvider theme={theme} style={{ backgroundColor: "transparent" }}>
      <div className={styles.root}>
        <div className={styles.header}>
          <div className={styles.identity}>
            <Title2>LanBlaze</Title2>
            <Body1>
              <b>{session.settings.deviceName}</b> · {session.localIp ?? "—"}
              {session.receiverPort != null ? `:${session.receiverPort}` : ""}
            </Body1>
            <Caption1 className={styles.meta}>
              Kayıt klasörü: {session.settings.saveDir}
              {"  "}
              <Button size="small" appearance="subtle" onClick={handleChangeSaveDir}>
                Değiştir
              </Button>
            </Caption1>
          </div>
          <Card className={styles.codeCard}>
            <Caption1 className={styles.meta}>Bu cihazın eşleştirme kodu</Caption1>
            <span className={styles.codeValue}>{session.authCode}</span>
            <Button
              size="small"
              appearance="subtle"
              icon={<ArrowSync24Regular />}
              onClick={handleRegenerate}
            >
              Yeni kod
            </Button>
          </Card>
        </div>

        <Card className={styles.card}>
          <Subtitle2>Ağdaki cihazlar</Subtitle2>
          {peerList.length === 0 ? (
            <Body1 className={styles.meta}>
              Henüz cihaz görünmüyor. Karşı tarafta uygulama açık ve dinliyor olmalı.
              Yoksa IP'yi aşağıya elle yaz.
            </Body1>
          ) : (
            <div className={styles.peerGrid}>
              {peerList.map((p) => {
                const isSel = selectedPeer?.instance === p.instance;
                return (
                  <Card
                    key={p.instance}
                    className={`${styles.peerCard} ${isSel ? styles.peerCardActive : ""}`}
                    onClick={() => setSelectedPeer(p)}
                  >
                    <Subtitle1>{p.deviceName}</Subtitle1>
                    <Caption1 className={styles.meta}>
                      {p.addresses[0] ?? "?"}:{p.port}
                    </Caption1>
                    <Caption1 className={styles.meta}>{p.os}</Caption1>
                  </Card>
                );
              })}
            </div>
          )}
          <Field label="Veya manuel IP" hint="mDNS engelliyse">
            <Input
              value={manualIp}
              onChange={(_, d) => { setManualIp(d.value); setSelectedPeer(null); }}
              placeholder="192.168.1.42"
            />
          </Field>
        </Card>

        <Card className={styles.card}>
          <Subtitle2>Gönder</Subtitle2>
          <div
            className={`${styles.dropZone} ${dragOver ? styles.dropZoneActive : ""}`}
          >
            {selectedPaths.length === 0 ? (
              <>
                <Body1>Dosya veya klasörü buraya bırak</Body1>
                <Caption1>ya da aşağıdaki düğmelerle seç</Caption1>
              </>
            ) : (
              <>
                <Body1>
                  {selectedPaths.length === 1
                    ? selectedPaths[0]
                    : `${selectedPaths.length} öğe seçildi`}
                </Body1>
              </>
            )}
            <div className={styles.row}>
              <Button icon={<Document24Regular />} onClick={pickFiles}>
                Dosya(lar)
              </Button>
              <Button icon={<Folder24Regular />} onClick={pickFolder}>
                Klasör
              </Button>
              {selectedPaths.length > 0 && (
                <Button
                  appearance="subtle"
                  icon={<Delete24Regular />}
                  onClick={() => setSelectedPaths([])}
                >
                  Temizle
                </Button>
              )}
            </div>
          </div>
          <Field label="Karşı tarafın eşleştirme kodu (6 hane)">
            <Input
              value={peerCode}
              onChange={(_, d) => setPeerCode(d.value.replace(/\D/g, "").slice(0, 6))}
              placeholder="000000"
            />
          </Field>
          <div className={styles.row}>
            <Button
              appearance="primary"
              icon={<Send24Regular />}
              onClick={handleSend}
              disabled={busySend}
            >
              Gönder
            </Button>
            {busySend && <Spinner size="tiny" />}
            <Body1 className={styles.meta}>
              Hedef: {selectedPeer
                ? `${selectedPeer.deviceName} (${selectedPeer.addresses[0]}:${selectedPeer.port})`
                : manualIp
                  ? `${manualIp}:${session.defaultPort}`
                  : "—"}
            </Body1>
          </div>
          {sendError && (
            <MessageBar intent="error">
              <MessageBarBody>
                <MessageBarTitle>Gönderim hatası</MessageBarTitle>
                {sendError}
              </MessageBarBody>
            </MessageBar>
          )}
        </Card>

        <Card className={styles.card}>
          <Subtitle2>Aktif transferler</Subtitle2>
          {activeList.length === 0 ? (
            <Body1 className={styles.meta}>Şu an aktif transfer yok.</Body1>
          ) : (
            activeList.map((a) => {
              const ratio = a.totalBytes > 0 ? a.totalBytesDone / a.totalBytes : 0;
              const elapsedS = Math.max(0.001, (performance.now() - a.startedAt) / 1000);
              const mbps = a.totalBytesDone / elapsedS / (1024 * 1024);
              return (
                <div key={a.id} style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <Body1>
                    {a.direction === "send" ? "Gönderim" : "Alım"} — {a.peer}
                    {" · "}
                    <span className={styles.meta}>{a.currentFile || "—"}</span>
                  </Body1>
                  <ProgressBar value={ratio} />
                  <Caption1 className={styles.meta}>
                    {formatBytes(a.totalBytesDone)} / {formatBytes(a.totalBytes)}
                    {" · "}
                    {mbps.toFixed(1)} MB/s · {a.filesDone}/{a.filesTotal} dosya
                  </Caption1>
                </div>
              );
            })
          )}
          {completed.length > 0 && (
            <>
              <Divider />
              <Subtitle2>Geçmiş</Subtitle2>
              {completed.map((c) => (
                <Caption1 key={c.id} className={styles.meta}>
                  {c.direction === "send" ? "Gönderildi" : "Alındı"} · {formatBytes(c.bytes)}
                  {" · "}{c.mbps.toFixed(1)} MB/s · {c.ok ? "Başarılı" : `Hata: ${c.msg}`}
                </Caption1>
              ))}
            </>
          )}
        </Card>

        <Dialog open={incoming !== null} modalType="alert">
          <DialogSurface>
            <DialogBody>
              <DialogTitle>Gelen dosya isteği</DialogTitle>
              <DialogContent>
                {incoming && (
                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    <Body1>
                      <b>{incoming.request.deviceName}</b> ({incoming.request.peer})
                      sana <b>{incoming.request.fileCount}</b> dosya
                      ({formatBytes(incoming.request.totalBytes)}) göndermek istiyor.
                    </Body1>
                    <Caption1 className={styles.meta}>
                      Kayıt klasörü: {effectiveSaveDir}
                    </Caption1>
                    <Button appearance="subtle" onClick={pickIncomingDir}>
                      Başka klasöre kaydet
                    </Button>
                  </div>
                )}
              </DialogContent>
              <DialogActions>
                <DialogTrigger disableButtonEnhancement>
                  <Button appearance="secondary" onClick={() => respondAndClose(false)}>
                    Reddet
                  </Button>
                </DialogTrigger>
                <DialogTrigger disableButtonEnhancement>
                  <Button appearance="primary" onClick={() => respondAndClose(true)}>
                    Kabul et
                  </Button>
                </DialogTrigger>
              </DialogActions>
            </DialogBody>
          </DialogSurface>
        </Dialog>
      </div>
    </FluentProvider>
  );
}

// Silence unused import warnings for icons we may add later.
const _silence_unused = { Title3 };
export default App;
export { _silence_unused };
