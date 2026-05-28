import { useEffect, useMemo, useState } from "react";
import {
  FluentProvider,
  webLightTheme,
  webDarkTheme,
  Theme,
  Title2,
  Subtitle2,
  Body1,
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
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import { open } from "@tauri-apps/plugin-dialog";
import {
  getDefaultPort,
  getDeviceName,
  getLocalIp,
  onTransferCompleted,
  onTransferProgress,
  onTransferStarted,
  sendPaths,
  startReceiving,
  stopReceiving,
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
    flexDirection: "column",
    gap: tokens.spacingVerticalXS,
  },
  grid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: tokens.spacingHorizontalL,
    alignItems: "stretch",
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

function App() {
  const styles = useStyles();
  const theme = useSystemTheme();

  const [deviceName, setDeviceName] = useState<string>("...");
  const [localIp, setLocalIp] = useState<string>("...");
  const [defaultPort, setDefaultPortState] = useState<number>(0);

  // Receiver state
  const [saveDir, setSaveDir] = useState<string>("");
  const [receiving, setReceiving] = useState<boolean>(false);
  const [receiverPort, setReceiverPort] = useState<number | null>(null);
  const [receiverError, setReceiverError] = useState<string | null>(null);
  const [busyReceiver, setBusyReceiver] = useState<boolean>(false);

  // Sender state
  const [peerIp, setPeerIp] = useState<string>("");
  const [peerPort, setPeerPort] = useState<string>("");
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [sendError, setSendError] = useState<string | null>(null);
  const [busySend, setBusySend] = useState<boolean>(false);

  const [active, setActive] = useState<Record<string, ActiveTransfer>>({});
  const [completed, setCompleted] = useState<CompletedRow[]>([]);

  useEffect(() => {
    (async () => {
      setDeviceName(await getDeviceName());
      setLocalIp((await getLocalIp()) ?? "—");
      const p = await getDefaultPort();
      setDefaultPortState(p);
      setPeerPort(String(p));
    })();
  }, []);

  useEffect(() => {
    const unsubs: Promise<Unlisten>[] = [];
    unsubs.push(
      onTransferStarted((e: TransferStarted) => {
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
      }),
    );
    unsubs.push(
      onTransferProgress((e: TransferProgress) => {
        setActive((prev) => {
          const a = prev[e.id];
          if (!a) return prev;
          return {
            ...prev,
            [e.id]: {
              ...a,
              filesDone: e.filesDone,
              filesTotal: e.filesTotal,
              totalBytes: e.totalBytes,
              totalBytesDone: e.totalBytesDone,
              currentFile: e.currentFile,
            },
          };
        });
      }),
    );
    unsubs.push(
      onTransferCompleted((e: TransferCompleted) => {
        setActive((prev) => {
          const a = prev[e.id];
          const elapsedMs = e.elapsedMs || (a ? performance.now() - a.startedAt : 1);
          const mbps =
            elapsedMs > 0 ? (e.totalBytes / (elapsedMs / 1000)) / (1024 * 1024) : 0;
          setCompleted((cs) => [
            {
              id: e.id,
              direction: e.direction,
              ok: e.success,
              msg: e.success ? "Tamam" : e.error ?? "Hata",
              mbps,
              bytes: e.totalBytes,
            },
            ...cs.slice(0, 19),
          ]);
          const { [e.id]: _gone, ...rest } = prev;
          return rest;
        });
      }),
    );
    return () => {
      unsubs.forEach((u) => u.then((fn) => fn()).catch(() => {}));
    };
  }, []);

  const pickSaveDir = async () => {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") setSaveDir(dir);
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
  const clearSelection = () => setSelectedPaths([]);
  const handleStart = async () => {
    setReceiverError(null);
    if (!saveDir) {
      setReceiverError("Önce kayıt klasörü seç.");
      return;
    }
    setBusyReceiver(true);
    try {
      const port = await startReceiving(saveDir, defaultPort || undefined);
      setReceiverPort(port);
      setReceiving(true);
    } catch (e) {
      setReceiverError(String(e));
    } finally {
      setBusyReceiver(false);
    }
  };
  const handleStop = async () => {
    setBusyReceiver(true);
    try {
      await stopReceiving();
      setReceiving(false);
      setReceiverPort(null);
    } finally {
      setBusyReceiver(false);
    }
  };
  const handleSend = async () => {
    setSendError(null);
    if (!peerIp.trim()) return setSendError("IP gir.");
    if (selectedPaths.length === 0) return setSendError("Dosya veya klasör seç.");
    const portNum = peerPort ? Number(peerPort) : undefined;
    setBusySend(true);
    try {
      await sendPaths(peerIp.trim(), selectedPaths, portNum);
    } catch (e) {
      setSendError(String(e));
    } finally {
      setBusySend(false);
    }
  };

  const activeList = useMemo(() => Object.values(active), [active]);

  return (
    <FluentProvider theme={theme} style={{ backgroundColor: "transparent" }}>
      <div className={styles.root}>
        <div className={styles.header}>
          <Title2>LanBlaze</Title2>
          <Body1 className={styles.meta}>
            Cihaz: <b>{deviceName}</b> · Yerel IP: <b>{localIp}</b> · Varsayılan
            port: <b>{defaultPort || "..."}</b>
          </Body1>
        </div>

        <div className={styles.grid}>
          <Card className={styles.card}>
            <Subtitle2>Alıcı</Subtitle2>
            <Field label="Kayıt klasörü">
              <div className={styles.row}>
                <Input
                  value={saveDir}
                  onChange={(_, d) => setSaveDir(d.value)}
                  placeholder="Klasör seçin"
                  style={{ flex: 1 }}
                  readOnly
                />
                <Button onClick={pickSaveDir}>Seç</Button>
              </div>
            </Field>
            <div className={styles.row}>
              {receiving ? (
                <Button appearance="secondary" onClick={handleStop} disabled={busyReceiver}>
                  Durdur
                </Button>
              ) : (
                <Button appearance="primary" onClick={handleStart} disabled={busyReceiver}>
                  Dinlemeyi başlat
                </Button>
              )}
              {busyReceiver && <Spinner size="tiny" />}
              {receiving && receiverPort !== null && (
                <Body1 className={styles.meta}>
                  Dinleniyor: <b>{localIp}:{receiverPort}</b>
                </Body1>
              )}
            </div>
            {receiverError && (
              <MessageBar intent="error">
                <MessageBarBody>
                  <MessageBarTitle>Alıcı hatası</MessageBarTitle>
                  {receiverError}
                </MessageBarBody>
              </MessageBar>
            )}
          </Card>

          <Card className={styles.card}>
            <Subtitle2>Gönderici</Subtitle2>
            <Field label="Karşı taraf IP">
              <Input
                value={peerIp}
                onChange={(_, d) => setPeerIp(d.value)}
                placeholder="örn. 192.168.1.42"
              />
            </Field>
            <Field label="Port">
              <Input
                value={peerPort}
                onChange={(_, d) => setPeerPort(d.value)}
                placeholder="47813"
              />
            </Field>
            <Field label="Gönderilecekler">
              <div className={styles.row}>
                <Input
                  value={
                    selectedPaths.length === 0
                      ? ""
                      : selectedPaths.length === 1
                        ? selectedPaths[0]
                        : `${selectedPaths.length} öğe seçildi`
                  }
                  placeholder="Dosya veya klasör seçin"
                  style={{ flex: 1 }}
                  readOnly
                />
                <Button onClick={pickFiles}>Dosya(lar)</Button>
                <Button onClick={pickFolder}>Klasör</Button>
                {selectedPaths.length > 0 && (
                  <Button appearance="subtle" onClick={clearSelection}>
                    Temizle
                  </Button>
                )}
              </div>
            </Field>
            <div className={styles.row}>
              <Button appearance="primary" onClick={handleSend} disabled={busySend}>
                Gönder
              </Button>
              {busySend && <Spinner size="tiny" />}
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
        </div>

        <Card className={styles.card}>
          <Subtitle2>Aktif transferler</Subtitle2>
          {activeList.length === 0 ? (
            <Body1 className={styles.meta}>Şu an aktif transfer yok.</Body1>
          ) : (
            activeList.map((a) => {
              const ratio =
                a.totalBytes > 0 ? a.totalBytesDone / a.totalBytes : 0;
              const elapsedS = Math.max(
                0.001,
                (performance.now() - a.startedAt) / 1000,
              );
              const mbps = a.totalBytesDone / elapsedS / (1024 * 1024);
              return (
                <div key={a.id} style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <Body1>
                    {a.direction === "send" ? "Gönderim" : "Alım"} — {a.peer}
                    {" · "}
                    <span className={styles.meta}>{a.currentFile || "—"}</span>
                  </Body1>
                  <ProgressBar value={ratio} />
                  <Body1 className={styles.meta}>
                    {formatBytes(a.totalBytesDone)} / {formatBytes(a.totalBytes)}
                    {" · "}
                    {mbps.toFixed(1)} MB/s · {a.filesDone}/{a.filesTotal} dosya
                  </Body1>
                </div>
              );
            })
          )}
          {completed.length > 0 && (
            <>
              <Divider />
              <Subtitle2>Geçmiş</Subtitle2>
              {completed.map((c) => (
                <Body1 key={c.id} className={styles.meta}>
                  {c.direction === "send" ? "Gönderildi" : "Alındı"} · {formatBytes(c.bytes)}
                  {" · "}
                  {c.mbps.toFixed(1)} MB/s · {c.ok ? "Başarılı" : `Hata: ${c.msg}`}
                </Body1>
              ))}
            </>
          )}
        </Card>
      </div>
    </FluentProvider>
  );
}

export default App;
