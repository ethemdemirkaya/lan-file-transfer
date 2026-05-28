import { useEffect, useState } from "react";
import {
  FluentProvider,
  webLightTheme,
  webDarkTheme,
  Theme,
  Title2,
  Body1,
  Button,
  Card,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import { invoke } from "@tauri-apps/api/core";

const useStyles = makeStyles({
  root: {
    minHeight: "100vh",
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    gap: tokens.spacingVerticalL,
    padding: tokens.spacingHorizontalXXL,
    backgroundColor: "transparent",
  },
  card: {
    padding: tokens.spacingHorizontalXXL,
    minWidth: "360px",
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalM,
    alignItems: "stretch",
  },
  row: {
    display: "flex",
    gap: tokens.spacingHorizontalM,
    alignItems: "center",
    justifyContent: "space-between",
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

function App() {
  const styles = useStyles();
  const theme = useSystemTheme();
  const [pong, setPong] = useState<string>("");

  const ping = async () => {
    try {
      const reply = await invoke<string>("ping", { name: "LanBlaze" });
      setPong(reply);
    } catch (e) {
      setPong(`Hata: ${String(e)}`);
    }
  };

  return (
    <FluentProvider theme={theme} style={{ backgroundColor: "transparent" }}>
      <div className={styles.root}>
        <Card className={styles.card}>
          <Title2>LanBlaze</Title2>
          <Body1>
            Yerel ağ üzerinden yüksek hızlı dosya aktarımı. Faz 0 iskeleti
            çalışıyor.
          </Body1>
          <div className={styles.row}>
            <Button appearance="primary" onClick={ping}>
              Ping gönder
            </Button>
            <Body1>{pong || "—"}</Body1>
          </div>
        </Card>
      </div>
    </FluentProvider>
  );
}

export default App;
